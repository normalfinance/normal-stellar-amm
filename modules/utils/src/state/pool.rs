use soroban_sdk::contracttype;
use soroban_sdk::Address;
use soroban_sdk::Symbol;

use crate::state::access::PrivilegedAddresses;
use crate::state::token::AddressAndAmount;
use crate::state::token::TokenInitInfo;

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
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct InsuranceClaim {
    // The amount of revenue last settled
    // Positive if funds left the pool,
    // negative if funds were pulled into the pool
    pub rev_withdraw_since_last_settle: i128,
    pub max_insurance: u128, // The max amount of insurance that the pool can use to resolve liquidity deficits
    pub settled_insurance: u128, // The amount of insurance that has been used to resolve liquidity deficits
    pub last_revenue_withdraw_ts: u64, // The last time revenue was settled in/out of the pool
}

impl Default for InsuranceClaim {
    fn default() -> Self {
        InsuranceClaim {
            rev_withdraw_since_last_settle: 0,
            max_insurance: 0,
            settled_insurance: 0,
            last_revenue_withdraw_ts: 0,
        }
    }
}

impl InsuranceClaim {
    pub fn new(max_insurance: u128) -> Self {
        InsuranceClaim {
            rev_withdraw_since_last_settle: 0,
            max_insurance: max_insurance,
            settled_insurance: 0,
            last_revenue_withdraw_ts: 0,
        }
    }
}

#[contracttype]
#[derive(Clone)]
pub struct PoolConfig {
    pub admin: Address,
    pub privileged_addrs: PrivilegedAddresses,
    pub router: Address,
    pub oracle_registry: Address,
    pub token_a_sac_address: Address,
    pub token_b: Address,
    pub lp_token_info: TokenInitInfo,
    pub assets: (Symbol, Symbol),
    pub status: PoolStatus,
    pub tier: PoolTier,
    pub fee_fraction: u32,
    pub protocol_fee_fraction: u32,
    pub max_insurance: u128,
}

#[contracttype]
#[derive(Clone)]
pub struct PoolDetails {
    pub assets: (Symbol, Symbol),
    pub status: PoolStatus,
    pub tier: PoolTier,
    pub fee_fraction: u32,
    pub protocol_fee_fraction: u32,
    pub insurance: InsuranceClaim,
}

// This struct is used to return a query result with the total amount of LP tokens and assets in a specific pool.
#[contracttype]
#[derive(Clone)]
pub struct PoolResponse {
    pub pool: PoolDetails,
    pub token_a: AddressAndAmount,
    pub token_b: AddressAndAmount,
    pub token_share: AddressAndAmount,
}

#[contracttype]
#[derive(Clone)]
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
pub struct InitializeAllParams {
    pub config: PoolConfig,
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
