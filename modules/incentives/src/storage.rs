use soroban_sdk::{contracttype, panic_with_error, Address, Env, Map, Vec};
use utils::{
    bump::{bump_instance, bump_persistent},
    errors::storage_errors::StorageError,
};

// ------------------------------------
// Data Structures
// ------------------------------------

// Incentives configuration for a specific pool.
#[derive(Clone)]
#[contracttype]
pub struct PoolIncentiveConfig {
    pub reward_tps: u128,
    pub reward_expired_at: u64,
}

// Mutable pool incentive data that evolves over time.
#[derive(Clone)]
#[contracttype]
pub struct PoolIncentiveData {
    // rewards
    pub block: u64,
    pub accumulated_rewards: u128,
    pub claimed_rewards: u128,
    pub rewards_last_time: u64,
    // lp fees - Tracks how much of token_b has been collected as fees per unit of LP token, cumulatively.
    pub fee_growth_per_lp: u128,
}

// Per-user incentive data.
#[derive(Clone)]
#[contracttype]
pub struct UserIncentiveData {
    // rewards
    pub pool_accumulated_rewards: u128,
    pub rewards_to_claim: u128,
    pub last_block: u64,
    // lp fees
    pub fee_checkpoint: u128,
}

#[derive(Clone)]
#[contracttype]
enum DataKey {
    // Pool-level data
    PoolIncentiveConfig,
    PoolIncentiveData,

    // User-level data
    UserIncentiveData(Address),

    // Reward invariants
    RewardInvDataV2(u32, u64),

    // Tokens
    LPToken,
    RewardToken,

    // Working balances
    WorkingBalance(Address),
    WorkingSupply,
}

// ------------------------------------
// Core Storage Struct
// ------------------------------------

// Storage struct contains the environment and a local cache (`inv_cache`)
// to avoid repeated loading for reward invariants.
pub struct Storage {
    env: Env,
    inv_cache: Map<DataKey, Vec<u128>>,
}

impl Storage {
    pub fn new(e: &Env) -> Storage {
        Storage {
            env: e.clone(),
            inv_cache: Map::new(e),
        }
    }
}

// ------------------------------------
// Sub-trait: Working Balances
// ------------------------------------

pub trait WorkingBalancesStorageTrait {
    fn get_working_balance(&self, user: &Address) -> u128;
    fn has_working_balance(&self, user: &Address) -> bool;
    fn set_working_balance(&self, user: &Address, value: u128);

    fn get_working_supply(&self) -> u128;
    fn set_working_supply(&self, value: u128);
    fn has_working_supply(&self) -> bool;
}

impl WorkingBalancesStorageTrait for Storage {
    fn get_working_balance(&self, user: &Address) -> u128 {
        self.env
            .storage()
            .persistent()
            .get(&DataKey::WorkingBalance(user.clone()))
            .unwrap()
    }

    fn has_working_balance(&self, user: &Address) -> bool {
        self.env
            .storage()
            .persistent()
            .has(&DataKey::WorkingBalance(user.clone()))
    }

    fn set_working_balance(&self, user: &Address, value: u128) {
        let key = DataKey::WorkingBalance(user.clone());
        self.env.storage().persistent().set(&key, &value);
        bump_persistent(&self.env, &key);
    }

    fn get_working_supply(&self) -> u128 {
        self.env
            .storage()
            .instance()
            .get(&DataKey::WorkingSupply)
            .unwrap()
    }

    fn set_working_supply(&self, value: u128) {
        bump_instance(&self.env);
        self.env
            .storage()
            .instance()
            .set(&DataKey::WorkingSupply, &value);
    }

    fn has_working_supply(&self) -> bool {
        self.env.storage().instance().has(&DataKey::WorkingSupply)
    }
}

// ------------------------------------
// Sub-trait: Pool Incentives
// ------------------------------------

pub trait PoolIncentivesStorageTrait {
    fn get_pool_incentive_config(&self) -> PoolIncentiveConfig;
    fn set_pool_incentive_config(&self, config: &PoolIncentiveConfig);

    fn get_pool_incentive_data(&self) -> PoolIncentiveData;
    fn set_pool_incentive_data(&self, data: &PoolIncentiveData);
}

impl PoolIncentivesStorageTrait for Storage {
    fn get_pool_incentive_config(&self) -> PoolIncentiveConfig {
        match self
            .env
            .storage()
            .instance()
            .get(&DataKey::PoolIncentiveConfig)
        {
            Some(v) => v,
            None => PoolIncentiveConfig {
                reward_tps: 0,
                reward_expired_at: 0,
            },
        }
    }

    fn set_pool_incentive_config(&self, config: &PoolIncentiveConfig) {
        self.env
            .storage()
            .instance()
            .set(&DataKey::PoolIncentiveConfig, config);
    }

