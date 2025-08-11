use sep_40_oracle::{Asset, PriceFeedClient};
use soroban_fixed_point_math::SorobanFixedPoint;
use soroban_sdk::{log, Address, Env, Symbol};
// use soroban_fixed_point_math::
use utils::{
    constant::{FIVE_MINUTE, PERCENTAGE_PRECISION_U64},
    math::{pool::sanitize_new_price, safe_math::SafeMath, stats::calculate_new_twap},
    state::oracle_registry::{HistoricalOracleData, OraclePriceData, OracleValidity},
};

use crate::storage::{get_oracle_guard_rails, put_historical_oracle_data};

// Fetches the latest oracle price and timestamp for a given asset.
//
// Wraps the `PriceFeedClient` to retrieve the last published price and calculates
// the delay since publication based on the current timestamp.
//
// # Arguments
// * `e` - Soroban environment reference.
// * `oracle` - Address of the price oracle contract.
// * `asset` - Address of the asset being queried.
// * `now` - Current timestamp.
//
// # Returns
// - `OraclePriceData` containing the price and delay since last update.
pub fn get_oracle_price(e: &Env, oracle: &Address, asset: &Symbol, now: u64) -> OraclePriceData {
    assert!(now > 0, "now timestamp must be positive");
    let oracle_client = PriceFeedClient::new(e, oracle);
    let oracle_asset = Asset::Other(asset.clone());

    let oracle_price: u128;
    let published_ts: u64;

    let oracle_price_data = oracle_client.lastprice(&oracle_asset).unwrap();

    oracle_price = oracle_price_data.price as u128;
    published_ts = oracle_price_data.timestamp;

    let oracle_delay = now.saturating_sub(published_ts);

    OraclePriceData {
        price: oracle_price,
        delay: oracle_delay,
    }
}

// Updates the time-weighted average price (TWAP) for a given asset using a new oracle price.
//
// The new price is first sanitized to prevent manipulation, then incorporated into the TWAP
// using a weighted rolling average. The result is stored as updated historical oracle data.
//
// # Arguments
// * `e` - Soroban environment reference.
// * `asset` - Symbol of the asset whose TWAP is being updated.
// * `historical_oracle_data` - The previously recorded oracle data.
// * `oracle_price_data` - The newly observed price and timestamp.
// * `sanitize_clamp_denominator` - Clamp denominator for price sanitization.
// * `now` - Current timestamp.
// * `registering` - If true, initializes TWAP to 0 instead of computing it.
pub fn update_twap(
    e: &Env,
    asset: &Symbol,
    historical_oracle_data: &HistoricalOracleData,
    oracle_price_data: &OraclePriceData,
    sanitize_clamp_denominator: u64,
    now: u64,
    registering: bool,
) {
    let capped_oracle_update_price = sanitize_new_price(
        e,
        oracle_price_data.price,
        historical_oracle_data.last_oracle_price_twap,
        sanitize_clamp_denominator,
    );

    let oracle_price_twap = calculate_new_twap(
        e,
        capped_oracle_update_price,
        now,
        historical_oracle_data.last_oracle_price_twap,
        historical_oracle_data.last_oracle_price_twap_ts,
        FIVE_MINUTE as u64,
    );

    put_historical_oracle_data(
        e,
        &asset,
        &(HistoricalOracleData {
            last_oracle_price_twap: oracle_price_twap,
            last_oracle_price: oracle_price_data.price,
            last_oracle_delay: oracle_price_data.delay.as_seconds(),
            last_oracle_price_twap_ts: now,
        }),
    );
}

// Classifies the current oracle price data as valid, stale, or invalid.
//
// Uses three core checks:
// - Price is positive
// - Price is not too volatile relative to last TWAP
// - Price is not too old (stale) for use in pools
//
// # Arguments
// * `e` - Soroban environment reference.
// * `last_oracle_twap` - Previous TWAP value.
// * `oracle_price_data` - Current oracle price and timestamp.
//
// # Returns
// - `OracleValidity` enum indicating the health of the oracle data.
pub fn oracle_validity(
    e: &Env,
    last_oracle_twap: u128,
    oracle_price_data: &OraclePriceData,
) -> OracleValidity {
    let OraclePriceData {
        price: oracle_price,
        delay: oracle_delay,
    } = *oracle_price_data;

    let oracle_guard_rails = get_oracle_guard_rails(e);

    // NonPositive
    let is_oracle_price_nonpositive = oracle_price <= 0;

    // Volatility
    // if Δprice <= 0.80 or 1.20 <= Δprice → too volatile
    let lower_bound =
        PERCENTAGE_PRECISION_U64.safe_sub(e, oracle_guard_rails.validity.too_volatile_ratio);
    let upper_bound = oracle_guard_rails
        .validity
        .too_volatile_ratio
        .safe_add(e, PERCENTAGE_PRECISION_U64);

    let price_delta = oracle_price.safe_div(e, last_oracle_twap.max(1)) as u64;
    let is_oracle_price_too_volatile = price_delta <= lower_bound || upper_bound <= price_delta;

    // StaleForPool
    let is_stale_for_pool =
        oracle_delay.gt(&oracle_guard_rails.validity.seconds_before_stale_for_pool);

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
