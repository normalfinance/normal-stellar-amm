use crate::storage_types::{HistoricalOracleData, OracleGuardRails};
use paste::paste;
use soroban_sdk::{contracttype, panic_with_error, Env, Symbol};
use utils::bump::{bump_instance, bump_persistent};
use utils::errors::storage_errors::StorageError;
use utils::state::oracle_registry::OracleInfo;
use utils::{
    generate_instance_storage_getter, generate_instance_storage_getter_and_setter,
    generate_instance_storage_setter,
};

#[derive(Clone)]
#[contracttype]
enum DataKey {
    OracleGuardRails,   // a set of oracle price data validations and protections.
    OraclesSet(Symbol), // map of asset (i.e. "BTC") > OracleInfo.
    HistoricalOracleData(Symbol), // map of asset (i.e. "BTC") > HistoricalOracleData (historically witnessed oracle data).
}

generate_instance_storage_getter_and_setter!(
    oracle_guard_rails,
    DataKey::OracleGuardRails,
    OracleGuardRails
);

pub(crate) fn get_oracle_base(e: &Env, asset: &Symbol) -> Option<OracleInfo> {
    let key = DataKey::OraclesSet(asset.clone());
    match e.storage().persistent().get(&key) {
        Some(value) => {
            bump_persistent(e, &key);
            value
        }
        None => None,
    }
}

pub(crate) fn get_oracle(e: &Env, asset: &Symbol) -> OracleInfo {
    let result = get_oracle_base(e, asset);
    match result {
        Some(value) => value,
        None => panic_with_error!(&e, StorageError::ValueNotInitialized),
    }
}

pub(crate) fn put_oracle(e: &Env, asset: &Symbol, info: &OracleInfo) {
    let key = DataKey::OraclesSet(asset.clone());
    e.storage().persistent().set(&key, info);
    bump_persistent(e, &key);
}

pub fn get_historical_oracle_data(e: &Env, asset: &Symbol) -> HistoricalOracleData {
    let key = DataKey::HistoricalOracleData(asset.clone());
    match e.storage().persistent().get(&key) {
        Some(data) => data,
        None => HistoricalOracleData::default_quote_oracle(),
    }
}

pub fn put_historical_oracle_data(e: &Env, asset: &Symbol, data: &HistoricalOracleData) {
    let key = DataKey::HistoricalOracleData(asset.clone());
    e.storage().instance().set(&key, data)
}
