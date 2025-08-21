use sep_40_oracle::{ Asset, PriceFeedClient };
use soroban_sdk::{ Address, Env, Symbol };
use utils::{
    constant::{ FIVE_MINUTE, PERCENTAGE_PRECISION_U64, PRICE_PRECISION_U64 },
    math::{ pool::sanitize_new_price, safe_math::SafeMath, stats::calculate_new_twap },
    state::oracle_registry::{ NormalAction, OraclePriceData },
    temporal::Delay,
};

use crate::{
    storage::{ get_oracle_guard_rails, put_historical_oracle_data },
    storage_types::{ HistoricalOracleData, OracleStatus, OracleValidity },
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
pub fn get_oracle_price(e: &Env, oracle: &Address, asset: &Address, now: u64) -> OraclePriceData {
    assert!(now > 0, "now timestamp must be positive");
    
    let oracle_client = PriceFeedClient::new(e, oracle);
    let oracle_asset = Asset::Stellar(asset.clone());

    let oracle_price: u128;
    let published_ts: u64;

    let oracle_price_data = oracle_client.lastprice(&oracle_asset).unwrap();

    oracle_price = oracle_price_data.price as u128;
    published_ts = oracle_price_data.timestamp;

    let oracle_delay = Delay::from_timestamp_diff_expect(
        now, 
        published_ts,
        "Oracle published timestamp cannot be in the future"
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
    registering: bool
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
        last_oracle_price_twap: if registering {
            0
        } else {
            oracle_price_twap
        },
        last_oracle_price: oracle_price_data.price,
        last_oracle_delay: oracle_price_data.delay.as_seconds(),
        last_oracle_price_twap_ts: now,
    };
    put_historical_oracle_data(e, &asset, &new_historical_oracle_data);
}

// Returns a full status summary of the oracle's health and price divergence.
//
// Assesses oracle freshness, price validity, and spread between reserve price
// and TWAP to categorize risk and guide on-chain logic like rebalancing.
//
// # Arguments
// * `e` - Soroban environment reference.
// * `oracle_price_data` - Latest oracle price and delay info.
// * `reserve_price` - Internal reserve-derived price.
// * `last_oracle_price_twap` - Last known oracle TWAP.
//
// # Returns
// - `OracleStatus` struct with validation, divergence, and spread metrics.
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

// Calculates the percentage difference between the reserve price and oracle TWAP.
//
// Used to evaluate whether the oracle data is in line with internal pricing,
// expressed as a percentage spread for risk assessment.
//
// # Arguments
// * `e` - Soroban environment reference.
// * `other_price` - Reserve-derived price.
// * `last_oracle_price_twap` - Oracle's last TWAP.
//
// # Returns
// - The price spread as a percentage (scaled by `PRICE_PRECISION_U64`).
pub fn calculate_oracle_twap_price_spread_pct(
    e: &Env,
    other_price: u128,
    last_oracle_price_twap: u128
) -> i64 {

    assert!(other_price > 0, "other_price must be positive for spread calculation");
    assert!(last_oracle_price_twap >= 0, "last_oracle_price_twap must be non-negative");
    let (price_spread, is_positive_spread) = if other_price >= last_oracle_price_twap {
        let other_price_u64 = (other_price as u64).min(u64::MAX);
        let twap_u64 = (last_oracle_price_twap as u64).min(u64::MAX);
        if other_price_u64 >= twap_u64 {
            (other_price_u64.safe_sub(e, twap_u64), true)
        } else {
            (0, true)  // casting changed order, treat as no spread
        }
    } else {
        let other_price_u64 = (other_price as u64).min(u64::MAX);
        let twap_u64 = (last_oracle_price_twap as u64).min(u64::MAX);
        if twap_u64 >= other_price_u64 {
            (twap_u64.safe_sub(e, other_price_u64), false)
        } else {
            (0, false)  // casting changed order, treat as no spread
        }
    };

    let other_price_u64 = (other_price as u64).min(u64::MAX).max(1);
    let abs_price_spread_pct = price_spread.safe_mul(e, PRICE_PRECISION_U64).safe_div(e, other_price_u64) as i64;
    
    if is_positive_spread {
        abs_price_spread_pct
    } else {
        -abs_price_spread_pct
    }
}

/// Determines whether the oracle data is valid for a specific contract action.
///
/// The oracle validity is evaluated based on the context of the action:
///
/// - `AddLiquidity` / `RemoveLiquidity`: Allowed if oracle data is valid or mildly stale.
/// - `Swap`: Requires strictly valid oracle data.
/// - `UpdateTwap`: Allowed if oracle price is positive.
/// - `Rebalance`: Requires strictly valid oracle data.
/// - `ClaimInsurance`: Disallowed if oracle is too volatile or non-positive.
///
/// If no action is provided, it defaults to requiring strictly valid oracle data.
///
/// # Arguments
/// * `oracle_validity` - Classification of the current oracle state.
/// * `action` - The protocol action being evaluated (optional).
///
/// # Returns
/// - `true` if the oracle data meets the criteria for the specified action.
/// - `false` if the action should be blocked due to stale, volatile, or invalid data.
pub fn is_oracle_valid_for_action(
    oracle_validity: OracleValidity,
    action: Option<NormalAction>
) -> bool {
    let is_ok = match action {
        Some(action) =>
            match action {
                NormalAction::AddLiquidity =>
                    matches!(oracle_validity, OracleValidity::Valid | OracleValidity::StaleForPool),
                NormalAction::RemoveLiquidity =>
                    matches!(oracle_validity, OracleValidity::Valid | OracleValidity::StaleForPool),
                NormalAction::Swap => matches!(oracle_validity, OracleValidity::Valid),
                NormalAction::UpdateTwap => !matches!(oracle_validity, OracleValidity::NonPositive),
                NormalAction::Rebalance => { matches!(oracle_validity, OracleValidity::Valid) }
                NormalAction::ClaimInsurance =>
                    !matches!(
                        oracle_validity,
                        OracleValidity::NonPositive | OracleValidity::TooVolatile
                    ),
            }
        None => { matches!(oracle_validity, OracleValidity::Valid) }
    };

    is_ok
}

// Determines whether an operation should be blocked due to oracle data issues.
//
// Uses oracle validity and price divergence checks to decide whether conditions
// are safe for performing sensitive actions like rebalancing or updates.
//
// # Arguments
// * `e` - Soroban environment reference.
// * `oracle_price_data` - Latest oracle price and delay info.
// * `reserve_price` - Current price from internal reserves.
// * `last_oracle_price_twap` - Last known oracle TWAP.
//
// # Returns
// - `true` if the operation should be blocked due to stale or divergent data.
pub fn block_operation(
    e: &Env,
    oracle_price_data: &OraclePriceData,
    reserve_price: u128,
    last_oracle_price_twap: u128,
    action: NormalAction
) -> bool {
    let OracleStatus {
        oracle_validity,
        price_too_divergent,
        oracle_reserve_price_spread_pct: _,
        ..
    } = get_oracle_status(e, oracle_price_data, reserve_price, last_oracle_price_twap);

    let is_oracle_valid = is_oracle_valid_for_action(oracle_validity, Some(action));

    let block = !is_oracle_valid || price_too_divergent;
    block
}

// Determines whether the oracle price diverges too far from the reserve price.
//
// Uses protocol-defined guard rails to decide if the deviation is outside
// acceptable limits (e.g., >10%) and may indicate manipulation or lag.
//
//
// # Arguments
// * `e` - Soroban environment reference.
// * `price_spread_pct` - Absolute spread percentage between oracle and reserve.
//
// # Returns
// - `true` if the spread exceeds the maximum allowed divergence.
pub fn is_oracle_price_too_divergent(e: &Env, price_spread_pct: i64) -> bool {
    let oracle_guard_rails = get_oracle_guard_rails(e);
    let max_divergence = oracle_guard_rails.price_divergence.oracle_twap_percent_divergence.max(
        PERCENTAGE_PRECISION_U64 / 10
    );
    price_spread_pct.unsigned_abs() > max_divergence
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
    oracle_price_data: &OraclePriceData
) -> OracleValidity {
    let OraclePriceData { price: oracle_price, delay: oracle_delay } = *oracle_price_data;

    let oracle_guard_rails = get_oracle_guard_rails(e);

    // NonPositive
    let is_oracle_price_nonpositive = oracle_price <= 0;

    // Volatility
    // if price / twap >= 1.10 or twap / price >= 1.10 → too volatile
    let max_ratio = oracle_guard_rails.validity.too_volatile_ratio as u128;
    let ratio_1 = oracle_price.safe_div(e, last_oracle_twap.max(1));
    let ratio_2 = last_oracle_twap.safe_div(e, oracle_price.max(1));
    let is_oracle_price_too_volatile = ratio_1 >= max_ratio || ratio_2 >= max_ratio;

    // StaleForPool
    let is_stale_for_pool = oracle_delay.gt(
        &oracle_guard_rails.validity.seconds_before_stale_for_pool
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
