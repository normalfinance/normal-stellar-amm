use crate::temporal::Delay;
use soroban_sdk::{contracttype, Address};

use crate::{
    constant::{FIVE_MINUTE, PERCENTAGE_PRECISION_U64, PRICE_PRECISION},
    errors::oracle_error::OracleError,
};


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

impl HistoricalOracleData {
    pub fn default_quote_oracle() -> Self {
        HistoricalOracleData {
            last_oracle_price: PRICE_PRECISION,
            last_oracle_price_twap: PRICE_PRECISION,
            ..HistoricalOracleData::default()
        }
    }

    pub fn default_price(price: u128) -> Self {
        HistoricalOracleData {
            last_oracle_price: price,
            last_oracle_price_twap: price,
            ..HistoricalOracleData::default()
        }
    }

    pub fn default_with_current_oracle(oracle_price_data: OraclePriceData) -> Self {
        HistoricalOracleData {
            last_oracle_price: oracle_price_data.price,
            last_oracle_price_twap: oracle_price_data.price,
            ..HistoricalOracleData::default()
        }
    }
}
