use sep_40_oracle::{ Asset, PriceFeedClient };
use soroban_fixed_point_math::FixedPoint;
use soroban_sdk::{ panic_with_error, Address, Env };
use utils::{
    constant::{ FIVE_MINUTE, PRICE_PRECISION, PRICE_PRECISION_I64 },
    math::{ pool::sanitize_new_price, safe_math::SafeMath, stats::calculate_new_twap },
    oracle::OraclePriceData,
    storage::AssetId,
};

use crate::{
    errors::OracleRegistryError,
    storage::{ get_historical_oracle_data, put_historical_oracle_data, HistoricalOracleData },
};

pub fn update_twap(
    e: &Env,
    asset_id: AssetId,
    oracle_price_data: &OraclePriceData,
    sanitize_clamp_denominator: Option<i64>,
    now: u64
) {
    let historical_oracle_data = get_historical_oracle_data(e, asset_id.clone());

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
    put_historical_oracle_data(e, asset_id, &new_historical_oracle_data);
}

pub fn get_oracle_price(e: &Env, oracle: &Address, asset: &Address, now: u64) -> OraclePriceData {
    let oracle_client = PriceFeedClient::new(e, oracle);
    let oracle_asset = Asset::Stellar(asset.clone());

    let oracle_price: u128;
    let published_ts: u64;

    let oracle_price_data = oracle_client.lastprice(&oracle_asset).unwrap();
    // let decimals = oracle_client.decimals();

    if
        oracle_price_data.timestamp + 24 * 60 * 60 < e.ledger().timestamp() ||
        oracle_price_data.price <= 0
    {
        panic_with_error!(e, OracleRegistryError::InvalidPrice);
    }

    oracle_price = oracle_price_data.price as u128;
    published_ts = oracle_price_data.timestamp;

    let oracle_delay = now.safe_sub(e, published_ts);

    OraclePriceData {
        price: oracle_price,
        delay: oracle_delay,
    }
}
