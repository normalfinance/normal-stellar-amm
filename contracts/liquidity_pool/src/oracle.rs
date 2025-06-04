use sep_40_oracle::{ Asset };
use soroban_sdk::{ contracttype, panic_with_error, Address, Env, Symbol };
use soroban_fixed_point_math::FixedPoint;
use utils::{
    constant::{ PRICE_PRECISION, PRICE_PRECISION_I128, PRICE_PRECISION_I64, PRICE_PRECISION_U64 },
    interfaces::reflector::ReflectorOracleClient,
    math::safe_math::SafeMath,
    oracle::{
        is_oracle_price_too_divergent,
        is_oracle_valid_for_action,
        oracle_validity,
        NormalAction,
        OraclePriceData,
        OracleStatus,
    },
};

use crate::{
    errors::LiquidityPoolError,
    pool::Pool,
    storage::{ get_historical_oracle_data, put_historical_oracle_data },
};

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
pub fn get_target_oracle_price(e: &Env, pool: &Pool) -> u128 {
    let now = e.ledger().timestamp();
    let base_oracle_price_data = get_base_oracle_price(e, pool, now);
    let quote_oracle_price_data = get_quote_oracle_price(e, pool, now);

    if base_oracle_price_data.price == 0 || quote_oracle_price_data.price == 0 {
        return 0;
    }

    // validate price...

    (quote_oracle_price_data.price as u128)
        .fixed_div_floor(base_oracle_price_data.price as u128, PRICE_PRECISION)
        .unwrap()
}

// Gets the base (token_a) oracle price.
//
// # Arguments
//
// # Returns
//
// The price of the token as a u128.
pub fn get_base_oracle_price(e: &Env, pool: &Pool, now: u64) -> OraclePriceData {
    let oracle_price_data: OraclePriceData = e.invoke_contract(
        &&get_oracle_registry(&e),
        &Symbol::new(&e, "get_price"),
        Vec::from_array(&e, [
            e.current_contract_address().to_val(),
            pool.base_oracle.source.to_val(),
            pool.base_oracle.address.clone().to_val(),
            pool.asset.into_val(&e),
            now.into_val(&e),
        ])
    );
    oracle_price_data
}

// Gets the quote (token_b) oracle price.
//
// # Arguments
//
// # Returns
//
// The price of the token as a u128.
pub fn get_quote_oracle_price(e: &Env, pool: &Pool, now: u64) -> OraclePriceData {
    let oracle_price_data: OraclePriceData = e.invoke_contract(
        &&get_oracle_registry(&e),
        &Symbol::new(&e, "get_price"),
        Vec::from_array(&e, [
            e.current_contract_address().to_val(),
            pool.quote_oracle.source.to_val(),
            pool.quote_oracle.address.clone().to_val(),
            Asset::Other(Symbol::new(e, "XLM")).into_val(&e),
            now.into_val(&e),
        ])
    );
    oracle_price_data
}

pub fn block_operation(
    e: &Env,
    pool: &Pool,
    oracle_price_data: &OraclePriceData,
    reserve_price: u64
) -> bool {
    let OracleStatus {
        oracle_validity,
        price_too_divergent,
        oracle_reserve_price_spread_pct: _,
        ..
    } = get_oracle_status(e, pool, oracle_price_data, reserve_price);

    let is_oracle_valid = is_oracle_valid_for_action(
        oracle_validity,
        Some(NormalAction::Rebalance)
    );

    let block = !is_oracle_valid || price_too_divergent;
    block
}

pub fn get_oracle_status(
    e: &Env,
    pool: &Pool,
    oracle_price_data: &OraclePriceData,
    reserve_price: u64
) -> OracleStatus {
    let historical_oracle_data = get_historical_oracle_data(e);
    let oracle_validity = oracle_validity(
        e,
        e.current_contract_address(),
        historical_oracle_data.last_oracle_price_twap,
        oracle_price_data,
        &pool.oracle_guard_rails.validity,
        pool.get_max_confidence_interval_multiplier(),
        false
    );
    let oracle_reserve_price_spread_pct = calculate_oracle_twap_price_spread_pct(e, reserve_price);
    let is_oracle_price_too_divergent = is_oracle_price_too_divergent(
        oracle_reserve_price_spread_pct,
        &pool.oracle_guard_rails.price_divergence
    );

    OracleStatus {
        price_data: *oracle_price_data,
        oracle_reserve_price_spread_pct,
        price_too_divergent: is_oracle_price_too_divergent,
        oracle_validity,
    }
}

pub fn calculate_oracle_twap_price_spread_pct(e: &Env, other_price: u64) -> i64 {
    let historical_oracle_data = get_historical_oracle_data(e);
    let price_spread = other_price.safe_sub(
        e,
        historical_oracle_data.last_oracle_price_twap as u64
    );

    // price_spread_pct
    price_spread.safe_mul(e, PRICE_PRECISION_U64).safe_div(e, other_price) as i64
}
