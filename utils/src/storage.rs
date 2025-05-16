use sep_40_oracle::Asset;
use soroban_sdk::{contracttype, Address, BytesN, String, Vec};

#[macro_export]
macro_rules! generate_instance_storage_setter {
    ($attr_name:ident, $key:expr, $data_type:ty) => {
        paste! {
            pub fn [<set_ $attr_name>](e: &Env, $attr_name: &$data_type) {
                bump_instance(e);
                e.storage()
                    .instance()
                    .set(&$key, $attr_name)
            }
        }
    };
}

#[macro_export]
macro_rules! generate_instance_storage_getter {
    ($attr_name:ident, $key:expr, $data_type:ty) => {
        paste! {
            pub fn [<get_ $attr_name>](e: &Env) -> $data_type {
                bump_instance(e);
                let value_result = e.storage().instance().get(&$key);
                match value_result {
                    Some(value) => value,
                    None => {
                        panic_with_error!(e, StorageError::ValueNotInitialized)
                    }
                }
            }
        }
    };
}

#[macro_export]
macro_rules! generate_instance_storage_getter_with_default {
    ($attr_name:ident, $key:expr, $data_type:ty, $default:expr) => {
        paste! {
            pub fn [<get_ $attr_name>](e: &Env) -> $data_type {
                bump_instance(e);
                e.storage().instance().get(&$key).unwrap_or($default)
            }
        }
    };
}

#[macro_export]
macro_rules! generate_instance_storage_getter_and_setter {
    ($attr_name:ident, $key:expr, $data_type:ty) => {
        generate_instance_storage_getter!($attr_name, $key, $data_type);
        generate_instance_storage_setter!($attr_name, $key, $data_type);
    };
}

#[macro_export]
macro_rules! generate_instance_storage_getter_and_setter_with_default {
    ($attr_name:ident, $key:expr, $data_type:ty, $default:expr) => {
        generate_instance_storage_getter_with_default!($attr_name, $key, $data_type, $default);
        generate_instance_storage_setter!($attr_name, $key, $data_type);
    };
}

#[contracttype]
#[derive(Clone)]
pub struct TokenInitInfo {
    pub token_wasm_hash: BytesN<32>,
    pub name: String,
    pub symbol: String,
}

#[contracttype]
#[derive(Clone)]
pub struct PrivilegedAddresses {
    pub emergency_admin: Address,
    pub rewards_admin: Address,
    pub operations_admin: Address,
    pub pause_admin: Address,
    pub emergency_pause_admins: Vec<Address>,
}

#[contracttype]
#[derive(Clone)]
pub struct OraclePair {
    pub base_oracle: Address,
    pub quote_oracle: Address,
}

#[contracttype]
#[derive(Clone)]
pub struct RewardConfig {
    /// The address of the reward token.
    pub reward_token: Address,
    /// The address of the reward boost token.
    pub reward_boost_token: Address,
    /// The address of the reward boost feed.
    pub reward_boost_feed: Address,
}

#[contracttype]
#[derive(Clone)]
pub struct InitializeParams {
    pub admin: Address,
    pub privileged_addrs: PrivilegedAddresses,
    pub router: Address,
    pub oracles: OraclePair,
    pub target_asset: Asset,
    pub lp_token_info: TokenInitInfo,
    pub tokens: Vec<Address>,
    pub fee_fraction: u32,
}

#[contracttype]
#[derive(Clone)]
pub struct InitializeAllParams {
    pub base: InitializeParams,
    pub reward_config: RewardConfig,
    pub plane: Address,
}

//  Queries

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AddressAndAmount {
    /// Address of the asset
    pub address: Address,
    /// The total amount of those tokens in the pool
    pub amount: u128,
}

/// This struct is used to return a query result with the total amount of LP tokens and assets in a specific pool.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PoolResponse {
    /// The asset A in the pool together with asset amounts
    pub asset_a: AddressAndAmount,
    /// The asset B in the pool together with asset amounts
    pub asset_b: AddressAndAmount,
    /// The total amount of LP tokens currently issued
    pub asset_lp_share: AddressAndAmount,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiquidityPoolInfo {
    pub pool_address: Address,
    pub pool_response: PoolResponse,
    pub total_fee_bps: u32,
}
