use crate::errors::PoolRouterError;
use crate::pool_utils::get_tokens_salt;
use paste::paste;
use soroban_sdk::{
    contracterror,
    contracttype,
    panic_with_error,
    Address,
    BytesN,
    Env,
    Map,
    Vec,
    U256,
};
use utils::bump::{ bump_instance, bump_persistent, bump_temporary };
use utils::constant::MAX_POOLS_FOR_PAIR;
use utils::errors::storage_errors::StorageError;
use utils::{
    generate_instance_storage_getter,
    generate_instance_storage_getter_and_setter,
    generate_instance_storage_getter_and_setter_with_default,
    generate_instance_storage_getter_with_default,
    generate_instance_storage_setter,
};

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum PoolType {
    MissingPool = 0,
    ConstantProduct = 1,
    // StableSwap = 2,
    Custom = 3,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolData {
    pub pool_type: PoolType,
    pub address: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GlobalRewardsConfig {
    pub tps: u128,
    pub expired_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolRewardInfo {
    pub voting_share: u32,
    pub processed: bool,
    pub total_liquidity: U256,
}

#[derive(Clone)]
#[contracttype]
enum DataKey {
    TokensSet(u128),
    TokensSetCounter,
    TokensSetPools(BytesN<32>),
    PoolsVec,
    TokenHash,
    ConstantPoolHash,
    PoolCounter,
    PoolPlane,
    LiquidityCalculator,

    // Temporary storage
    RewardsConfig, // Global reward config
    RewardTokensList, // Tokens for reward
    RewardTokensPoolsLiquidity(BytesN<32>), // Per pool liquidity
}

#[contracterror]
#[derive(Copy, Clone)]
#[repr(u32)]
pub enum PoolError {
    #[doc = "PoolError: PoolAlreadyExists"]
    PoolAlreadyExists = 401,
    PoolNotFound = 404,
}

fn get_pools(e: &Env, salt: BytesN<32>) -> Map<BytesN<32>, PoolData> {
    let key = DataKey::TokensSetPools(salt);
    match e.storage().persistent().get(&key) {
        Some(value) => {
            bump_persistent(e, &key);
            value
        }
        None => Map::new(e),
    }
}

pub fn get_pools_vec(e: &Env) -> Vec<Address> {
    let key = DataKey::PoolsVec;
    match e.storage().persistent().get(&key) {
        Some(value) => {
            bump_persistent(e, &key);
            value
        }
        None => Vec::new(e),
    }
}

fn put_pools_vec(e: &Env, pools: &Vec<Address>) {
    let key = DataKey::PoolsVec;
    e.storage().persistent().set(&key, pools);
    bump_persistent(e, &key);
}

generate_instance_storage_getter_and_setter!(
    constant_product_pool_hash,
    DataKey::ConstantPoolHash,
    BytesN<32>
);
generate_instance_storage_getter_and_setter!(token_hash, DataKey::TokenHash, BytesN<32>);
generate_instance_storage_getter_and_setter_with_default!(
    pool_counter,
    DataKey::PoolCounter,
    u128,
    0
);
generate_instance_storage_getter_and_setter_with_default!(
    tokens_set_count,
    DataKey::TokensSetCounter,
    u128,
    0
);
generate_instance_storage_getter_and_setter!(pool_plane, DataKey::PoolPlane, Address);
generate_instance_storage_getter_and_setter!(
    liquidity_calculator,
    DataKey::LiquidityCalculator,
    Address
);

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

pub fn get_reward_tokens(e: &Env) -> Map<Vec<Address>, PoolRewardInfo> {
    let key = DataKey::RewardTokensList;
    match e.storage().temporary().get(&key) {
        Some(v) => {
            bump_temporary(e, &key);
            v
        }
        None => panic_with_error!(&e, PoolRouterError::RewardsNotConfigured),
    }
}

pub fn set_reward_tokens(e: &Env, value: &Map<Vec<Address>, PoolRewardInfo>) {
    let key = DataKey::RewardTokensList;
    e.storage().temporary().set(&key, value);
    bump_temporary(e, &key);
}

pub fn get_reward_tokens_detailed(e: &Env, salt: BytesN<32>) -> Map<BytesN<32>, (U256, bool)> {
    let key = DataKey::RewardTokensPoolsLiquidity(salt);
    match e.storage().temporary().get(&key) {
        Some(v) => {
            bump_temporary(e, &key);
            v
        }
        None => panic_with_error!(&e, PoolRouterError::LiquidityNotFilled),
    }
}

pub fn set_reward_tokens_detailed(
    e: &Env,
    salt: BytesN<32>,
    value: &Map<BytesN<32>, (U256, bool)>
) {
    let key = DataKey::RewardTokensPoolsLiquidity(salt);
    let result = e.storage().temporary().set(&key, value);
    bump_temporary(e, &key);
    result
}

pub fn get_pools_plain(e: &Env, salt: BytesN<32>) -> Map<BytesN<32>, Address> {
    let pools = get_pools(e, salt);
    let mut pools_plain = Map::new(e);
    for (key, value) in pools {
        pools_plain.set(key, value.address);
    }
    pools_plain
}

pub fn put_pools(e: &Env, salt: BytesN<32>, pools: &Map<BytesN<32>, PoolData>) {
    let key = DataKey::TokensSetPools(salt);
    e.storage().persistent().set(&key, pools);
    bump_persistent(e, &key);
}

pub fn has_pool(e: &Env, salt: BytesN<32>, pool_index: BytesN<32>) -> bool {
    get_pools(e, salt).contains_key(pool_index)
}

pub fn get_pool(e: &Env, tokens: &Vec<Address>, pool_index: BytesN<32>) -> Address {
    let salt = get_tokens_salt(e, tokens);
    let pools = get_pools(e, salt);
    match pools.get(pool_index) {
        Some(data) => data.address,
        None => panic_with_error!(&e, PoolError::PoolNotFound),
    }
}

pub fn add_pool(
    e: &Env,
    salt: BytesN<32>,
    pool_index: BytesN<32>,
    pool_type: PoolType,
    pool_address: Address
) {
    let mut pools = get_pools(e, salt.clone());
    pools.set(pool_index, PoolData {
        pool_type,
        address: pool_address.clone(),
    });

    if pools.len() > MAX_POOLS_FOR_PAIR {
        panic_with_error!(&e, PoolRouterError::PoolsOverMax);
    }
    put_pools(e, salt, &pools);

    let mut pools_vec = get_pools_vec(&e);
    pools_vec.push_back(pool_address.clone());
    put_pools_vec(&e, &pools_vec);
}

// remember unique tokens set
pub fn add_tokens_set(e: &Env, tokens: &Vec<Address>) {
    let salt = get_tokens_salt(e, &tokens);
    let pools = get_pools(e, salt);
    if pools.len() > 0 {
        return;
    }

    let tokens_set_count = get_tokens_set_count(e);
    put_tokens_set(e, tokens_set_count, &tokens);
    set_tokens_set_count(e, &(tokens_set_count + 1));
}

pub fn remove_pool(e: &Env, salt: BytesN<32>, pool_index: BytesN<32>) {
    let mut pools = get_pools(e, salt.clone());
    pools.remove(pool_index);
    put_pools(e, salt, &pools);
}

pub fn get_pool_next_counter(e: &Env) -> u128 {
    let value = get_pool_counter(e);
    set_pool_counter(e, &(value + 1));
    value
}

pub fn get_tokens_set(e: &Env, index: u128) -> Vec<Address> {
    let key = DataKey::TokensSet(index);
    match e.storage().persistent().get(&key) {
        Some(v) => {
            bump_persistent(e, &key);
            v
        }
        None => panic_with_error!(&e, StorageError::ValueNotInitialized),
    }
}

pub fn put_tokens_set(e: &Env, index: u128, tokens: &Vec<Address>) {
    let key = DataKey::TokensSet(index);
    e.storage().persistent().set(&key, tokens);
    bump_persistent(e, &key);
}
