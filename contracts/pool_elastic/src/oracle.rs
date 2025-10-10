use pool_validation_errors::PoolValidationError;
use sep_40_oracle::{Asset, PriceFeedClient};
use soroban_sdk::{panic_with_error, Env, Symbol};
use utils::{
    constant::{
        FIVE_MINUTE, PERCENTAGE_PRECISION, PERCENTAGE_PRECISION_U64, PRICE_PRECISION,
        PRICE_PRECISION_I64,
    },
    math::{
        pool::sanitize_new_price,
        safe_math::{PrecisionMath, SafeConversion, SafeMath},
        stats::calculate_new_twap,
    },
    state::oracle_registry::{
        HistoricalOracleData, OracleGuardRails, OraclePriceData, OracleValidity,
    },
    temporal::Delay,
};

use crate::storage::{
    get_historical_oracle_data, get_oracle, get_oracle_guard_rails, set_historical_oracle_data,
};

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
pub fn get_oracle_price(e: &Env, asset: &Symbol, now: u64) -> OraclePriceData {
    assert!(now > 0, "now timestamp must be positive");

    let oracle_addr = get_oracle(e);
    let oracle_client = PriceFeedClient::new(e, &oracle_addr);
    let oracle_asset = Asset::Other(asset.clone());

    let oracle_price: u128;
    let published_ts: u64;

    let oracle_price_data = oracle_client.lastprice(&oracle_asset).unwrap();

    oracle_price = (oracle_price_data.price as u128).safe_div(&e, PRICE_PRECISION);
    published_ts = oracle_price_data.timestamp;

    let oracle_delay = Delay::from_timestamp_diff_expect(
        now,
        published_ts,
        "Oracle published timestamp exceeds allowed clock drift tolerance",
    );

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
// * `historical_oracle_data` - The previously recorded oracle data.
// * `oracle_price_data` - The newly observed price and timestamp.
// * `sanitize_clamp_denominator` - Clamp denominator for price sanitization.
// * `now` - Current timestamp.
pub fn update_twap(
    e: &Env,
    historical_oracle_data: &HistoricalOracleData,
    oracle_price_data: &OraclePriceData,
    sanitize_clamp_denominator: u64,
    now: u64,
) {
    let capped_oracle_update_price = sanitize_new_price(
        e,
        oracle_price_data.price,
        historical_oracle_data.last_price_twap,
        sanitize_clamp_denominator,
    );

    let oracle_price_twap = calculate_new_twap(
        e,
        capped_oracle_update_price,
        now,
        historical_oracle_data.last_price_twap,
        historical_oracle_data.last_update_ts,
        FIVE_MINUTE as u64,
    );

    set_historical_oracle_data(
        e,
        &(HistoricalOracleData {
            last_price_twap: oracle_price_twap,
            last_price: oracle_price_data.price,
            last_update_ts: now,
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

    // Use round-to-nearest for volatility calculation (fair assessment)
    let price_delta = oracle_price
        .safe_fixed_div_round(e, last_oracle_twap, PERCENTAGE_PRECISION)
        .safe_to_u64(e);

    let is_oracle_price_too_volatile = price_delta <= lower_bound || upper_bound <= price_delta;

    // StaleForPool
    let is_stale_for_pool = oracle_delay
        .as_seconds()
        .ge(&oracle_guard_rails.validity.seconds_before_stale_for_pool);

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

pub fn get_oracle_price_with_validity(
    e: &Env,
    asset: &Symbol,
    current_time: u64,
) -> HistoricalOracleData {
    let oracle_price_data = get_oracle_price(&e, &asset, current_time);

    let historical_oracle_data = get_historical_oracle_data(e);

    let oracle_validity = oracle_validity(
        &e,
        historical_oracle_data.last_price_twap,
        &oracle_price_data,
    );

    if oracle_validity != OracleValidity::Valid {
        panic_with_error!(e, PoolValidationError::InvalidOracle);
    }

    update_twap(
        e,
        &historical_oracle_data,
        &oracle_price_data,
        1,
        current_time,
    );

    get_historical_oracle_data(e)
}

// Calculates the percentage difference between the pool price and oracle TWAP.
//
// Used to evaluate whether the oracle data is in line with internal pricing,
// expressed as a percentage spread for risk assessment.
//
// # Arguments
// * `e` - Soroban environment reference.
// * `pool_price` - The current pool price.
// * `last_oracle_price_twap` - Oracle's last TWAP.
//
// # Returns
// - The price spread as a percentage (scaled by `PRICE_PRECISION_U64`).
pub fn calculate_oracle_twap_price_spread_pct(
    e: &Env,
    pool_price: u128,
    last_oracle_price_twap: u128,
) -> i64 {
    // Use safe conversions to prevent overflow
    let pool_price_i128 = pool_price.safe_to_i128(e);
    let oracle_price_i128 = last_oracle_price_twap.safe_to_i128(e);

    let price_spread_i128 = pool_price_i128.safe_sub(e, oracle_price_i128);

    // Safe conversion to i64 with overflow protection
    let price_spread = price_spread_i128.safe_to_i64(e);
    let pool_price_i64 = pool_price.safe_to_i64(e);

    // Calculate (price_spread * PRICE_PRECISION_I64) / pool_price_i64 using safe arithmetic
    let numerator = price_spread.safe_mul(e, PRICE_PRECISION_I64);
    numerator.safe_div(e, pool_price_i64)
}

// Determines whether the oracle price diverges too far from the reserve price.
//
// Uses protocol-defined guard rails to decide if the deviation is outside
// acceptable limits (e.g., >10%) and may indicate manipulation or lag.
//
//
// # Arguments
// * `price_spread_pct` - Absolute spread percentage between oracle and reserve.
// * `oracle_guard_rails` - .
//
// # Returns
// - `true` if the spread exceeds the maximum allowed divergence.
pub fn is_oracle_price_too_divergent(
    price_spread_pct: i64,
    oracle_guard_rails: OracleGuardRails,
) -> bool {
    let max_divergence = oracle_guard_rails
        .price_divergence
        .oracle_twap_percent_divergence;

    price_spread_pct.unsigned_abs() > max_divergence
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
