use sep_40_oracle::{ Asset, PriceFeedClient };
use soroban_sdk::{ panic_with_error, Address, Env, Symbol };
use soroban_fixed_point_math::FixedPoint;
use utils::{
    constant::{PRICE_PRECISION, PRICE_PRECISION_I64},
    interfaces::reflector::ReflectorOracle,
    oracle::{ OraclePriceData, OracleSource },
    storage::OraclePriceData,
};

use crate::{ errors::LiquidityPoolError, pool::Pool, storage::{ put_historical_oracle_data } };

#[derive(Default, Clone, Copy, Eq, PartialEq, Debug)]
pub struct HistoricalOracleData {
    /// precision: PRICE_PRECISION
    pub last_oracle_price: i64,
    /// precision: PRICE_PRECISION
    pub last_oracle_conf: u64,
    /// number of slots since last update
    pub last_oracle_delay: i64,
    /// precision: PRICE_PRECISION
    pub last_oracle_price_twap: i64,
    /// unix_timestamp of last snapshot
    pub last_oracle_price_twap_ts: i64,
}

impl HistoricalOracleData {
    pub fn default_quote_oracle() -> Self {
        HistoricalOracleData {
            last_oracle_price: PRICE_PRECISION_I64,
            last_oracle_conf: 0,
            last_oracle_delay: 0,
            last_oracle_price_twap: PRICE_PRECISION_I64,
            ..HistoricalOracleData::default()
        }
    }

    pub fn default_price(price: i64) -> Self {
        HistoricalOracleData {
            last_oracle_price: price,
            last_oracle_conf: 0,
            last_oracle_delay: 10,
            last_oracle_price_twap: price,
            ..HistoricalOracleData::default()
        }
    }

    pub fn default_with_current_oracle(oracle_price_data: OraclePriceData) -> Self {
        HistoricalOracleData {
            last_oracle_price: oracle_price_data.price,
            last_oracle_conf: oracle_price_data.confidence,
            last_oracle_delay: oracle_price_data.delay,
            last_oracle_price_twap: oracle_price_data.price,
            // last_oracle_price_twap_ts: now,
            ..HistoricalOracleData::default()
        }
    }
}

pub fn get_oracle_price(
    e: &Env,
    oracle_source: &OracleSource,
    price_oracle: &Address,
    asset: &Asset,
    now: u64
) -> OraclePriceData {
    match oracle_source {
        OracleSource::Reflector => get_reflector_price(e, price_oracle, asset, now, 1),
        // OracleSource::Band => None,
        OracleSource::QuoteAsset =>
            OraclePriceData {
                price: PRICE_PRECISION_I64,
                confidence: 1,
                delay: 0,
                has_sufficient_data_points: true,
            },
    }
}

