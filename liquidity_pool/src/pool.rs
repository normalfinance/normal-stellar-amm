use crate::storage::{
    get_fee_fraction, get_oracle, get_reserve_a, get_reserve_b, get_target_asset, put_reserve_a,
};
use crate::token::{burn_a, mint_a};
use crate::{constants::FEE_MULTIPLIER, errors::LiquidityPoolValidationError};
// use sep_40_oracle::PriceFeedClient;
use soroban_fixed_point_math::SorobanFixedPoint;
use soroban_sdk::{panic_with_error, Env, U256};
use utils::u256_math::ExtraMath;

//
pub fn rebalance_pool(e: &Env, a_to_b: bool) {
    // Compute the price difference between the oracle price and the current pool price
    let oracle_price = get_oracle_price(&e, false);
    let current_price = get_current_price(e, a_to_b);
    let price_delta = oracle_price - current_price;

    // Find the ideal reserve_a amount such that the pool's price is equal to the oracle price
    // A_new = sqrt(k / P_target)
    let reserve_a = U256::from_u128(&e, get_reserve_a(&e));
    let reserve_b = U256::from_u128(&e, get_reserve_b(&e));

    let k = reserve_a.mul(&reserve_b);
    let target_reserve_a = k.div(&U256::from_u128(&e, oracle_price)).sqrt();

    if price_delta > 0 {
        let amount_to_mint = target_reserve_a.sub(&reserve_a).to_u128().unwrap();

        mint_a(&e, &e.current_contract_address(), amount_to_mint);
        put_reserve_a(&e, reserve_a.to_u128().unwrap() + amount_to_mint);
    } else {
        let amount_to_burn = reserve_a.sub(&target_reserve_a).to_u128().unwrap();

        burn_a(&e, &e.current_contract_address(), amount_to_burn);
        put_reserve_a(&e, reserve_a.to_u128().unwrap() + amount_to_burn);
    }
}

pub fn get_oracle_price(e: &Env, squared: bool) -> u128 {
    let oracle = get_oracle(&e);
    // let price_feed_client = PriceFeedClient::new(&e, &oracle);

    let target_asset = get_target_asset(&e);
    // let oracle_price_data = price_feed_client.lastprice(&target_asset).unwrap();

    // TODO: oracle price checks and validation

    // let oracle_price: u128 = oracle_price_data.price as u128;
    let oracle_price: u128 = 0 as u128;

    if squared {
        oracle_price.isqrt()
    } else {
        oracle_price
    }
}

pub fn get_current_price(e: &Env, _a_to_b: bool) -> u128 {
    let (reserve_a, reserve_b) = (get_reserve_a(&e), get_reserve_b(&e));

    let current_price = U256::from_u128(&e, reserve_b)
        .div(&U256::from_u128(&e, reserve_a))
        .to_u128()
        .unwrap();

    current_price
}

// Δx = (x / y) ⋅ Δy
// x = P target / y
pub fn get_mint_amount(e: &Env, delta_b: u128, reserve_a: u128, reserve_b: u128) -> u128 {
    if delta_b == 0 {
        return 0;
    }

    // Initial deposit
    if reserve_a == 0 && reserve_b == 0 {
        let oracle_price = get_oracle_price(e, false);
        let amount_to_mint = U256::from_u128(&e, oracle_price)
            .div(&U256::from_u128(&e, delta_b))
            .to_u128()
            .unwrap();

        amount_to_mint;
    }

    let amount_to_mint = U256::from_u128(&e, reserve_a)
        .div(&U256::from_u128(&e, reserve_b))
        .mul(&U256::from_u128(&e, delta_b))
        .to_u128()
        .unwrap();

    amount_to_mint
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
