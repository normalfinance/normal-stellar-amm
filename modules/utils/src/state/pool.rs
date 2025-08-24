use soroban_fixed_point_math::SorobanFixedPoint;
use soroban_sdk::contracttype;
use soroban_sdk::Address;
use soroban_sdk::Env;
use soroban_sdk::Symbol;

use crate::constant::FEE_MULTIPLIER;
use crate::state::access::PrivilegedAddresses;
use crate::state::token::AddressAndAmount;
use crate::state::token::TokenInitInfo;

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pool {
    pub token_b: Address,    // the quote token address (usually XLM or USDC).
    pub base_asset: Symbol, // the Oracle Registry asset id for the base (synthetic) asset (i.e. nBTC).
    pub quote_asset: Symbol, // the Oracle Registry asset id for the quote asset (token_b).
    pub tier: PoolTier,
    pub status: PoolStatus,
    pub fee_fraction: u32,               // the swap fee (in basis points).
    pub insurance_claim: InsuranceClaim, // the pool's claim on the insurance fund.
    // The max liquidity imbalance before price premiums are added and/or the buffer/if is used
    // liquidity imbalance is the difference between quote token and base token value. When it's less than 0,
    // the pool does not have enough liquidity to fill all orders and will apply a price premium to new swaps.
    // precision = QUOTE_PRECISION
    pub liquidity_max_imbalance: u128,
    pub expiry_ts: u64, // The time the pool is set to expire. Only set if pool is in reduce only mode
    pub expiry_price: u128, // The frozen price used to settle positions when a pool is set to reduce only mode
}

impl Pool {
    pub fn is_in_settlement(&self, now: u64) -> bool {
        let in_settlement = matches!(self.status, PoolStatus::Settlement | PoolStatus::Delisted);
        let expired = self.expiry_ts != 0 && now >= self.expiry_ts;
        in_settlement || expired
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

#[contracttype]
#[derive(Default, Clone, PartialEq, Eq, Debug)]
pub enum PoolStatus {
    // warm up period for initialization, swaps are paused
    #[default]
    Initialized,
    // all operations allowed
    Active,
    //
    Frozen,
    // swaps only able to reduce liability (sell)
    ReduceOnly,
    // pool has determined settlement price and positions are expired must be settled
    Settlement,
    // pool has no remaining participants
    Delisted,
}

#[contracttype]
#[derive(Clone, Copy, PartialEq, Debug, Eq, PartialOrd, Ord, Default)]
pub enum PoolTier {
    // max insurance capped at A level
    A,
    // max insurance capped at B level
    B,
    // max insurance capped at C level
    C,
    // no insurance
    Speculative,
    // no insurance, another tranches below
    #[default]
    HighlySpeculative,
    // no insurance, only single position allowed
    Isolated,
}

impl PoolTier {
    pub fn is_as_safe_as(&self, other: &PoolTier) -> bool {
        // Pool Tier A safest
        self <= other
    }
}

#[contracttype]
#[derive(Default, Clone, Eq, PartialEq, Debug)]
pub struct InsuranceClaim {
    // The amount of revenue last settled
    // Positive if funds left the pool,
    // negative if funds were pulled into the pool
    pub rev_withdraw_since_last_settle: i128,
    pub quote_max_insurance: u128, // The max amount of insurance that the pool can use to resolve liquidity deficits
    pub quote_settled_insurance: u128, // The amount of insurance that has been used to resolve liquidity deficits
    pub last_revenue_withdraw_ts: u64, // The last time revenue was settled in/out of the pool
}

// This struct is used to return a query result with the total amount of LP tokens and assets in a specific pool.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PoolResponse {
    pub pool: Pool,
    pub token_a: AddressAndAmount,
    pub token_b: AddressAndAmount,
    pub token_share: AddressAndAmount,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PoolInfo {
    pub pool_address: Address,
    pub pool_response: PoolResponse,
}

#[contracttype]
#[derive(Clone)]
pub struct RewardConfig {
    pub reward_token: Address,
}

#[contracttype]
#[derive(Clone)]
pub struct InitializeParams {
    pub admin: Address,
    pub privileged_addrs: PrivilegedAddresses,
    pub router: Address,
    pub oracle_registry: Address,
    pub assets: (Symbol, Symbol),
    pub synthetic_sac_address: Address,
    // pub synthetic_token_info: TokenInitInfo,
    pub lp_token_info: TokenInitInfo,
    pub token_b: Address,
    pub fee_fraction: u32,
    pub tier: PoolTier,
    pub quote_max_insurance: u128,
}

#[contracttype]
#[derive(Clone)]
pub struct InitializeAllParams {
    pub base: InitializeParams,
    pub reward_config: RewardConfig,
    pub plane: Address,
}

#[contracttype]
#[derive(Default, Clone, PartialEq, Eq, Debug)]
pub enum SwapDirection {
    #[default]
    Buy,
    Sell,
}

#[contracttype]
#[derive(Clone)]
pub struct PoolInfoForCoverage {
    pub asset: Symbol,
    pub target_asset_price: u128,
    pub total_synthetic_supply: u128,
    pub token_b_reserve: u128,
}
