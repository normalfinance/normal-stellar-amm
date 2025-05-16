use crate::constants::PRICE_PRECISION;
use crate::oracle;
use crate::events::Events as PoolEvents;
use crate::events::LiquidityPoolEvents;
use crate::storage::{ get_fee_fraction, get_reserve_a, get_reserve_b, put_reserve_a };
use crate::{ constants::FEE_MULTIPLIER, errors::LiquidityPoolValidationError };
use soroban_fixed_point_math::SorobanFixedPoint;
use soroban_sdk::{ log, panic_with_error, Env };

use token_synthetic::{ burn_synthetic, mint_synthetic };

// Gets the current pool price.
//
// # Arguments
//
// * a_in_b - Should the price be denominated in Token A or B.
// * in_usd - Should that price be in USD.
//
// # Returns
//
// The price of the pool as a u128.
pub fn get_current_price(e: &Env, a_in_b: bool, in_usd: bool) -> u128 {
    let (reserve_a, reserve_b) = (get_reserve_a(&e), get_reserve_b(&e));

    let mut price = 0_u128;

    if reserve_a == 0 || reserve_b == 0 {
        return price;
    }

    if a_in_b {
        // price of 1 A in terms of B
        price = reserve_b.fixed_div_floor(e, &reserve_a, &PRICE_PRECISION);

        if in_usd {
            let quote_oracle_price = oracle::get_quote_oracle_price(e, false);
            price = price.fixed_mul_floor(e, &quote_oracle_price, &PRICE_PRECISION);
        }
    } else {
        // price of 1 B in terms of A
        price = reserve_a.fixed_div_floor(e, &reserve_b, &PRICE_PRECISION);

        if in_usd {
            let base_oracle_price = oracle::get_base_oracle_price(e, false);
            price = price.fixed_mul_floor(e, &base_oracle_price, &PRICE_PRECISION);
        }
    }

    price
}

// Initializes the liquidity pool.
//
// # Arguments
//
// * `admin` - The address of the admin user.
//
// # Returns
//
// The type of the pool as a Symbol.
pub fn rebalance(e: &Env) {
    // Find the ideal reserve_a amount such that the pool's price is equal to the oracle price
    let reserve_a = get_reserve_a(&e);
    let reserve_b = get_reserve_b(&e);

    let delta_a = get_delta_a(e, reserve_a, reserve_b);

    if delta_a > 0 {
        mint_synthetic(&e, &e.current_contract_address(), delta_a);
        put_reserve_a(&e, reserve_a + (delta_a as u128));
    }
    if delta_a < 0 {
        burn_synthetic(&e, &e.current_contract_address(), delta_a.abs() as u128);
        put_reserve_a(&e, reserve_a - (delta_a.abs() as u128));
    }

    let price = get_current_price(e, true, false);

    let new_reserve_a = get_reserve_a(&e);
    let new_reserve_b = get_reserve_b(&e);

    PoolEvents::new(&e).rebalance(delta_a, reserve_a, reserve_b);
}

pub fn get_delta_a(e: &Env, reserve_a: u128, reserve_b: u128) -> i128 {
    let target_price = oracle::get_target_oracle_price(e, false);
    let target_reserve_a = reserve_b.fixed_div_floor(e, &target_price, &PRICE_PRECISION);
    let delta_a = (target_reserve_a as i128).checked_sub(reserve_a as i128).unwrap();

    delta_a
}

pub fn get_amount_out(
    e: &Env,
    in_amount: u128,
    reserve_sell: u128,
    reserve_buy: u128
) -> (u128, u128) {
    if in_amount == 0 {
        return (0, 0);
    }

    // in * reserve_buy / (reserve_sell + in) - fee
    let fee_fraction = get_fee_fraction(&e);
    let result = in_amount.fixed_mul_floor(&e, &reserve_buy, &(reserve_sell + in_amount));
    let fee = result.fixed_mul_ceil(&e, &(fee_fraction as u128), &FEE_MULTIPLIER);
    (result - fee, fee)
}

pub fn get_amount_out_strict_receive(
    e: &Env,
    out_amount: u128,
    reserve_sell: u128,
    reserve_buy: u128
) -> (u128, u128) {
    if out_amount == 0 {
        return (0, 0);
    }

    let dy_w_fee = out_amount.fixed_mul_ceil(
        &e,
        &FEE_MULTIPLIER,
        &(FEE_MULTIPLIER - (get_fee_fraction(&e) as u128))
    );
    // if total value including fee is more than the reserve, math can't be done properly
    if dy_w_fee >= reserve_buy {
        panic_with_error!(e, LiquidityPoolValidationError::InsufficientBalance);
    }
    // +1 just in case there were some rounding errors & convert to real units in place
    let result =
        reserve_buy.fixed_mul_floor(&e, &reserve_sell, &(reserve_buy - dy_w_fee)) -
        reserve_sell +
        1;
    (result, dy_w_fee - out_amount)
}
