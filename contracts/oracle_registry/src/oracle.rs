use soroban_fixed_point_math::FixedPoint;
use soroban_sdk::{ panic_with_error, Address, Env };
use utils::{
    constant::{ PRICE_PRECISION, PRICE_PRECISION_I64 },
    math::safe_math::SafeMath,
    oracle::{ OraclePriceData, OracleSource },
};

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
