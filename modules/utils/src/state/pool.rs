use soroban_fixed_point_math::SorobanFixedPoint;
use soroban_sdk::{contracttype, Address, Env, Symbol};
use normal_rust_types::{Pool, PoolStatus, PoolTier};

use crate::constant::FEE_MULTIPLIER;

impl Pool {
    pub fn is_in_settlement(&self, now: u64) -> bool {
        let in_settlement = matches!(self.status, PoolStatus::Settlement | PoolStatus::Delisted);
        // let expired = self.expiry_ts != 0 && now >= self.expiry_ts;
        // in_settlement || expired
        in_settlement
    }

    pub fn is_reduce_only(&self) -> bool {
        self.status == PoolStatus::ReduceOnly
    }

    pub fn get_sanitize_clamp_denominator(&self) -> Option<i64> {
        match self.tier {
            PoolTier::A => Some(10_i64),         // 10%
            PoolTier::B => Some(5_i64),          // 20%
            PoolTier::C => Some(2_i64),          // 50%
            PoolTier::Speculative => None,       // DEFAULT_MAX_TWAP_UPDATE_PRICE_BAND_DENOMINATOR
            PoolTier::HighlySpeculative => None, // DEFAULT_MAX_TWAP_UPDATE_PRICE_BAND_DENOMINATOR
            PoolTier::Isolated => None,          // DEFAULT_MAX_TWAP_UPDATE_PRICE_BAND_DENOMINATOR
        }
    }

    pub fn get_insurance_coverage_multiplier(&self) -> u64 {
        match self.tier {
            PoolTier::A => 10_u64, // 10%
            PoolTier::B => 5_u64,  // 20%
            PoolTier::C => 2_u64,  // 50%
            PoolTier::Speculative => 10_u64,
            PoolTier::HighlySpeculative => 10_u64,
            PoolTier::Isolated => 10_u64,
        }
    }
}


impl PoolTier {
    pub fn is_as_safe_as(&self, other: &PoolTier) -> bool {
        // Pool Tier A safest
        self <= other
    }
}
