use sep_40_oracle::{ Asset, PriceFeedClient };
use soroban_sdk::{ log, panic_with_error, Address, Env, Symbol };
use utils::{
    constant::{ FIVE_MINUTE, PERCENTAGE_PRECISION_U64, PRICE_PRECISION_U64 },
    math::{ pool::sanitize_new_price, safe_math::SafeMath, stats::calculate_new_twap },
    storage::OraclePriceData,
};

use crate::{
    storage::{ get_oracle_guard_rails, put_historical_oracle_data },
    storage_types::{ HistoricalOracleData, NormalAction, OracleStatus, OracleValidity },
};

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
pub fn get_oracle_price(e: &Env, oracle: &Address, asset: &Address, now: u64) -> OraclePriceData {
    let oracle_client = PriceFeedClient::new(e, oracle);
    let oracle_asset = Asset::Stellar(asset.clone());

    let oracle_price: u128;
    let published_ts: u64;

    let oracle_price_data = oracle_client.lastprice(&oracle_asset).unwrap();

    oracle_price = oracle_price_data.price as u128;
    published_ts = oracle_price_data.timestamp;

    let oracle_delay = now.safe_sub(e, published_ts);

    OraclePriceData {
        price: oracle_price,
        delay: oracle_delay,
    }
}

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
pub fn update_twap(
    e: &Env,
    asset_id: &Symbol,
    historical_oracle_data: &HistoricalOracleData,
    oracle_price_data: &OraclePriceData,
    sanitize_clamp_denominator: i64,
    now: u64
) {
    let capped_oracle_update_price = sanitize_new_price(
        e,
        oracle_price_data.price,
        historical_oracle_data.last_oracle_price_twap,
        sanitize_clamp_denominator
    );

    let oracle_price_twap = calculate_new_twap(
        e,
        capped_oracle_update_price,
        now,
        historical_oracle_data.last_oracle_price_twap,
        historical_oracle_data.last_oracle_price_twap_ts,
        FIVE_MINUTE as u64
    );

    let new_historical_oracle_data = HistoricalOracleData {
        last_oracle_price_twap: oracle_price_twap,
        last_oracle_price: oracle_price_data.price,
        last_oracle_delay: oracle_price_data.delay,
        last_oracle_price_twap_ts: now,
    };
    put_historical_oracle_data(e, &asset_id, &new_historical_oracle_data);
}

pub fn block_operation(
    e: &Env,
    oracle_price_data: &OraclePriceData,
    reserve_price: u128,
    last_oracle_price_twap: u128
) -> bool {
    let OracleStatus {
        oracle_validity,
        price_too_divergent,
        oracle_reserve_price_spread_pct: _,
        ..
    } = get_oracle_status(e, oracle_price_data, reserve_price, last_oracle_price_twap);

    let is_oracle_valid = is_oracle_valid_for_action(
        oracle_validity,
        Some(NormalAction::Rebalance)
    );

    let block = !is_oracle_valid || price_too_divergent;
    block
}

pub fn get_oracle_status(
    e: &Env,
    oracle_price_data: &OraclePriceData,
    reserve_price: u128,
    last_oracle_price_twap: u128
) -> OracleStatus {
    let oracle_validity = oracle_validity(e, last_oracle_price_twap, oracle_price_data);
    let oracle_reserve_price_spread_pct = calculate_oracle_twap_price_spread_pct(
        e,
        reserve_price,
        last_oracle_price_twap
    );
    let is_oracle_price_too_divergent = is_oracle_price_too_divergent(
        e,
        oracle_reserve_price_spread_pct
    );

    OracleStatus {
        price_data: *oracle_price_data,
        oracle_reserve_price_spread_pct,
        price_too_divergent: is_oracle_price_too_divergent,
        oracle_validity,
    }
}

pub fn calculate_oracle_twap_price_spread_pct(
    e: &Env,
    other_price: u128,
    last_oracle_price_twap: u128
) -> i64 {
    let price_spread = (other_price as u64).safe_sub(e, last_oracle_price_twap as u64);

    // price_spread_pct
    price_spread.safe_mul(e, PRICE_PRECISION_U64).safe_div(e, other_price as u64) as i64
}

pub fn is_oracle_valid_for_action(
    oracle_validity: OracleValidity,
    action: Option<NormalAction>
) -> bool {
    let is_ok = match action {
        Some(action) =>
            match action {
                NormalAction::UpdateTwap => !matches!(oracle_validity, OracleValidity::NonPositive),
                NormalAction::Rebalance => {
                    matches!(
                        oracle_validity,
                        OracleValidity::Valid |
                            OracleValidity::StaleForPool |
                            OracleValidity::InsufficientDataPoints
                    )
                }
                NormalAction::BufferPayout =>
                    !matches!(
                        oracle_validity,
                        OracleValidity::NonPositive | OracleValidity::TooVolatile
                    ),
                NormalAction::InsuranceFundClaim =>
                    !matches!(
                        oracle_validity,
                        OracleValidity::NonPositive | OracleValidity::TooVolatile
                    ),
            }
        None => { matches!(oracle_validity, OracleValidity::Valid) }
    };

    is_ok
}

pub fn is_oracle_price_too_divergent(e: &Env, price_spread_pct: i64) -> bool {
    let oracle_guard_rails = get_oracle_guard_rails(e);
    let max_divergence = oracle_guard_rails.price_divergence.oracle_twap_percent_divergence.max(
        PERCENTAGE_PRECISION_U64 / 10
    );
    price_spread_pct.unsigned_abs() > max_divergence
}

pub fn oracle_validity(
    e: &Env,
    last_oracle_twap: u128,
    oracle_price_data: &OraclePriceData
) -> OracleValidity {
    let OraclePriceData { price: oracle_price, delay: oracle_delay, .. } = *oracle_price_data;

    let oracle_guard_rails = get_oracle_guard_rails(e);

    let is_oracle_price_nonpositive = oracle_price <= 0;

    let is_oracle_price_too_volatile = oracle_price
        .max(last_oracle_twap)
        .safe_div(e, last_oracle_twap.min(oracle_price).max(1))
        .gt(&(oracle_guard_rails.validity.too_volatile_ratio as u128));

    let is_stale_for_pool = oracle_delay.gt(
        &oracle_guard_rails.validity.slots_before_stale_for_pool
    );

    let oracle_validity = if is_oracle_price_nonpositive {
        OracleValidity::NonPositive
    } else if is_oracle_price_too_volatile {
        OracleValidity::TooVolatile
    } else if is_stale_for_pool {
        OracleValidity::StaleForPool
    } else {
        OracleValidity::Valid
    };

    oracle_validity
}
