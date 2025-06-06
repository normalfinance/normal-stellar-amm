use soroban_sdk::{ contracttype, panic_with_error, Env, Map };
use utils::bump::{ bump_instance, bump_persistent, bump_temporary };
use utils::constant::{ PRICE_PRECISION, PRICE_PRECISION_I128 };
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
    pub last_oracle_price: u128,
    /// amount of time since last update
    pub last_oracle_delay: u64,
    /// precision: PRICE_PRECISION
    pub last_oracle_price_twap: u128,
    /// unix_timestamp of last snapshot
    pub last_oracle_price_twap_ts: u64,
}

impl HistoricalOracleData {
    pub fn default_quote_oracle() -> Self {
        HistoricalOracleData {
            last_oracle_price: PRICE_PRECISION,
            last_oracle_delay: 0,
            last_oracle_price_twap: PRICE_PRECISION,
            ..HistoricalOracleData::default()
        }
    }

    pub fn default_price(price: u128) -> Self {
        HistoricalOracleData {
            last_oracle_price: price,
            last_oracle_delay: 10,
            last_oracle_price_twap: price,
            ..HistoricalOracleData::default()
        }
    }

    pub fn default_with_current_oracle(oracle_price_data: OraclePriceData) -> Self {
        HistoricalOracleData {
            last_oracle_price: oracle_price_data.price,
            last_oracle_delay: oracle_price_data.delay,
            last_oracle_price_twap: oracle_price_data.price,
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
    50 // basis points (0.50%)
);

pub(crate) fn get_oracle(e: &Env, asset_id: AssetId) -> OracleInfo {
    let key = DataKey::OraclesSet(asset_id);
    match e.storage().persistent().get(&key) {
        Some(value) => {
            bump_persistent(e, &key);
            value
        }
        None => panic_with_error!(&e, StorageError::ValueNotInitialized),
    }
}

pub(crate) fn put_oracle(e: &Env, asset_id: AssetId, info: &OracleInfo) {
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

pub fn get_historical_oracle_data(e: &Env, asset_id: AssetId) -> HistoricalOracleData {
    let key = DataKey::HistoricalOracleData(asset_id);
    match e.storage().persistent().get(&key) {
        Some(data) => data,
        None => panic_with_error!(&e, StorageError::ValueNotInitialized),
    }
}

pub fn put_historical_oracle_data(e: &Env, asset_id: AssetId, data: &HistoricalOracleData) {
    let key = DataKey::HistoricalOracleData(asset_id);
    e.storage().instance().set(&key, data)
}
