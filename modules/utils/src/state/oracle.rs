use crate::temporal::Delay;
use soroban_sdk::contracttype;

use crate::{
    constant::{FIVE_MINUTE, PERCENTAGE_PRECISION_U64, PRICE_PRECISION},
    errors::oracle_error::OracleError,
};

#[contracttype]
#[derive(Default, Clone, Copy, Debug)]
pub struct OraclePriceData {
    pub price: u128,
    pub delay: Delay,
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
    pub too_volatile_ratio: u64,
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
                seconds_before_stale_for_pool: FIVE_MINUTE as u64,
                too_volatile_ratio: PERCENTAGE_PRECISION_U64 / 5, // ±20%
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
#[contracttype]
#[derive(Clone, Copy, PartialEq, Debug, Eq, Default)]
pub enum OracleValidity {
    NonPositive,
    TooVolatile,
    StaleForPool,
    Frozen,
    #[default]
    Valid,
}

impl OracleValidity {
    pub fn get_error_code(&self) -> OracleError {
        match self {
            OracleValidity::NonPositive => OracleError::OracleNonPositive,
            OracleValidity::TooVolatile => OracleError::OracleTooVolatile,
            OracleValidity::StaleForPool => OracleError::OracleStaleForPool,
            OracleValidity::Frozen => unreachable!(),
            OracleValidity::Valid => unreachable!(),
        }
    }
}

#[contracttype]
#[derive(Default, Clone, Copy, Eq, PartialEq, Debug)]
pub struct HistoricalOracleData {
    pub last_price: u128,
    pub last_price_twap: u128,
    pub last_update_ts: u64, // unix_timestamp of last snapshot.
}

impl HistoricalOracleData {
    pub fn default_quote_oracle() -> Self {
        HistoricalOracleData {
            last_price: PRICE_PRECISION,
            last_price_twap: PRICE_PRECISION,
            ..HistoricalOracleData::default()
        }
    }

    pub fn default_price(price: u128) -> Self {
        HistoricalOracleData {
            last_price: price,
            last_price_twap: price,
            ..HistoricalOracleData::default()
        }
    }

    pub fn default_with_current_oracle(price_data: OraclePriceData) -> Self {
        HistoricalOracleData {
            last_price: price_data.price,
            last_price_twap: price_data.price,
            ..HistoricalOracleData::default()
        }
    }
}