    fn get_pool_incentive_data(&self) -> PoolIncentiveData {
        match self
            .env
            .storage()
            .instance()
            .get(&DataKey::PoolIncentiveData)
        {
            Some(v) => v,
            None => PoolIncentiveData {
                block: 0,
                accumulated_rewards: 0,
                claimed_rewards: 0,
                rewards_last_time: 0,
                fee_growth_a_per_lp: 0,
                fee_growth_b_per_lp: 0,
            },
        }
    }

    fn set_pool_incentive_data(&self, data: &PoolIncentiveData) {
        self.env
            .storage()
            .instance()
            .set(&DataKey::PoolIncentiveData, data);
    }
}

// ------------------------------------
// Sub-trait: User Incentives
// ------------------------------------

pub trait UserIncentivesStorageTrait {
    fn get_user_incentive_data(&self, user: &Address) -> Option<UserIncentiveData>;
    fn set_user_incentive_data(&self, user: &Address, config: &UserIncentiveData);
    fn bump_user_incentive_data(&self, user: &Address);
}

impl UserIncentivesStorageTrait for Storage {
    fn get_user_incentive_data(&self, user: &Address) -> Option<UserIncentiveData> {
        match self
            .env
            .storage()
            .persistent()
            .get(&DataKey::UserIncentiveData(user.clone()))
        {
            Some(data) => data,
            None => None,
        }
    }

    fn set_user_incentive_data(&self, user: &Address, config: &UserIncentiveData) {
        self.env
            .storage()
            .persistent()
            .set(&DataKey::UserIncentiveData(user.clone()), config);
    }

    fn bump_user_incentive_data(&self, user: &Address) {
        bump_persistent(&self.env, &DataKey::UserIncentiveData(user.clone()))
    }
}

// ------------------------------------
// Sub-trait: Reward Invariants
// ------------------------------------

pub trait RewardInvDataStorageTrait {
    fn get_reward_inv_data(&mut self, pow: u32, page_number: u64) -> Vec<u128>;
    fn set_reward_inv_data(&mut self, pow: u32, page_number: u64, value: Vec<u128>);
}

impl RewardInvDataStorageTrait for Storage {
    fn get_reward_inv_data(&mut self, pow: u32, page_number: u64) -> Vec<u128> {
        let key = DataKey::RewardInvDataV2(pow, page_number);
        if let Some(cached) = self.inv_cache.get(key.clone()) {
            return cached;
        }

        let value = match self.env.storage().persistent().get::<_, Vec<u128>>(&key) {
            Some(v) => v,
            None => {
                return Vec::new(&self.env);
            }
        };

        self.inv_cache.set(key, value.clone());
        value
    }

    fn set_reward_inv_data(&mut self, pow: u32, page_number: u64, value: Vec<u128>) {
        let key = DataKey::RewardInvDataV2(pow, page_number);
        self.inv_cache.set(key.clone(), value.clone());
        self.env.storage().persistent().set(&key, &value);
        bump_persistent(&self.env, &key);
    }
}

// ------------------------------------
// Sub-trait: LP Token
// ------------------------------------

pub trait LPTokenStorageTrait {
    fn get_lp_token(&self) -> Address;
    fn put_lp_token(&self, contract: Address);
    fn has_lp_token(&self) -> bool;
}

impl LPTokenStorageTrait for Storage {
    fn get_lp_token(&self) -> Address {
        match self.env.storage().instance().get(&DataKey::LPToken) {
            Some(v) => v,
            None => panic_with_error!(self.env, StorageError::ValueNotInitialized),
        }
    }

    fn put_lp_token(&self, contract: Address) {
        self.env
            .storage()
            .instance()
            .set(&DataKey::LPToken, &contract);
    }

    fn has_lp_token(&self) -> bool {
        self.env.storage().instance().has(&DataKey::LPToken)
    }
}

// ------------------------------------
// Sub-trait: Reward Token
// ------------------------------------

pub trait RewardTokenStorageTrait {
    fn get_reward_token(&self) -> Address;
    fn put_reward_token(&self, contract: Address);
    fn has_reward_token(&self) -> bool;
}

impl RewardTokenStorageTrait for Storage {
    fn get_reward_token(&self) -> Address {
        match self.env.storage().instance().get(&DataKey::RewardToken) {
            Some(v) => v,
            None => panic_with_error!(self.env, StorageError::ValueNotInitialized),
        }
    }

    fn put_reward_token(&self, contract: Address) {
        self.env
            .storage()
            .instance()
            .set(&DataKey::RewardToken, &contract);
    }

    fn has_reward_token(&self) -> bool {
        self.env.storage().instance().has(&DataKey::RewardToken)
    }
}
