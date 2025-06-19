use paste::paste;
use soroban_sdk::{ contracttype, panic_with_error, Address, BytesN, Env };
pub use utils::bump::bump_instance;
use utils::errors::storage_errors::StorageError;
use utils::state::pool::Pool as PoolType;
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

generate_instance_storage_getter_and_setter_with_default!(reserve_a, DataKey::ReserveA, u128, 0);
generate_instance_storage_getter_and_setter_with_default!(reserve_b, DataKey::ReserveB, u128, 0);

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
generate_instance_storage_getter_and_setter!(router, DataKey::Router, Address);

pub(crate) fn has_plane(e: &Env) -> bool {
    let key = DataKey::Plane;
    e.storage().instance().has(&key)
}

pub(crate) fn set_pool(e: &Env, pool: &PoolType) {
    let key = DataKey::Pool;
    bump_instance(e);
    e.storage().instance().set(&key, pool);
}

pub(crate) fn get_pool(e: &Env) -> PoolType {
    let key = DataKey::Pool;
    match e.storage().instance().get(&key) {
        Some(v) => v,
        None => panic_with_error!(e, StorageError::ValueNotInitialized),
    }
}

// paused ops
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
