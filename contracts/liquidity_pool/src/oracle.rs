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
        OracleSource,
        OracleStatus,
    },
};

use crate::{
    errors::LiquidityPoolError,
    pool::Pool,
    storage::{ get_historical_oracle_data, put_historical_oracle_data },
};

#[contracttype]
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
    let price_feed_client = ReflectorOracleClient::new(&e, oracle);

    let oracle_price: i128;
    let oracle_conf: u64;
    let mut has_sufficient_data_points: bool = true;
    let mut oracle_precision: u128;
    let published_ts: u64;

    let oracle_price_data = price_feed_client.lastprice(asset).unwrap();

    oracle_price = oracle_price_data.price;
    // FIXME: unsupported by reflector
    oracle_conf = 0;
    // oracle_precision = (10_u128).pow(oracle_price_data.exponent.unsigned_abs());
    oracle_precision = (10_u128).pow(1);
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
        .fixed_mul_floor(oracle_scale_mult as i128, oracle_scale_div as i128)
        .unwrap();

    let oracle_conf_scaled = oracle_conf
        .fixed_mul_floor(oracle_scale_mult as u64, oracle_scale_div as u64)
        .unwrap();

    let oracle_delay: i64 = (now as i64).safe_sub(e, published_ts as i64);

    OraclePriceData {
        price: oracle_price_scaled as i64,
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
pub fn get_target_oracle_price(e: &Env, pool: &Pool) -> u128 {
    let base_oracle_price_data = get_base_oracle_price(e, pool);
    let quote_oracle_price_data = get_quote_oracle_price(e, pool);

    if base_oracle_price_data.price == 0 || quote_oracle_price_data.price == 0 {
        return 0;
    }

    // validate price...

    // update historical oracle data
    let new_historical_data = HistoricalOracleData {
        last_oracle_price: 0,
        last_oracle_conf: 0,
        last_oracle_delay: 0,
        last_oracle_price_twap: 0,
        last_oracle_price_twap_ts: 0,
    };
    put_historical_oracle_data(e, &new_historical_data);

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
pub fn get_base_oracle_price(e: &Env, pool: &Pool) -> OraclePriceData {
    get_oracle_price(
        e,
        &pool.base_oracle.source,
        &pool.base_oracle.address,
        &pool.target_asset,
        e.ledger().timestamp()
    )
}

// Gets the quote (token_b) oracle price.
//
// # Arguments
//
// # Returns
//
// The price of the token as a u128.
pub fn get_quote_oracle_price(e: &Env, pool: &Pool) -> OraclePriceData {
    get_oracle_price(
        e,
        &pool.quote_oracle.source,
        &pool.quote_oracle.address,
        &Asset::Other(Symbol::new(e, "XLM")),
        e.ledger().timestamp()
    )
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
