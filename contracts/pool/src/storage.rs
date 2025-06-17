use crate::pool::Pool;
use paste::paste;
use soroban_sdk::{ contracttype, panic_with_error, Address, BytesN, Env };
pub use utils::bump::bump_instance;
use utils::errors::storage_errors::StorageError;
use utils::{
    generate_instance_storage_getter_and_setter_with_default,
    generate_instance_storage_getter_with_default,
    generate_instance_storage_getter_and_setter,
    generate_instance_storage_getter,
    generate_instance_storage_setter,
};

#[derive(Clone)]
#[contracttype]
enum DataKey {
    ReserveA, // total token_a amount in the pool (x in the constant product formula)
    ReserveB, // total token_b amount in the pool (y in the constant product formula)

    Pool, // struct containing infrequently updated pool data
    Plane,
    Router, // the Pool Router contract address
    OracleRegistry, // the Oracle Registry contract address (for getting oracle prices)

    Volume24h, // estimated total of volume in market
    LastTradeTs, // the blockchain unix timestamp at the time of the last trade
    LastUpdateTs, // the last blockchain slot the amm was updated
    LastOracleValid, // tracks whether the oracle was considered valid at the last AMM update

    IsKilledSwap,
    IsKilledDeposit,
    IsKilledWithdraw,
    IsKilledClaim,

    TokenFutureWASM,
}

generate_instance_storage_getter_and_setter!(plane, DataKey::Plane, Address);

generate_instance_storage_getter_and_setter_with_default!(
    is_killed_swap,
    DataKey::IsKilledSwap,
    bool,
    false
);
generate_instance_storage_getter_and_setter_with_default!(
    is_killed_deposit,
    DataKey::IsKilledDeposit,
    bool,
    false
);
generate_instance_storage_getter_and_setter_with_default!(
    is_killed_withdraw,
    DataKey::IsKilledWithdraw,
    bool,
    false
);
generate_instance_storage_getter_and_setter_with_default!(
    is_killed_claim,
    DataKey::IsKilledClaim,
    bool,
    false
);

generate_instance_storage_getter_and_setter_with_default!(volume_24h, DataKey::Volume24h, u128, 0);
generate_instance_storage_getter_and_setter_with_default!(
    last_trade_ts,
    DataKey::LastTradeTs,
    u64,
    0
);
generate_instance_storage_getter_and_setter_with_default!(
    last_oracle_valid,
    DataKey::LastOracleValid,
    bool,
    false
);
generate_instance_storage_getter_and_setter_with_default!(
    last_update_ts,
    DataKey::LastUpdateTs,
    u64,
    0
);

pub(crate) fn set_pool(e: &Env, pool: &Pool) {
    let key = DataKey::Pool;
    bump_instance(e);
    e.storage().instance().set(&key, pool);
}

pub(crate) fn get_pool(e: &Env) -> Pool {
    let key = DataKey::Pool;
    match e.storage().instance().get(&key) {
        Some(v) => v,
        None => panic_with_error!(e, StorageError::ValueNotInitialized),
    }
}

pub fn get_reserve_a(e: &Env) -> u128 {
    bump_instance(e);
    match e.storage().instance().get(&DataKey::ReserveA) {
        Some(v) => v,
        None => panic_with_error!(e, StorageError::ValueNotInitialized),
    }
}

pub fn get_reserve_b(e: &Env) -> u128 {
    bump_instance(e);
    match e.storage().instance().get(&DataKey::ReserveB) {
        Some(v) => v,
        None => panic_with_error!(e, StorageError::ValueNotInitialized),
    }
}

pub fn put_reserve_a(e: &Env, amount: u128) {
    bump_instance(e);
    e.storage().instance().set(&DataKey::ReserveA, &amount)
}

pub fn put_reserve_b(e: &Env, amount: u128) {
    bump_instance(e);
    e.storage().instance().set(&DataKey::ReserveB, &amount)
}

pub(crate) fn set_router(e: &Env, router: &Address) {
    let key = DataKey::Router;
    bump_instance(e);
    e.storage().instance().set(&key, router);
}

pub(crate) fn get_router(e: &Env) -> Address {
    let key = DataKey::Router;
    match e.storage().instance().get(&key) {
        Some(v) => v,
        None => panic_with_error!(e, StorageError::ValueNotInitialized),
    }
}

pub(crate) fn set_oracle_registry(e: &Env, oracle_registry: &Address) {
    let key = DataKey::OracleRegistry;
    bump_instance(e);
    e.storage().instance().set(&key, oracle_registry);
}

pub(crate) fn get_oracle_registry(e: &Env) -> Address {
    let key = DataKey::OracleRegistry;
    match e.storage().instance().get(&key) {
        Some(v) => v,
        None => panic_with_error!(e, StorageError::ValueNotInitialized),
    }
}

pub(crate) fn set_token_future_wasm(e: &Env, value: &BytesN<32>) {
    bump_instance(e);
    e.storage().instance().set(&DataKey::TokenFutureWASM, value)
}

pub(crate) fn get_token_future_wasm(e: &Env) -> BytesN<32> {
    bump_instance(e);
    match e.storage().instance().get(&DataKey::TokenFutureWASM) {
        Some(v) => v,
        None => panic_with_error!(e, StorageError::ValueNotInitialized),
    }
}
