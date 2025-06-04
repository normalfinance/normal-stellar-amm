use soroban_sdk::{ contracttype, panic_with_error, Env, Map };
use utils::bump::{ bump_instance, bump_persistent, bump_temporary };
use utils::constant::{ PRICE_PRECISION_I128 };
use utils::errors::storage_errors::StorageError;
use utils::oracle::{ OracleGuardRails, OraclePriceData };
use utils::storage::{ AssetId, OracleInfo };
use utils::{
    generate_instance_storage_getter_and_setter_with_default,
    generate_instance_storage_getter_with_default,
    generate_instance_storage_setter,
};
use paste::paste;

use crate::errors::OracleRegistryError;

#[derive(Clone)]
#[contracttype]
enum DataKey {
    OracleGuardRails, // Oracle price data validations and protections
    OraclesSet(AssetId), // Map of AssetId to OracleInfo
    HistoricalOracleData(AssetId), // Stores historically witnessed oracle data
    PriceOverrideLimit, // The max an oracle price can manually be overriden in a single tx
}

//  ___________  ___  ___  _______    _______   ________
// ("     _   ")|"  \/"  ||   __ "\  /"     "| /"       )
//  )__/  \\__/  \   \  / (. |__) :)(: ______)(:   \___/
//     \\_ /      \\  \/  |:  ____/  \/    |   \___  \
//     |.  |      /   /   (|  /      // ___)_   __/  \\
//     \:  |     /   /   /|__/ \    (:      "| /" \   :)
//      \__|    |___/   (_______)    \_______)(_______/

#[contracttype]
#[derive(Default, Clone, Copy, Eq, PartialEq, Debug)]
pub struct HistoricalOracleData {
    /// precision: PRICE_PRECISION
    pub last_oracle_price: i128,
    /// precision: PRICE_PRECISION
    pub last_oracle_conf: u64,
    /// amount of time since last update
    pub last_oracle_delay: i64,
    /// precision: PRICE_PRECISION
    pub last_oracle_price_twap: i128,
    /// unix_timestamp of last snapshot
    pub last_oracle_price_twap_ts: i64,
}

impl HistoricalOracleData {
    pub fn default_quote_oracle() -> Self {
        HistoricalOracleData {
            last_oracle_price: PRICE_PRECISION_I128,
            last_oracle_conf: 0,
            last_oracle_delay: 0,
            last_oracle_price_twap: PRICE_PRECISION_I128,
            ..HistoricalOracleData::default()
        }
    }

    pub fn default_price(price: i128) -> Self {
        HistoricalOracleData {
            last_oracle_price: price,
            last_oracle_conf: 0,
            last_oracle_delay: 10,
            last_oracle_price_twap: price,
            ..HistoricalOracleData::default()
        }
    }

    pub fn default_with_current_oracle(oracle_price_data: OraclePriceData) -> Self {
        HistoricalOracleData {
            last_oracle_price: oracle_price_data.price,
            last_oracle_conf: oracle_price_data.confidence,
            last_oracle_delay: oracle_price_data.delay,
            last_oracle_price_twap: oracle_price_data.price,
            // last_oracle_price_twap_ts: now,
            ..HistoricalOracleData::default()
        }
    }
}

//  ____  ____  ___________  __    ___        ________
// ("  _||_ " |("     _   ")|" \  |"  |      /"       )
// |   (  ) : | )__/  \\__/ ||  | ||  |     (:   \___/
// (:  |  | . )    \\_ /    |:  | |:  |      \___  \
//  \\ \__/ //     |.  |    |.  |  \  |___    __/  \\
//  /\\ __ //\     \:  |    /\  |\( \_|:  \  /" \   :)
// (__________)     \__|   (__\_|_)\_______)(_______/

generate_instance_storage_getter_and_setter_with_default!(
    price_override_limit,
    DataKey::PriceOverrideLimit,
    u128,
    50 // basis points (0.50&)
);

fn get_oracles(e: &Env) -> Map<AssetId, OracleInfo> {
    let key = DataKey::OraclesSet;
    match e.storage().persistent().get(&key) {
        Some(value) => {
            bump_persistent(e, &key);
            value
        }
        None => Map::new(e),
    }
}

pub fn has_oracle(e: &Env, asset_id: AssetId) -> bool {
    get_oracles(e).contains_key(asset_id)
}

pub fn get_oracle(e: &Env, asset_id: &AssetId) -> OracleInfo {
    let oracles = get_oracles(e);
    match oracles.get(asset_id) {
        Some(data) => data,
        None => panic_with_error!(&e, OracleRegistryError::OracleNotFound),
    }
}

pub fn set_oracle(e: &Env, asset_id: AssetId, info: OracleInfo) {
    let key = DataKey::OraclesSet;
    let mut oracles: Map<AssetId, OracleInfo> = e
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| Map::new(e));

    oracles.set(asset_id, info);
    e.storage().persistent().set(&key, &oracles);
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

pub fn get_historical_oracle_data(e: &Env, asset_id: &AssetId) -> HistoricalOracleData {
    let key = DataKey::HistoricalOracleData;
    match e.storage().persistent().get(&key).get(asset_id) {
        Some(data) => data,
        None => panic_with_error!(&e, StorageError::ValueNotInitialized),
    }
}

pub fn put_historical_oracle_data(e: &Env, asset_id: &AssetId, data: &HistoricalOracleData) {
    let key = DataKey::HistoricalOracleData(asset_id);
    e.storage().instance().set(&key, data)
}
