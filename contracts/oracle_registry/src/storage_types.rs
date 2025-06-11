use soroban_sdk::contracttype;
use utils::{
    constant::{PERCENTAGE_PRECISION_U64, PRICE_PRECISION},
    errors::oracle_error::OracleError, storage::OraclePriceData,
};

#[contracttype]
#[derive(Default, Clone, Copy, Eq, PartialEq, Debug)]
pub struct HistoricalOracleData {
    // precision: PRICE_PRECISION
    pub last_oracle_price: u128,
    // amount of time since last update
    pub last_oracle_delay: u64,
    // precision: PRICE_PRECISION
    pub last_oracle_price_twap: u128,
    // unix_timestamp of last snapshot
    pub last_oracle_price_twap_ts: u64,
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

    pub fn default_with_current_oracle(oracle_price_data: OraclePriceData) -> Self {
        HistoricalOracleData {
            last_oracle_price: oracle_price_data.price,
            last_oracle_delay: oracle_price_data.delay,
            last_oracle_price_twap: oracle_price_data.price,
            ..HistoricalOracleData::default()
        }
    }
}

#[contracttype]
#[derive(Copy, Clone, Debug)]
pub struct PriceDivergenceGuardRails {
    pub oracle_twap_percent_divergence: u64,
}

#[contracttype]
#[derive(Copy, Clone, Default, Debug)]
pub struct ValidityGuardRails {
    pub slots_before_stale_for_pool: u64,
    pub confidence_interval_max_size: u64,
    pub too_volatile_ratio: i64,
}

#[contracttype]
#[derive(Copy, Clone, Debug)]
pub struct OracleGuardRails {
    pub price_divergence: PriceDivergenceGuardRails,
    pub validity: ValidityGuardRails,
}

impl Default for OracleGuardRails {
    fn default() -> Self {
        OracleGuardRails {
            price_divergence: PriceDivergenceGuardRails {
                oracle_twap_percent_divergence: PERCENTAGE_PRECISION_U64 / 2,
            },
            validity: ValidityGuardRails {
                slots_before_stale_for_pool: 10,      // ~5 seconds
                confidence_interval_max_size: 20_000, // 2% of price
                too_volatile_ratio: 5,                // 5x or 80% down
            },
        }
    }
}

impl OracleGuardRails {
    pub fn max_oracle_twap_percent_divergence(&self) -> u64 {
        self.price_divergence
            .oracle_twap_percent_divergence
            .max(PERCENTAGE_PRECISION_U64 / 2)
    }
}

// ordered by "severity"
#[derive(Clone, Copy, PartialEq, Debug, Eq, Default)]
pub enum OracleValidity {
    NonPositive,
    TooVolatile,
    TooUncertain,
    InsufficientDataPoints,
    StaleForPool,
    #[default]
    Valid,
}

impl OracleValidity {
    pub fn get_error_code(&self) -> OracleError {
        match self {
            OracleValidity::NonPositive => OracleError::OracleNonPositive,
            OracleValidity::TooVolatile => OracleError::OracleTooVolatile,
            OracleValidity::TooUncertain => OracleError::OracleTooUncertain,
            OracleValidity::InsufficientDataPoints => OracleError::OracleInsufficientDataPoints,
            OracleValidity::StaleForPool => OracleError::OracleStaleForPool,
            OracleValidity::Valid => unreachable!(),
        }
    }
}

// Actions dependant on oracle prices
#[derive(Clone, Copy, PartialEq, Debug, Eq)]
pub enum NormalAction {
    UpdateTwap,         // Save time-weighted average price to historical oracle data
    Rebalance, // Mint or burn synthetic tokens (token_a) in a Pool to peg its price to an oracle
    BufferPayout, // Cover a pool liquidity deficit with Buffer reserves
    InsuranceFundClaim, // Cover a pool liquidity deficit with Insurance Fund stakes
}

#[derive(Default, Clone, Copy, Debug)]
pub struct OracleStatus {
    pub price_data: OraclePriceData,
    pub oracle_reserve_price_spread_pct: i64,
    pub price_too_divergent: bool,
    pub oracle_validity: OracleValidity,
}
