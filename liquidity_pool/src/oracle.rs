use sep_40_oracle::{Asset, PriceFeedClient};
use soroban_sdk::{Address, Env, Symbol};

use crate::storage::{get_base_oracle, get_quote_oracle, get_target_asset};

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

pub fn get_base_oracle_price(e: &Env, squared: bool) -> u128 {
    let base_oracle = get_base_oracle(e);
    let target_asset = get_target_asset(&e);
    get_oracle_price(e, &base_oracle, &target_asset, squared)
}

pub fn get_quote_oracle_price(e: &Env, squared: bool) -> u128 {
    let quote_oracle = get_quote_oracle(e);
    get_oracle_price(
        e,
        &quote_oracle,
        &Asset::Other(Symbol::new(e, "XLM")),
        squared,
    )
}
