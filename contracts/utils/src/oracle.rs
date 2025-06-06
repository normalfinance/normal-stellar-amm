use core::cmp::max;

use soroban_sdk::{ contracttype, log, Address, Env };

use crate::{
    constant::{ PERCENTAGE_PRECISION_U64 },
    errors::oracle_error::OracleError,
    math::safe_math::SafeMath,
};

//  ___________  ___  ___  _______    _______   ________
// ("     _   ")|"  \/"  ||   __ "\  /"     "| /"       )
//  )__/  \\__/  \   \  / (. |__) :)(: ______)(:   \___/
//     \\_ /      \\  \/  |:  ____/  \/    |   \___  \
//     |.  |      /   /   (|  /      // ___)_   __/  \\
//     \:  |     /   /   /|__/ \    (:      "| /" \   :)
//      \__|    |___/   (_______)    \_______)(_______/

#[contracttype]
#[derive(Default, Clone, Copy, Debug)]
pub struct OraclePriceData {
    pub price: u128,
    pub delay: u64,
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

// ordered by "severity"
#[derive(Clone, Copy, PartialEq, Debug, Eq, Default)]
pub enum OracleValidity {
    NonPositive,
    TooVolatile,
    TooUncertain,
    InsufficientDataPoints,
    StaleForAMM,
    #[default]
    Valid,
}

#[derive(Clone, Copy, PartialEq, Debug, Eq)]
pub enum NormalAction {
    Rebalance,
    UpdateTwap,
}

#[derive(Default, Clone, Copy, Debug)]
pub struct OracleStatus {
    pub price_data: OraclePriceData,
    pub oracle_reserve_price_spread_pct: i64,
    pub price_too_divergent: bool,
    pub oracle_validity: OracleValidity,
}

//  ____  ____  ___________  __    ___        ________
// ("  _||_ " |("     _   ")|" \  |"  |      /"       )
// |   (  ) : | )__/  \\__/ ||  | ||  |     (:   \___/
// (:  |  | . )    \\_ /    |:  | |:  |      \___  \
//  \\ \__/ //     |.  |    |.  |  \  |___    __/  \\
//  /\\ __ //\     \:  |    /\  |\( \_|:  \  /" \   :)
// (__________)     \__|   (__\_|_)\_______)(_______/

impl Default for OracleGuardRails {
    fn default() -> Self {
        OracleGuardRails {
            price_divergence: PriceDivergenceGuardRails {
                // mark_oracle_percent_divergence: PERCENTAGE_PRECISION_U64 / 10,
                oracle_twap_percent_divergence: PERCENTAGE_PRECISION_U64 / 2,
            },
            validity: ValidityGuardRails {
                slots_before_stale_for_pool: 10, // ~5 seconds
                confidence_interval_max_size: 20_000, // 2% of price
                too_volatile_ratio: 5, // 5x or 80% down
            },
        }
    }
}

impl OracleGuardRails {
    pub fn max_oracle_twap_percent_divergence(&self) -> u64 {
        self.price_divergence.oracle_twap_percent_divergence.max(PERCENTAGE_PRECISION_U64 / 2)
    }
}

impl OracleValidity {
    pub fn get_error_code(&self) -> OracleError {
        match self {
            OracleValidity::NonPositive => OracleError::OracleNonPositive,
            OracleValidity::TooVolatile => OracleError::OracleTooVolatile,
            OracleValidity::TooUncertain => OracleError::OracleTooUncertain,
            OracleValidity::InsufficientDataPoints => OracleError::OracleInsufficientDataPoints,
            OracleValidity::StaleForAMM => OracleError::OracleStaleForAMM,
            OracleValidity::Valid => unreachable!(),
        }
    }
}

pub fn is_oracle_valid_for_action(
    oracle_validity: OracleValidity,
    action: Option<NormalAction>
) -> bool {
    let is_ok = match action {
        Some(action) =>
            match action {
                NormalAction::Rebalance => {
                    matches!(
                        oracle_validity,
                        OracleValidity::Valid |
                            OracleValidity::StaleForAMM |
                            OracleValidity::InsufficientDataPoints
                    )
                }
                NormalAction::UpdateTwap => !matches!(oracle_validity, OracleValidity::NonPositive),
            }
        None => { matches!(oracle_validity, OracleValidity::Valid) }
    };

    is_ok
}

pub fn is_oracle_price_too_divergent(
    price_spread_pct: i64,
    oracle_guard_rails: &PriceDivergenceGuardRails
) -> bool {
    let max_divergence = oracle_guard_rails.oracle_twap_percent_divergence.max(
        PERCENTAGE_PRECISION_U64 / 10
    );
    price_spread_pct.unsigned_abs() > max_divergence
}

pub fn oracle_validity(
    e: &Env,
    pool_address: Address,
    last_oracle_twap: u128,
    oracle_price_data: &OraclePriceData,
    valid_oracle_guard_rails: &ValidityGuardRails,
    log_validity: bool
) -> OracleValidity {
    let OraclePriceData { price: oracle_price, delay: oracle_delay, .. } = *oracle_price_data;

    let is_oracle_price_nonpositive = oracle_price <= 0;

    let is_oracle_price_too_volatile = oracle_price
        .max(last_oracle_twap)
        .safe_div(e, last_oracle_twap.min(oracle_price).max(1))
        .gt(&(valid_oracle_guard_rails.too_volatile_ratio as u128));

    let is_stale_for_pool = oracle_delay.gt(&valid_oracle_guard_rails.slots_before_stale_for_pool);

    let oracle_validity = if is_oracle_price_nonpositive {
        OracleValidity::NonPositive
    } else if is_oracle_price_too_volatile {
        OracleValidity::TooVolatile
    } else if is_stale_for_pool {
        OracleValidity::StaleForAMM
    } else {
        OracleValidity::Valid
    };

    if log_validity {
        if is_oracle_price_nonpositive {
            log!(e, "Invalid {} Oracle: Non-positive (oracle_price <=0)", pool_address);
        }

        if is_oracle_price_too_volatile {
            log!(
                e,
                "Invalid {} Oracle: Too Volatile (last_oracle_price_twap={:?} vs oracle_price={:?})",
                pool_address,
                last_oracle_twap,
                oracle_price
            );
        }

        // if is_conf_too_large {
        //     log!(
        //         e,
        //         "Invalid {} Oracle: Confidence Too Large (is_conf_too_large={:?})",
        //         pool_address,
        //         conf_pct_of_price
        //     );
        // }

        if is_stale_for_pool {
            log!(e, "Invalid {} Oracle: Stale (oracle_delay={:?})", pool_address, oracle_delay);
        }
    }

    oracle_validity
}
