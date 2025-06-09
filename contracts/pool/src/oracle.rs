use soroban_fixed_point_math::FixedPoint;
use soroban_sdk::{Env, IntoVal, Symbol, Vec};
use utils::{constant::PRICE_PRECISION, oracle::OraclePriceData, storage::AssetId};

use crate::{pool::Pool, storage::get_oracle_registry};

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
pub fn get_target_oracle_price(e: &Env, pool: &Pool, now: u64) -> u128 {
    let base_oracle_price_data = get_oracle_price(e, &pool.base_asset_id, false, now);
    let quote_oracle_price_data = get_oracle_price(e, &pool.quote_asset_id, false, now);

    if base_oracle_price_data.price == 0 || quote_oracle_price_data.price == 0 {
        return 0;
    }

    // validate price...

    (quote_oracle_price_data.price as u128)
        .fixed_div_floor(base_oracle_price_data.price as u128, PRICE_PRECISION)
        .unwrap()
}

// Gets an oracle price from the Oracle Registry.
//
// # Arguments
//
// * asset_id - The Oracle Registry asset id to get the price for.
// * cached - If the returned price may be cached or fresh.
// * now - The current timestamp.
//
// # Returns
//
// The latest (or cached) oracle price data.
pub fn get_oracle_price(e: &Env, asset_id: &AssetId, cached: bool, now: u64) -> OraclePriceData {
    let oracle_price_data: OraclePriceData = e.invoke_contract(
        &get_oracle_registry(&e),
        &Symbol::new(&e, "get_price"),
        Vec::from_array(
            &e,
            [
                e.current_contract_address().to_val(),
                asset_id.to_val(),
                now.into_val(&e),
            ],
        ),
    );
    oracle_price_data
}
