use crate::constants::PRICE_PRECISION;
use crate::oracle;
// use crate::events::Events as PoolEvents;
// use crate::events::LiquidityPoolEvents;
use crate::storage::{get_fee_fraction, get_reserve_a, get_reserve_b, put_reserve_a};
use crate::{constants::FEE_MULTIPLIER, errors::LiquidityPoolValidationError};
use soroban_fixed_point_math::SorobanFixedPoint;
use soroban_sdk::{log, panic_with_error, Env};

use token_synthetic::{burn_synthetic, mint_synthetic};

// Gets the current pool price
// * a_in_b - Should the price be denominated in Token A or B.
// * in_usd - Should that price be in USD.
//
pub fn get_pool_price(e: &Env, a_in_b: bool, in_usd: bool) -> u128 {
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

pub fn rebalance_pool(e: &Env) {
    // Compute the price difference between the oracle price and the pool price
    let base_oracle_price = oracle::get_base_oracle_price(e, false);
    log!(e, "base_oracle_price: {}", base_oracle_price);

    // Find the ideal reserve_a amount such that the pool's price is equal to the oracle price
    // A_new = sqrt(k / P_target)
    let reserve_a = get_reserve_a(&e);
    let reserve_b = get_reserve_b(&e);
    log!(e, "reserve_a: {}", reserve_a);
    log!(e, "reserve_b: {}", reserve_b);

    let target_reserve_a = reserve_b.fixed_div_floor(e, &base_oracle_price, &PRICE_PRECISION);
    log!(e, "target_reserve_a: {}", target_reserve_a);

    let delta_a = (target_reserve_a as i128)
        .checked_sub(reserve_a as i128)
        .unwrap();
    log!(e, "delta_a: {}", delta_a);

    if delta_a > 0 {
        mint_synthetic(&e, &e.current_contract_address(), delta_a);
        put_reserve_a(&e, reserve_a + (delta_a as u128));
    }
    if delta_a < 0 {
        burn_synthetic(&e, &e.current_contract_address(), delta_a.abs() as u128);
        put_reserve_a(&e, reserve_a - (delta_a.abs() as u128));
    }

    let price = get_pool_price(e, true, true);
    log!(e, "price_after: {}", price);

    let new_reserve_a = get_reserve_a(&e);
    let new_reserve_b = get_reserve_b(&e);
    log!(e, "new_reserve_a: {}", new_reserve_a);
    log!(e, "new_reserve_b: {}", new_reserve_b);

    // PoolEvents::new(&e).rebalance(user, base_oracle_price, pool_price, 0, reserve_a, reserve_b);
}

pub fn get_amount_out(
    e: &Env,
    in_amount: u128,
    reserve_sell: u128,
    reserve_buy: u128,
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
    reserve_buy: u128,
) -> (u128, u128) {
    if out_amount == 0 {
        return (0, 0);
    }

    let dy_w_fee = out_amount.fixed_mul_ceil(
        &e,
        &FEE_MULTIPLIER,
        &(FEE_MULTIPLIER - (get_fee_fraction(&e) as u128)),
    );
    // if total value including fee is more than the reserve, math can't be done properly
    if dy_w_fee >= reserve_buy {
        panic_with_error!(e, LiquidityPoolValidationError::InsufficientBalance);
    }
    // +1 just in case there were some rounding errors & convert to real units in place
    let result = reserve_buy.fixed_mul_floor(&e, &reserve_sell, &(reserve_buy - dy_w_fee))
        - reserve_sell
        + 1;
    (result, dy_w_fee - out_amount)
}
