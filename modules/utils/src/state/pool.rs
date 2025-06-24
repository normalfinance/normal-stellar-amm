use soroban_sdk::contracttype;
use soroban_sdk::Address;
use soroban_sdk::Symbol;
use soroban_sdk::Vec;
use soroban_sdk::{ Env };
use soroban_fixed_point_math::SorobanFixedPoint;

use crate::constant::FEE_MULTIPLIER;
use crate::state::access::PrivilegedAddresses;
use crate::state::token::AddressAndAmount;
use crate::state::token::TokenInitInfo;

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pool {
    pub token_b: Address,
    pub base_asset: Symbol, // Oracle address for the base (synthetic) asset (i.e. nBTC)
    pub quote_asset: Symbol, // Oracle address for the quote asset (TokenB) - usually XLM or USDC
    pub tier: PoolTier,
    pub status: PoolStatus,
    pub fee_fraction: u32, // 1 = 0.01%
    // The pool's claim on the insurance fund
    pub insurance_claim: InsuranceClaim,
    // The max liquidity imbalance before price premiums are added and/or the buffer/if is used
    // liquidity imbalance is the difference between quote token and base token value. When it's less than 0,
    // the pool does not have enough liquidity to fill all orders and will apply a price premium to new swaps.
    // precision = QUOTE_PRECISION
    pub liquidity_max_imbalance: u128,
    pub expiry_ts: u64, // The time the market is set to expire. Only set if market is in reduce only mode
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

    pub fn get_max_confidence_interval_multiplier(&self) -> u64 {
        // assuming validity_guard_rails max confidence pct is 2%
        match self.tier {
            PoolTier::A => 1, // 2%
            PoolTier::B => 1, // 2%
            PoolTier::C => 2, // 4%
            PoolTier::Speculative => 10, // 20%
            PoolTier::HighlySpeculative => 50, // 100%
            PoolTier::Isolated => 50, // 100%
        }
    }

    pub fn get_sanitize_clamp_denominator(&self) -> Option<i64> {
        match self.tier {
            PoolTier::A => Some(10_i64), // 10%
            PoolTier::B => Some(5_i64), // 20%
            PoolTier::C => Some(2_i64), // 50%
            PoolTier::Speculative => None, // DEFAULT_MAX_TWAP_UPDATE_PRICE_BAND_DENOMINATOR
            PoolTier::HighlySpeculative => None, // DEFAULT_MAX_TWAP_UPDATE_PRICE_BAND_DENOMINATOR
            PoolTier::Isolated => None, // DEFAULT_MAX_TWAP_UPDATE_PRICE_BAND_DENOMINATOR
        }
    }

    pub fn get_insurance_coverage_multiplier(&self) -> u64 {
        match self.tier {
            PoolTier::A => 10_u64, // 10%
            PoolTier::B => 5_u64, // 20%
            PoolTier::C => 2_u64, // 50%
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
        reserve_buy: u128
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
    // warm up period for initialization, fills are paused
    #[default]
    Initialized,
    // all operations allowed
    Active,
    //
    Frozen,
    // fills only able to reduce liability
    ReduceOnly,
    // market has determined settlement price and positions are expired must be settled
    Settlement,
    // market has no remaining participants
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
    // precision: QUOTE_PRECISION
    pub rev_withdraw_since_last_settle: i128,
    // The max amount of insurance that the pool can use to resolve liquidity deficits
    // precision: QUOTE_PRECISION
    pub quote_max_insurance: u128,
    // The amount of insurance that has been used to resolve liquidity deficits
    // precision: QUOTE_PRECISION
    pub quote_settled_insurance: u128,
    // The last time revenue was settled in/out of the pool
    pub last_revenue_withdraw_ts: u64,
}

// This struct is used to return a query result with the total amount of LP tokens and assets in a specific pool.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PoolResponse {
    // The Pool info
    pub pool: Pool,
    // The asset A in the pool together with asset amounts
    pub asset_a: AddressAndAmount,
    // The asset B in the pool together with asset amounts
    pub asset_b: AddressAndAmount,
    // The total amount of LP tokens currently issued
    pub asset_lp_share: AddressAndAmount,
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
    // The address of the reward token.
    pub reward_token: Address,
}

#[contracttype]
#[derive(Clone)]
pub struct InitializeParams {
    // The address of the admin user.
    pub admin: Address,
    pub privileged_addrs: PrivilegedAddresses,
    // The address of the Router.
    pub router: Address,
    pub assets: (Symbol, Symbol),
    pub lp_token_info: TokenInitInfo,
    // A vector of token addresses.
    pub tokens: Vec<Address>,
    // The fee fraction for the pool.
    pub fee_fraction: u32,
    //
    pub tier: PoolTier,
    //
    pub quote_max_insurance: u128,
}

#[contracttype]
#[derive(Clone)]
pub struct InitializeAllParams {
    pub base: InitializeParams,
    pub reward_config: RewardConfig,
    /// The address of the plane.
    pub plane: Address,
}
