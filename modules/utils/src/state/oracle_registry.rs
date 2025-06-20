use soroban_sdk::{ contracttype, Address };

#[contracttype]
#[derive(Default, Clone, Copy, Debug)]
pub struct OraclePriceData {
    pub price: u128,
    pub delay: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[contracttype]
pub struct OracleInfo {
    pub address: Address,
    pub asset: Address,
    pub decimals: u32,
    pub frozen: bool,
    pub sanitize_clamp_denominator: i64, // zero if not set
    pub last_updated: u64,
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct MutableOracleInfo {
    pub address: Option<Address>,
    pub decimals: Option<u32>,
    pub frozen: Option<bool>,
    pub sanitize_clamp_denominator: Option<i64>,
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
