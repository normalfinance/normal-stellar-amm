use paste::paste;
use soroban_sdk::{contracttype, panic_with_error, Env, Symbol};
use utils::bump::{bump_instance, bump_persistent, bump_temporary};
use utils::constant::ONE_HOUR;
use utils::errors::storage_errors::StorageError;
use utils::state::oracle_registry::{HistoricalOracleData, OracleGuardRails, OracleInfo};
use utils::{
    generate_instance_storage_getter, generate_instance_storage_getter_and_setter,
    generate_instance_storage_setter,
};

#[derive(Clone)]
#[contracttype]
enum DataKey {
    OracleGuardRails,   // a set of oracle price data validations and protections.
    OraclesSet(Symbol), // map of asset (i.e. "BTC") > OracleInfo.
    HistoricalOracleData(Symbol), // map of asset (i.e. "BTC") > HistoricalOracleData (historically witnessed oracle data - LEGACY).

    HotOracleData(Symbol),
    ColdOracleData(Symbol),
    OracleFetchConfig, // Configuration for oracle fetch optimization policies
}

#[contracttype]
#[derive(Default, Clone, Copy, Debug)]
pub struct HotOracleData {
    pub data: HistoricalOracleData,
    pub created_at: u64,
    pub last_accessed: u64,
}

#[contracttype]
#[derive(Default, Clone, Copy, Debug)]
pub struct ColdOracleData {
    pub data: HistoricalOracleData,
    pub checkpoint_timestamp: u64,
}

// Oracle fetch configuration for managing hot/cold storage optimization
#[contracttype]
#[derive(Clone, Copy, Debug)]
pub struct OracleFetchConfig {
    pub hot_data_ttl_seconds: u64,
    pub cold_checkpoint_interval: u64,
}

impl Default for OracleFetchConfig {
    fn default() -> Self {
        OracleFetchConfig {
            hot_data_ttl_seconds: 48 * ONE_HOUR,
            cold_checkpoint_interval: 24 * ONE_HOUR,
        }
    }
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
    let now = e.ledger().timestamp();

    // 1. Try hot data first
    if let Some(hot_data) = get_hot_oracle_data(e, asset) {
        put_hot_oracle_data_with_access_time(e, asset, &hot_data, now);
        return hot_data.data;
    }

    // 2. Fall back to cold data
    if let Some(hot_data) = get_hot_oracle_data(e, asset) {
        put_hot_oracle_data_with_access_time(e, asset, &hot_data, now);
        return hot_data.data;
    }

    // 2. Fall back to cold data (checkpoint-based historical data)
    if let Some(cold_data) = get_cold_oracle_data(e, asset) {
        let config = get_oracle_fetch_config_or_default(e);
        let age = now - cold_data.checkpoint_timestamp;
        if age <= config.hot_data_ttl_seconds {
            let hot_data = HotOracleData {
                data: cold_data.data,
                created_at: now,
                last_accessed: now,
            };
            put_hot_oracle_data(e, asset, &hot_data);
        }
        return cold_data.data;
    }

    // 3. Fall back to legacy persistent storage
    if let Some(legacy_data) = get_legacy_oracle_data(e, asset) {
        let hot_data = HotOracleData {
            data: legacy_data,
            created_at: now,
            last_accessed: now,
        };
        put_hot_oracle_data(e, asset, &hot_data);

        let cold_data = ColdOracleData {
            data: legacy_data,
            checkpoint_timestamp: now,
        };
        put_cold_oracle_data(e, asset, &cold_data);

        return legacy_data;
    }

    // 4. Return default if nothing found
    HistoricalOracleData::default_quote_oracle()
}

pub fn put_historical_oracle_data(e: &Env, asset: &Symbol, data: &HistoricalOracleData) {
    let now = e.ledger().timestamp();
    let config = get_oracle_fetch_config_or_default(e);

    // Always write to hot storage first
    let hot_data = HotOracleData {
        data: *data,
        created_at: now,
        last_accessed: now,
    };
    put_hot_oracle_data(e, asset, &hot_data);

    // Check if we need to create a cold checkpoint
    if should_create_cold_checkpoint(e, asset, now, &config) {
        create_cold_checkpoint(e, asset, data, now);
    }
}

