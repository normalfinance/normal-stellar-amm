use crate::storage::{get_base_tax_fraction, get_fee_fraction};
use crate::{constants::FEE_MULTIPLIER, storage::get_fee_rebate_fraction};
use pool_validation_errors::PoolValidationError;
use soroban_fixed_point_math::SorobanFixedPoint;
use soroban_sdk::{panic_with_error, Env};
use utils::constant::PRICE_PRECISION_I64;
use utils::{
    constant::PRICE_PRECISION,
    math::safe_math::{PrecisionMath, SafeConversion, SafeMath},
};

pub fn get_deposit_amounts(
    e: &Env,
    desired_a: u128,
    min_a: u128,
    desired_b: u128,
    min_b: u128,
    reserve_a: u128,
    reserve_b: u128,
) -> (u128, u128) {
    if reserve_a == 0 && reserve_b == 0 {
        return (desired_a, desired_b);
    }

    let amount_b = desired_a.fixed_mul_floor(e, &reserve_b, &reserve_a);
    if amount_b <= desired_b {
        if amount_b < min_b {
            panic_with_error!(e, PoolValidationError::InvalidDepositAmount);
        }
        (desired_a, amount_b)
    } else {
        let amount_a = desired_b.fixed_mul_floor(&e, &reserve_a, &reserve_b);
        if amount_a > desired_a || amount_a < min_a {
            panic_with_error!(e, PoolValidationError::InvalidDepositAmount);
        }
        (amount_a, desired_b)
    }
}

pub fn get_amount_out(
    e: &Env,
    in_amount: u128,    // dx  – exact tokens the trader wants to sell
    reserve_sell: u128, // x
    reserve_buy: u128,  // y
    risk_reducing: bool,
) -> (u128, u128) {
    if in_amount == 0 {
        return (0, 0);
    }

    // Fee
    let fee_fraction = get_fee_fraction(e) as u128; // e.g. 30 => 0.3 %
    let fee_rebate_fraction = get_fee_rebate_fraction(e) as u128; // e.g. 5000 => 50%
    let risk_adjusted_fee_fraction = if risk_reducing {
        fee_fraction.fixed_mul_floor(e, &fee_rebate_fraction, &FEE_MULTIPLIER)
    } else {
        fee_fraction
    };
    let in_after_fee = (in_amount * (FEE_MULTIPLIER - risk_adjusted_fee_fraction)) / FEE_MULTIPLIER;

    let raw_out = in_after_fee.fixed_mul_floor(e, &reserve_buy, &(reserve_sell + in_after_fee));

    (raw_out, in_amount - in_after_fee) // fee is taken on input
}

pub fn get_amount_out_strict_receive(
    e: &Env,
    out_amount: u128,   // dy  – exact tokens the trader wants to receive
    reserve_sell: u128, // x
    reserve_buy: u128,  // y
    risk_reducing: bool,
) -> (u128, u128) {
    if out_amount == 0 {
        return (0, 0);
    }
    if out_amount >= reserve_buy {
        panic_with_error!(e, PoolValidationError::InsufficientBalance);
    }

    let fee_fraction = get_fee_fraction(&e) as u128;
    let fee_rebate_fraction = get_fee_rebate_fraction(e) as u128;

    let risk_adjusted_fee_fraction = if risk_reducing {
        fee_fraction.fixed_mul_floor(e, &fee_rebate_fraction, &FEE_MULTIPLIER)
    } else {
        fee_fraction
    };

    // ----------  Step 1: dx_after_fee = ceil(x·dy / (y-dy))  ----------
    let dx_after_fee = reserve_sell.fixed_mul_ceil(e, &out_amount, &(reserve_buy - out_amount));

    // ----------  Step 2: gross-up for fee on *input* side  -------------
    // dx_before_fee = ceil( dx_after_fee / (1-f) )
    let dx_before_fee = dx_after_fee.fixed_mul_ceil(
        e,
        &FEE_MULTIPLIER,
        &(FEE_MULTIPLIER - risk_adjusted_fee_fraction),
    );

    // ----------  Step 3: fee = dx_before_fee - dx_after_fee -----------
    let fee = dx_before_fee - dx_after_fee;

    (dx_before_fee, fee)
}

pub fn pool_price(e: &Env, reserve_a: u128, reserve_b: u128) -> u128 {
    if reserve_a == 0 || reserve_b == 0 {
        return 0;
    }

    reserve_b.safe_fixed_div_round(e, reserve_a, PRICE_PRECISION)
}

// Calculates the peg price between the base and quote assets based on oracle prices.
//
// Returns `quote / base` to represent the current price ratio. If either price is zero,
// returns 0 to indicate an invalid state.
//
// # Arguments
// * `e` - Soroban environment reference.
// * `base_oracle_price` - Oracle price of the base asset.
// * `quote_oracle_price` - Oracle price of the quote asset.
//
// # Returns
// * `u128` — The derived peg price (scaled by `PRICE_PRECISION`), or 0 if invalid.
pub fn peg_price(e: &Env, base_oracle_price: u128, quote_oracle_price: u128) -> u128 {
    if base_oracle_price == 0 || quote_oracle_price == 0 {
        return 0;
    }

    // Calculate quote_oracle_price / base_oracle_price with round-to-nearest to reduce bias
    quote_oracle_price.safe_fixed_div_round(e, base_oracle_price, PRICE_PRECISION)
}

pub fn calculate_price_deviation(e: &Env, price_a: u128, price_b: u128) -> i128 {
    // Use safe conversions to prevent overflow
    let price_a_i128 = price_a.safe_to_i128(e);
    let price_b_i128 = price_b.safe_to_i128(e);

    price_a_i128.safe_sub(e, price_b_i128)
}

pub fn calculate_price_spread_pct(e: &Env, price_a: u128, price_b: u128) -> i64 {
    let price_deviation = calculate_price_deviation(e, price_a, price_b);

    // Safe conversion to i64 with overflow protection
    let price_spread = price_deviation.safe_to_i64(e);
    let price_a_i64 = price_a.safe_to_i64(e);

    // Calculate (price_spread * PRICE_PRECISION_I64) / price_a_i64 using safe arithmetic
    let numerator = price_spread.safe_mul(e, PRICE_PRECISION_I64);
    numerator.safe_div(e, price_a_i64)
}

pub fn is_swap_risk_reducing(e: &Env, price_deviation: i128, in_idx: u32) -> bool {
    if in_idx > 1 {
        panic_with_error!(&e, PoolValidationError::InTokenOutOfBounds);
    }

    // Pool price lower than peg, buys will be risk reducing
    if price_deviation > 0 {
        if in_idx == 0 {
            false
        } else {
            true
        }
    } else {
        // Pool price higher than peg, sells will be risk reducing
        if in_idx == 0 {
            true
        } else {
            false
        }
    }
}
