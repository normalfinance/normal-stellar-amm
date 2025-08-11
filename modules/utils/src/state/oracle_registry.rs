use soroban_sdk::{contracttype, Address};

use crate::constant::PRICE_PRECISION;

#[contracttype]
#[derive(Default, Clone, Copy, Debug)]
pub struct OraclePriceData {
    pub price: u128,
    pub delay: u64,
}

#[derive(Clone, Copy, Eq, PartialEq, Debug, Default)]
pub enum OracleSource {
    #[default]
    Reflector,
    DIA,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[contracttype]
pub struct OracleInfo {
    pub address: Address,
    // pub source: OracleSource, // coming soon
    pub decimals: u32,
    pub frozen: bool,
    pub sanitize_clamp_denominator: u64, // zero if not set
    pub last_updated: u64,
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct MutableOracleInfo {
    pub address: Option<Address>,
    pub decimals: Option<u32>,
    pub frozen: Option<bool>,
    pub sanitize_clamp_denominator: Option<u64>,
}

impl MutableOracleInfo {
    pub fn new() -> Self {
        MutableOracleInfo {
            address: None,
            decimals: None,
            frozen: None,
            sanitize_clamp_denominator: None,
        }
    }
}

// Actions dependant on oracle prices
#[derive(Clone, Copy, PartialEq, Debug, Eq)]
#[contracttype]
pub enum NormalAction {
    AddLiquidity,
    RemoveLiquidity,
    Swap,
    UpdateTwap,     // Save time-weighted average price to historical oracle data
    Rebalance, // Mint or burn synthetic tokens (token_a) in a Pool to peg its price to an oracle
    ClaimInsurance, // Cover a pool liquidity deficit with a Buffer reserve and/or Insurance Fund stakes
}

#[contracttype]
#[derive(Default, Clone, Copy, Eq, PartialEq, Debug)]
pub struct HistoricalOracleData {
    pub last_oracle_price: u128,
    pub last_oracle_delay: u64, // amount of time since last update.
    pub last_oracle_price_twap: u128,
    pub last_oracle_price_twap_ts: u64, // unix_timestamp of last snapshot.
}

impl HistoricalOracleData {
    pub fn default_quote_oracle() -> Self {
        HistoricalOracleData {
            last_oracle_price: PRICE_PRECISION,
            last_oracle_delay: 0,
            last_oracle_price_twap: PRICE_PRECISION,
            ..HistoricalOracleData::default()
        }
    }

    pub fn default_price(price: u128) -> Self {
        HistoricalOracleData {
            last_oracle_price: price,
            last_oracle_delay: 10,
            last_oracle_price_twap: price,
            ..HistoricalOracleData::default()
        }
    }

    pub fn default_with_current_oracle(oracle_price_data: OraclePriceData, now: u64) -> Self {
        HistoricalOracleData {
            last_oracle_price: oracle_price_data.price,
            last_oracle_delay: oracle_price_data.delay,
            last_oracle_price_twap: oracle_price_data.price,
            last_oracle_price_twap_ts: now,
        }
    }
}
