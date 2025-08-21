use soroban_sdk::contracttype;
use utils::{
    constant::{ PERCENTAGE_PRECISION_U64, PRICE_PRECISION },
    errors::oracle_error::OracleError,
    state::oracle_registry::OraclePriceData,
};

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

    pub fn default_with_current_oracle(oracle_price_data: OraclePriceData) -> Self {
        HistoricalOracleData {
            last_oracle_price: oracle_price_data.price,
            last_oracle_delay: oracle_price_data.delay.as_seconds(),
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
    pub seconds_before_stale_for_pool: u64,
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
                oracle_twap_percent_divergence: PERCENTAGE_PRECISION_U64 / 10, // 10%
            },
            validity: ValidityGuardRails {
                seconds_before_stale_for_pool: 5,
                too_volatile_ratio: 120, // allows up to ±20%
            },
        }
    }
}

impl OracleGuardRails {
    pub fn max_oracle_twap_percent_divergence(&self) -> u64 {
        self.price_divergence.oracle_twap_percent_divergence.max(PERCENTAGE_PRECISION_U64 / 2)
    }
}

// ordered by "severity"
#[derive(Clone, Copy, PartialEq, Debug, Eq, Default)]
pub enum OracleValidity {
    NonPositive,
    TooVolatile,
    // @dev have code ready to implement but oracle response does not support
    // TooUncertain,
    // InsufficientDataPoints,
    StaleForPool,
    #[default]
    Valid,
}

impl OracleValidity {
    pub fn get_error_code(&self) -> OracleError {
        match self {
            OracleValidity::NonPositive => OracleError::OracleNonPositive,
            OracleValidity::TooVolatile => OracleError::OracleTooVolatile,
            // OracleValidity::TooUncertain => OracleError::OracleTooUncertain,
            // OracleValidity::InsufficientDataPoints => OracleError::OracleInsufficientDataPoints,
            OracleValidity::StaleForPool => OracleError::OracleStaleForPool,
            OracleValidity::Valid => unreachable!(),
        }
    }
}

#[derive(Default, Clone, Copy, Debug)]
pub struct OracleStatus {
    pub price_data: OraclePriceData,
    pub oracle_reserve_price_spread_pct: i64,
    pub price_too_divergent: bool,
    pub oracle_validity: OracleValidity,
}
