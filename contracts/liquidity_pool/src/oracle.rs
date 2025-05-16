use sep_40_oracle::{ Asset, PriceFeedClient };
use soroban_sdk::{ Address, Env, Symbol };
use soroban_fixed_point_math::FixedPoint;

use crate::{
    constants::PRICE_PRECISION,
    storage::{ get_base_oracle, get_quote_oracle, get_target_asset },
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
pub fn get_oracle_price(e: &Env, oracle: &Address, asset: &Asset, squared: bool) -> u128 {
    let price_feed_client = PriceFeedClient::new(&e, oracle);

    // let target_asset = get_target_asset(&e);
    let oracle_price_data = price_feed_client.lastprice(asset).unwrap();

    // TODO: oracle price checks and validation

    let oracle_price: u128 = oracle_price_data.price as u128;

    if squared {
        oracle_price.isqrt()
    } else {
        oracle_price
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
pub fn get_target_oracle_price(e: &Env, squared: bool) -> u128 {
    let base_price = get_base_oracle_price(e, squared);
    let quote_price = get_quote_oracle_price(e, squared);
    quote_price.fixed_div_floor(base_price, PRICE_PRECISION).unwrap()
}

// Gets the base (token_a) oracle price.
//
// # Arguments
//
// * squared - Should the price be square rooted.
//
// # Returns
//
// The price of the token as a u128.
pub fn get_base_oracle_price(e: &Env, squared: bool) -> u128 {
    let base_oracle = get_base_oracle(e);
    let target_asset = get_target_asset(&e);
    get_oracle_price(e, &base_oracle, &target_asset, squared)
}

// Gets the quote (token_b) oracle price.
//
// # Arguments
//
// * squared - Should the price be square rooted.
//
// # Returns
//
// The price of the token as a u128.
pub fn get_quote_oracle_price(e: &Env, squared: bool) -> u128 {
    let quote_oracle = get_quote_oracle(e);
    get_oracle_price(e, &quote_oracle, &Asset::Other(Symbol::new(e, "XLM")), squared)
}
