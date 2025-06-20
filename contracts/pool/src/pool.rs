use core::cmp::max;

use crate::errors::PoolError;
use crate::errors::PoolValidationError;
use crate::events::Events as LiquidityPoolEvents;
use crate::events::PoolEvents;
use crate::storage::get_last_oracle_valid;
use crate::storage::get_last_trade_ts;
use crate::storage::get_last_update_ts;
use crate::storage::get_router;
use crate::storage::get_volume_24h;
use crate::storage::set_last_trade_ts;
use crate::storage::set_volume_24h;
use crate::storage::{ get_reserve_a, get_reserve_b, set_reserve_a };
use pool_tokens::{ burn_synthetic_tokens, get_total_synthetic_tokens, mint_synthetic_tokens };
use soroban_fixed_point_math::SorobanFixedPoint;
use soroban_sdk::contracttype;
use soroban_sdk::Address;
use soroban_sdk::IntoVal;
use soroban_sdk::Symbol;
use soroban_sdk::Vec;
use soroban_sdk::{ panic_with_error, Env };

use utils::constant::PRICE_TIMES_AMM_TO_QUOTE_PRECISION_RATIO_I128;
use utils::constant::TWENTY_FOUR_HOUR;
use utils::constant::{ FEE_MULTIPLIER, PRICE_PRECISION };
use utils::math::safe_math::SafeMath;
use utils::math::stats::calculate_rolling_sum;
use utils::state::oracle_registry::OraclePriceData;
use utils::state::pool::Pool;
use utils::token::get_token_balance;
use utils::validate;

// Gets the current pool liquidity imbalance.
//
// # Arguments
//
// * base_oracle_price - Price of the base token.
// * quote_oracle_price - Price of the quote token.
//
// # Returns
//
// The liquidity imbalance of the pool as an i128.
pub fn get_net_liquidity_imbalance(
    e: &Env,
    base_oracle_price: u128,
    quote_oracle_price: u128
) -> i128 {
    validate!(e, base_oracle_price > 0, PoolError::InvalidOracle);
    validate!(e, quote_oracle_price > 0, PoolError::InvalidOracle);

    let base_token_supply = get_total_synthetic_tokens(&e);
    let reserve_b = get_reserve_b(e);

    let net_base_asset_value = (base_token_supply as i128)
        .safe_mul(e, base_oracle_price as i128)
        .safe_div(e, PRICE_TIMES_AMM_TO_QUOTE_PRECISION_RATIO_I128);

    let net_quote_asset_value = (reserve_b as i128)
        .safe_mul(e, quote_oracle_price as i128)
        .safe_div(e, PRICE_TIMES_AMM_TO_QUOTE_PRECISION_RATIO_I128);

    net_quote_asset_value.safe_sub(e, net_base_asset_value)
}

pub fn get_oracle_price(e: Env, asset_id: Symbol, now: u64) -> OraclePriceData {
    let oracle_price_data: OraclePriceData = e.invoke_contract(
        &get_router(&e),
        &Symbol::new(&e, "get_price"),
        Vec::from_array(&e, [
            e.current_contract_address().to_val(),
            asset_id.to_val(),
            now.into_val(&e),
        ])
    );
    oracle_price_data
}

pub fn peg_price(e: &Env, base_oracle_price: u128, quote_oracle_price: u128) -> u128 {
    if base_oracle_price == 0 || quote_oracle_price == 0 {
        return 0;
    }

    quote_oracle_price.fixed_div_floor(e, &base_oracle_price, &PRICE_PRECISION)
}

pub fn update_volume_24h(e: &Env, quote_asset_amount: u128, now: u64) {
    let since_last = max(1_u64, now.safe_sub(e, get_last_trade_ts(e)));

    let volume_24h = get_volume_24h(e);

    set_volume_24h(
        e,
        &calculate_rolling_sum(e, volume_24h, quote_asset_amount, since_last, TWENTY_FOUR_HOUR)
    );

    set_last_trade_ts(e, &now);
}

pub fn is_recent_oracle_valid(e: &Env, current_ts: u64) -> bool {
    get_last_oracle_valid(e) && current_ts == get_last_update_ts(e)
}

pub fn get_delta_a(e: &Env, base_oracle_price: u128, quote_oracle_price: u128) -> i128 {
    let (reserve_a, reserve_b) = (get_reserve_a(e), get_reserve_b(e));

    let peg_price = peg_price(e, base_oracle_price, quote_oracle_price);
    let target_reserve_a = reserve_b.fixed_div_floor(e, &peg_price, &PRICE_PRECISION);
    let delta_a = (target_reserve_a as i128).checked_sub(reserve_a as i128).unwrap();

    delta_a
}

// Mints or burns token_a to re-peg the pool's price to it's oracle price.
//
// # Arguments
//
// * `now` - The current timestamp.
pub fn rebalance(e: &Env, base_oracle_price: u128, quote_oracle_price: u128, now: u64) {
    let reserve_a = get_reserve_a(&e);

    // Find the ideal reserve_a amount such that the pool's price is equal to the oracle price
    let delta_a = get_delta_a(&e, base_oracle_price, quote_oracle_price);

    if delta_a > 0 {
        mint_synthetic_tokens(&e, &e.current_contract_address(), delta_a);
        set_reserve_a(&e, &(reserve_a + (delta_a as u128)));
    }
    if delta_a < 0 {
        burn_synthetic_tokens(&e, &e.current_contract_address(), delta_a.abs() as u128);
        set_reserve_a(&e, &(reserve_a - (delta_a.abs() as u128)));
    }

    LiquidityPoolEvents::new(&e).rebalance(delta_a, now);
}

pub fn get_amount_out_strict_receive(
    e: &Env,
    out_amount: u128,
    reserve_sell: u128,
    reserve_buy: u128,
    fee_fraction: u32
) -> (u128, u128) {
    if out_amount == 0 {
        return (0, 0);
    }

    let dy_w_fee = out_amount.fixed_mul_ceil(
        &e,
        &FEE_MULTIPLIER,
        &(FEE_MULTIPLIER - (fee_fraction as u128))
    );
    // if total value including fee is more than the reserve, math can't be done properly
    if dy_w_fee >= reserve_buy {
        panic_with_error!(e, PoolValidationError::InsufficientBalance);
    }
    // +1 just in case there were some rounding errors & convert to real units in place
    let result =
        reserve_buy.fixed_mul_floor(&e, &reserve_sell, &(reserve_buy - dy_w_fee)) -
        reserve_sell +
        1;
    (result, dy_w_fee - out_amount)
}
