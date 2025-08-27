use soroban_fixed_point_math::SorobanFixedPoint;
use soroban_sdk::contracttype;
use soroban_sdk::Address;
use soroban_sdk::Env;
use soroban_sdk::Symbol;

use crate::constant::FEE_MULTIPLIER;
use crate::state::access::PrivilegedAddresses;
use crate::state::token::AddressAndAmount;
use crate::state::token::TokenInitInfo;

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

    pub fn get_amount_out(
        &self,
        e: &Env,
        in_amount: u128,
        reserve_sell: u128,
        reserve_buy: u128,
    ) -> (u128, u128) {
        if in_amount == 0 {
            return (0, 0);
        }

        // in * reserve_buy / (reserve_sell + in) - fee
        let result = in_amount.fixed_mul_floor(&e, &reserve_buy, &(reserve_sell + in_amount));
        let fee = result.fixed_mul_ceil(&e, &(self.fee_fraction as u128), &FEE_MULTIPLIER);
        (result - fee, fee)
    }
}


impl PoolTier {
    pub fn is_as_safe_as(&self, other: &PoolTier) -> bool {
        // Pool Tier A safest
        self <= other
    }
}
