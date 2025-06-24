use soroban_sdk::{ contracttype, Address };

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

// Actions dependant on oracle prices
#[derive(Clone, Copy, PartialEq, Debug, Eq)]
#[contracttype]
pub enum NormalAction {
    AddLiquidity,
    RemoveLiquidity,
    Swap,
    UpdateTwap, // Save time-weighted average price to historical oracle data
    Rebalance, // Mint or burn synthetic tokens (token_a) in a Pool to peg its price to an oracle
    ClaimInsurance, // Cover a pool liquidity deficit with a Buffer reserve and/or Insurance Fund stakes
}