pub fn get_oracle_fetch_config(e: &Env) -> OracleFetchConfig {
    match e.storage().instance().get(&DataKey::OracleFetchConfig) {
        Some(config) => {
            bump_instance(e);
            config
        }
        None => OracleFetchConfig::default(),
    }
}

// Internal setter
pub fn set_oracle_fetch_config(e: &Env, config: &OracleFetchConfig) {
    e.storage()
        .instance()
        .set(&DataKey::OracleFetchConfig, config);
    bump_instance(e);
}

// Hot storage operations
fn get_hot_oracle_data(e: &Env, asset: &Symbol) -> Option<HotOracleData> {
    let key = DataKey::HotOracleData(asset.clone());
    match e.storage().temporary().get(&key) {
        Some(data) => {
            bump_temporary(e, &key);
            Some(data)
        }
        None => None,
    }
}

fn put_hot_oracle_data(e: &Env, asset: &Symbol, data: &HotOracleData) {
    let key = DataKey::HotOracleData(asset.clone());
    e.storage().temporary().set(&key, data);
    bump_temporary(e, &key);
}

fn put_hot_oracle_data_with_access_time(
    e: &Env,
    asset: &Symbol,
    data: &HotOracleData,
    access_time: u64,
) {
    let updated_data = HotOracleData {
        data: data.data,
        created_at: data.created_at,
        last_accessed: access_time,
    };
    put_hot_oracle_data(e, asset, &updated_data);
}

// Cold storage operations
fn get_cold_oracle_data(e: &Env, asset: &Symbol) -> Option<ColdOracleData> {
    let key = DataKey::ColdOracleData(asset.clone());
    match e.storage().persistent().get(&key) {
        Some(data) => {
            bump_persistent(e, &key);
            Some(data)
        }
        None => None,
    }
}

fn put_cold_oracle_data(e: &Env, asset: &Symbol, data: &ColdOracleData) {
    let key = DataKey::ColdOracleData(asset.clone());
    e.storage().persistent().set(&key, data);
    bump_persistent(e, &key);
}

// Legacy storage operations
fn get_legacy_oracle_data(e: &Env, asset: &Symbol) -> Option<HistoricalOracleData> {
    let key = DataKey::HistoricalOracleData(asset.clone());
    match e.storage().persistent().get(&key) {
        Some(data) => {
            bump_persistent(e, &key);
            Some(data)
        }
        None => None,
    }
}

// Helper function for oracle fetch optimization
pub fn get_oracle_fetch_config_or_default(e: &Env) -> OracleFetchConfig {
    let config = get_oracle_fetch_config(e);

    // Check if config has default values
    if config.hot_data_ttl_seconds == 0 {
        // Initialize with default config if not set
        let default_config = OracleFetchConfig::default();
        set_oracle_fetch_config(e, &default_config);
        default_config
    } else {
        config
    }
}

// Smart checkpointing logic
fn should_create_cold_checkpoint(
    e: &Env,
    asset: &Symbol,
    now: u64,
    config: &OracleFetchConfig,
) -> bool {
    if let Some(existing_cold) = get_cold_oracle_data(e, asset) {
        let time_since_checkpoint = now - existing_cold.checkpoint_timestamp;
        time_since_checkpoint >= config.cold_checkpoint_interval
    } else {
        // Create first checkpoint
        true
    }
}

fn create_cold_checkpoint(e: &Env, asset: &Symbol, data: &HistoricalOracleData, now: u64) {
    let cold_data = ColdOracleData {
        data: *data,
        checkpoint_timestamp: now,
    };

    put_cold_oracle_data(e, asset, &cold_data);
}

pub fn remove_oracle(e: &Env, asset: &Symbol) {
    let oracle_key = DataKey::OraclesSet(asset.clone());
    let historical_key = DataKey::HistoricalOracleData(asset.clone());

    e.storage().persistent().remove(&oracle_key);
    e.storage().persistent().remove(&historical_key);
}
