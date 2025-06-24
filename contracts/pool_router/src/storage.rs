use crate::errors::PoolRouterError;
use paste::paste;
use soroban_sdk::{
    contracterror,
    contracttype,
    panic_with_error,
    Address,
    BytesN,
    Env,
    Map,
    Symbol,
    Vec,
    U256,
};
use utils::bump::{ bump_instance, bump_persistent, bump_temporary };
use utils::errors::storage_errors::StorageError;
use utils::{
    generate_instance_storage_getter,
    generate_instance_storage_getter_and_setter,
    generate_instance_storage_setter,
};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GlobalRewardsConfig {
    pub tps: u128,
    pub expired_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolRewardInfo {
    pub processed: bool,
    pub total_liquidity: U256,
}

#[derive(Clone)]
#[contracttype]
enum DataKey {
    PoolsVec,
    Pools(Symbol), // Map of asset (i.e. "BTC") > Pool
    TokenHash,
    PoolHash,
    PoolPlane,
    LiquidityCalculator,
    OracleRegistry, // the address of the Oracle Registry contract.

    // Temporary storage
    RewardsConfig, // Global reward config
    RewardTokensList, // Tokens for reward - Map of oracle_id > PoolRewardInfo
    RewardTokensPoolsLiquidity(Symbol), // Per pool liquidity - Map of pool salt > (U256, bool)
}

#[contracterror]
#[derive(Copy, Clone)]
#[repr(u32)]
pub enum PoolError {
    #[doc = "PoolError"]
    PoolAlreadyExists = 401,
    PoolNotFound = 404,
}

generate_instance_storage_getter_and_setter!(
    pool_hash,
    DataKey::PoolHash,
    BytesN<32>
);
generate_instance_storage_getter_and_setter!(token_hash, DataKey::TokenHash, BytesN<32>);
generate_instance_storage_getter_and_setter!(pool_plane, DataKey::PoolPlane, Address);
generate_instance_storage_getter_and_setter!(
    liquidity_calculator,
    DataKey::LiquidityCalculator,
    Address
);
generate_instance_storage_getter_and_setter!(oracle_registry, DataKey::OracleRegistry, Address);

// Pool
pub fn get_pool(e: &Env, asset: &Symbol) -> Address {
    let result = get_pool_base(e, asset.clone());
    match result {
        Some(value) => { value }
        None => panic_with_error!(&e, PoolError::PoolNotFound),
    }
}
pub fn get_pool_base(e: &Env, asset: Symbol) -> Option<Address> {
    let key = DataKey::Pools(asset);
    match e.storage().persistent().get(&key) {
        Some(value) => {
            bump_persistent(e, &key);
            value
        }
        None => None,
    }
}

pub fn put_pool(e: &Env, asset: Symbol, pool_address: &Address) {
    let key = DataKey::Pools(asset);
    e.storage().persistent().set(&key, pool_address);
    bump_persistent(e, &key);
}

pub fn remove_pool(e: &Env, asset: Symbol) {
    let key = DataKey::Pools(asset);
    e.storage().persistent().remove(&key);
}

pub fn get_pools_vec(e: &Env) -> Vec<Address> {
    let key = DataKey::PoolsVec;
    match e.storage().temporary().get(&key) {
        Some(v) => {
            bump_temporary(e, &key);
            v
        }
        None => Vec::new(e),
    }
}

pub fn set_pools_vec(e: &Env, pools: &Vec<Address>) {
    let key = DataKey::PoolsVec;
    e.storage().persistent().set(&key, pools);
    bump_persistent(e, &key);
}

// Rewards
pub fn get_rewards_config(e: &Env) -> GlobalRewardsConfig {
    match e.storage().temporary().get(&DataKey::RewardsConfig) {
        Some(v) => {
            bump_temporary(e, &DataKey::RewardsConfig);
            v
        }
        None =>
            GlobalRewardsConfig {
                tps: 0,
                expired_at: 0,
            },
    }
}

pub fn set_rewards_config(e: &Env, value: &GlobalRewardsConfig) {
    let key = DataKey::RewardsConfig;
    e.storage().temporary().set(&key, value);
    bump_temporary(e, &key);
}

pub fn get_reward_tokens(e: &Env) -> Map<Symbol, PoolRewardInfo> {
    let key = DataKey::RewardTokensList;
    match e.storage().temporary().get(&key) {
        Some(v) => {
            bump_temporary(e, &key);
            v
        }
        None => panic_with_error!(&e, PoolRouterError::RewardsNotConfigured),
    }
}

pub fn set_reward_tokens(e: &Env, value: &Map<Symbol, PoolRewardInfo>) {
    let key = DataKey::RewardTokensList;
    e.storage().temporary().set(&key, value);
    bump_temporary(e, &key);
}

pub fn get_reward_tokens_detailed(e: &Env, asset: Symbol) -> (U256, bool) {
    let key = DataKey::RewardTokensPoolsLiquidity(asset);
    match e.storage().temporary().get(&key) {
        Some(v) => {
            bump_temporary(e, &key);
            v
        }
        None => panic_with_error!(&e, PoolRouterError::LiquidityNotFilled),
    }
}

pub fn set_reward_tokens_detailed(e: &Env, asset: Symbol, value: &(U256, bool)) {
    let key = DataKey::RewardTokensPoolsLiquidity(asset);
    let result = e.storage().temporary().set(&key, value);
    bump_temporary(e, &key);
    result
}
