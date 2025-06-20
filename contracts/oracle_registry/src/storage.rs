use paste::paste;
use soroban_sdk::{ contracttype, panic_with_error, Env, Symbol };
use utils::bump::{ bump_instance, bump_persistent };
use utils::constant::FIVE_MINUTE;
use utils::errors::storage_errors::StorageError;
use utils::state::oracle_registry::{ OracleInfo };
use utils::{
    generate_instance_storage_getter_and_setter_with_default,
    generate_instance_storage_getter_with_default,
    generate_instance_storage_getter_and_setter,
    generate_instance_storage_setter,
    generate_instance_storage_getter,
};
use crate::storage_types::{ HistoricalOracleData, OracleGuardRails };

#[derive(Clone)]
#[contracttype]
enum DataKey {
    OracleGuardRails, // a set of oracle price data validations and protections.
    OraclesSet(Symbol), // map of asset id symbol to OracleInfo.
    HistoricalOracleData(Symbol), // stores historically witnessed oracle data.
    PriceOverrideLimit, // the max an oracle price can manually be overriden in a single tx.
    PriceOverrideThreshold, // the minimum amount of time between manual price updates.
}

generate_instance_storage_getter_and_setter!(
    oracle_guard_rails,
    DataKey::OracleGuardRails,
    OracleGuardRails
);
generate_instance_storage_getter_and_setter_with_default!(
    price_override_limit,
    DataKey::PriceOverrideLimit,
    u32,
    50 // basis points (0.50%)
);
generate_instance_storage_getter_and_setter_with_default!(
    price_override_threshold,
    DataKey::PriceOverrideThreshold,
    u64,
    FIVE_MINUTE as u64
);

pub(crate) fn get_oracle_base(e: &Env, asset_id: &Symbol) -> Option<OracleInfo> {
    let key = DataKey::OraclesSet(asset_id.clone());
    match e.storage().persistent().get(&key) {
        Some(value) => {
            bump_persistent(e, &key);
            value
        }
        None => None,
    }
}

pub(crate) fn get_oracle(e: &Env, asset_id: &Symbol) -> OracleInfo {
    let result = get_oracle_base(e, asset_id);
    match result {
        Some(value) => { value }
        None => panic_with_error!(&e, StorageError::ValueNotInitialized),
    }
}

pub(crate) fn put_oracle(e: &Env, asset_id: &Symbol, info: &OracleInfo) {
    let key = DataKey::OraclesSet(asset_id.clone());
    e.storage().persistent().set(&key, info);
    bump_persistent(e, &key);
}

pub fn get_historical_oracle_data(e: &Env, asset_id: &Symbol) -> HistoricalOracleData {
    let key = DataKey::HistoricalOracleData(asset_id.clone());
    match e.storage().persistent().get(&key) {
        Some(data) => data,
        None => HistoricalOracleData::default_quote_oracle(),
    }
}

pub fn put_historical_oracle_data(e: &Env, asset_id: &Symbol, data: &HistoricalOracleData) {
    let key = DataKey::HistoricalOracleData(asset_id.clone());
    e.storage().instance().set(&key, data)
}