pub fn get_reflector_price(
    e: &Env,
    oracle: &Address,
    asset: &Asset,
    now: u64,
    multiple: u128
) -> OraclePriceData {
    let price_feed_client = ReflectorOracle::new(&e, oracle);

    let oracle_price: i128;
    let oracle_conf: u64;
    let mut has_sufficient_data_points: bool = true;
    let mut oracle_precision: u128;
    let published_ts: u64;

    let oracle_price_data = price_feed_client.lastprice(asset).unwrap();

    oracle_price = oracle_price_data.price;
    // FIXME: unsupported by reflector
    oracle_conf = 0;
    oracle_precision = (10_u128).pow(oracle_price_data.exponent.unsigned_abs());
    published_ts = oracle_price_data.timestamp;

    if oracle_precision <= multiple {
        // msg!("Multiple larger than oracle precision");
        panic_with_error!(e, LiquidityPoolError::InvalidOracle);
    }
    oracle_precision = oracle_precision.checked_div(multiple).unwrap();

    let mut oracle_scale_mult = 1;
    let mut oracle_scale_div = 1;

    if oracle_precision > PRICE_PRECISION {
        oracle_scale_div = oracle_precision.checked_div(PRICE_PRECISION).unwrap();
    } else {
        oracle_scale_mult = PRICE_PRECISION.checked_div(oracle_precision).unwrap();
    }

    let oracle_price_scaled = (oracle_price as i128)
        .fixed_mul_floor(oracle_scale_mult, oracle_scale_div)
        .unwrap();

    let oracle_price_scaled = (oracle_conf as u128)
        .fixed_mul_floor(oracle_scale_mult, oracle_scale_div)
        .unwrap();

    let oracle_delay: i64 = now.checked_sub(published_ts).unwrap();

    OraclePriceData {
        price: oracle_price_scaled,
        confidence: oracle_conf_scaled,
        delay: oracle_delay,
        has_sufficient_data_points,
    }
}

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
pub fn get_target_oracle_price(e: &Env) -> u128 {
    let base_oracle_price_data = get_base_oracle_price(e);
    let quote_oracle_price_data = get_quote_oracle_price(e);

    if base_oracle_price_data.price == 0 || quote_oracle_price_data.price == 0 {
        return 0;
    }

    // validate price...

    // update historical oracle data
    let new_historical_data = HistoricalOracleData {
        last_oracle_price: (),
        last_oracle_conf: (),
        last_oracle_delay: (),
        last_oracle_price_twap: (),
        last_oracle_price_twap_ts: (),
    };
    put_historical_oracle_data(e, new_historical_data);

    quote_oracle_price_data.price
        .fixed_div_floor(base_oracle_price_data.price, PRICE_PRECISION)
        .unwrap()
}

// Gets the base (token_a) oracle price.
//
// # Arguments
//
// # Returns
//
// The price of the token as a u128.
pub fn get_base_oracle_price(e: &Env) -> OraclePriceData {
    let base_oracle = get_base_oracle(e);
    let target_asset = get_target_asset(&e);
    get_oracle_price(e, &base_oracle, &target_asset, e.ledger().timestamp(), 1)
}

// Gets the quote (token_b) oracle price.
//
// # Arguments
//
// # Returns
//
// The price of the token as a u128.
pub fn get_quote_oracle_price(e: &Env) -> OraclePriceData {
    let quote_oracle = get_quote_oracle(e);
    get_oracle_price(
        e,
        &quote_oracle,
        &Asset::Other(Symbol::new(e, "XLM")),
        e.ledger().timestamp(),
        1
    )
}

pub fn block_operation(
    e: &Env,
    pool: &Pool,
    oracle_price_data: &OraclePriceData,
    reserve_price: u64,
    slot: u64
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
    let oracle_validity = oracle_validity(
        e,
        pool.market_index,
        pool.historical_oracle_data.last_oracle_price_twap,
        oracle_price_data,
        &pool.guard_rails.validity,
        pool.get_max_confidence_interval_multiplier(),
        false
    );
    let oracle_reserve_price_spread_pct = calculate_oracle_twap_price_spread_pct(
        e,
        &pool,
        reserve_price
    );
    let is_oracle_price_too_divergent = is_oracle_price_too_divergent(
        oracle_reserve_price_spread_pct,
        &guard_rails.price_divergence
    );

    OracleStatus {
        price_data: *oracle_price_data,
        oracle_reserve_price_spread_pct,
        price_too_divergent: is_oracle_price_too_divergent,
        oracle_validity,
    }
}

pub fn calculate_oracle_twap_price_spread_pct(e: &Env, pool: &Pool, other_price: u64) -> i64 {
    // let price_spread = other_price.safe_sub(
    //     pool.historical_oracle_data.last_oracle_price_twap_5min
    // );
    let price_spread = other_price.safe_sub(
        e,
        pool.historical_oracle_data.last_oracle_price_twap_5min
    );

    // price_spread_pct
    price_spread.safe_mul(e, BID_ASK_SPREAD_PRECISION_I128)?.safe_div(e, other_price)
}
