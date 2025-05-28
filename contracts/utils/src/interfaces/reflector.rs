use sep_40_oracle::{ Asset, PriceData };
use soroban_sdk::{ contractclient, Address, Env, Vec };

#[contractclient(name = "ReflectorOracleClient")]
pub trait ReflectorOracle {
    /// Return the base asset the price is reported in
    fn base(env: Env) -> Asset;

    /// Return all assets quoted by the price feed
    fn assets(env: Env) -> Vec<Asset>;

    /// Return the number of decimals for all assets quoted by the oracle
    fn decimals(env: Env) -> u32;

    /// Return default tick period timeframe (in seconds)
    fn resolution(env: Env) -> u32;

    /// Get price in base asset at specific timestamp
    fn price(env: Env, asset: Asset, timestamp: u64) -> Option<PriceData>;

    /// Get last N price records
    fn prices(env: Env, asset: Asset, records: u32) -> Option<Vec<PriceData>>;

    /// Get the most recent price for an asset
    fn lastprice(env: Env, asset: Asset) -> Option<PriceData>;

    //get the most recent cross price record for the pair of assets
    fn x_last_price(e: Env, base_asset: Asset, quote_asset: Asset) -> Option<PriceData>;

    //get the cross price for the pair of assets at specific timestamp
    fn x_price(e: Env, base_asset: Asset, quote_asset: Asset, timestamp: u64) -> Option<PriceData>;

    //get last N cross price records of for the pair of assets
    fn x_prices(
        e: Env,
        base_asset: Asset,
        quote_asset: Asset,
        records: u32
    ) -> Option<Vec<PriceData>>;

    //get the time-weighted average price for the given asset over N recent records
    fn twap(e: Env, asset: Asset, records: u32) -> Option<i128>;

    //get the time-weighted average cross price for the given asset pair over N recent records
    fn x_twap(e: Env, base_asset: Asset, quote_asset: Asset, records: u32) -> Option<i128>;

    //get historical records retention period, in seconds
    fn period(e: Env) -> Option<u64>;

    //get contract protocol version
    fn version(e: Env) -> u32;

    //get contract admin address
    fn admin(e: Env) -> Option<Address>;
}
