use paste::paste;
use soroban_sdk::{ contracttype, panic_with_error, Env, Symbol };
use utils::bump::{ bump_instance, bump_persistent, bump_temporary };
use utils::errors::storage_errors::StorageError;
use utils::storage::{ OracleInfo };
use utils::{
    generate_instance_storage_getter_and_setter_with_default,
    generate_instance_storage_getter_with_default,
    generate_instance_storage_setter,
};
use crate::storage_types::{ HistoricalOracleData, OracleGuardRails };

#[derive(Clone)]
#[contracttype]
enum DataKey {
    OracleGuardRails, // Oracle price data validations and protections
    OraclesSet(Symbol), // Map of Symbol to OracleInfo
    HistoricalOracleData(Symbol), // Stores historically witnessed oracle data
    PriceOverrideLimit, // The max an oracle price can manually be overriden in a single tx
}

generate_instance_storage_getter_and_setter_with_default!(
    price_override_limit,
    DataKey::PriceOverrideLimit,
    u128,
    50 // basis points (0.50%)
);

pub(crate) fn get_oracle(e: &Env, asset_id: Symbol) -> OracleInfo {
    let key = DataKey::OraclesSet(asset_id);
    match e.storage().persistent().get(&key) {
        Some(value) => {
            bump_persistent(e, &key);
            value
        }
        None => panic_with_error!(&e, StorageError::ValueNotInitialized),
    }
}

pub(crate) fn put_oracle(e: &Env, asset_id: Symbol, info: &OracleInfo) {
    let key = DataKey::OraclesSet(asset_id);
    e.storage().persistent().set(&key, info);
    bump_persistent(e, &key);
}

pub fn get_oracle_guard_rails(e: &Env) -> OracleGuardRails {
    let key = DataKey::OracleGuardRails;
    match e.storage().persistent().get(&key) {
        Some(v) => {
            bump_persistent(e, &key);
            v
        }
        None => panic_with_error!(&e, StorageError::ValueNotInitialized),
    }
}

pub fn put_oracle_guard_rails(e: &Env, guard_rails: &OracleGuardRails) {
    let key = DataKey::OracleGuardRails;
    e.storage().persistent().set(&key, guard_rails);
    bump_persistent(e, &key);
}

pub fn get_historical_oracle_data(e: &Env, asset_id: Symbol) -> HistoricalOracleData {
    let key = DataKey::HistoricalOracleData(asset_id);
    match e.storage().persistent().get(&key) {
        Some(data) => data,
        None => panic_with_error!(&e, StorageError::ValueNotInitialized),
    }
}

pub fn put_historical_oracle_data(e: &Env, asset_id: Symbol, data: &HistoricalOracleData) {
    let key = DataKey::HistoricalOracleData(asset_id);
    e.storage().instance().set(&key, data)
}
