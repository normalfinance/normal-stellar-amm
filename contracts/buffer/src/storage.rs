use soroban_sdk::{ contracttype, panic_with_error, Address, Env, Map };
use soroban_sdk::token::{ TokenClient as SorobanTokenClient };
use utils::bump::{ bump_instance, bump_persistent, bump_temporary };
use utils::errors::storage_errors::StorageError;
use utils::{
    generate_instance_storage_getter_and_setter_with_default,
    generate_instance_storage_getter_with_default,
    generate_instance_storage_setter,
};
use paste::paste;

use crate::reserve::Reserve;

#[derive(Clone)]
#[contracttype]
enum DataKey {
    Router, // The address of the Router contract (who can request_payout).
    FeeCollector, // The address of the Fee Collector contract (who can deposit).
    Reserve(Address), //
    LastPayoutTimestamp, // The last time a payout was executed.
    MinTimeBetweenPayouts, // The minimum time between payouts to prevent repeated or too-frequent payouts (rate limiting).
    MinReserveRatio, // The minimum reserve the Buffer must maintain

    IsKilledDeposit,
    IsKilledRequestPayout,
}

generate_instance_storage_getter_and_setter_with_default!(
    is_killed_deposit,
    DataKey::IsKilledDeposit,
    bool,
    false
);
generate_instance_storage_getter_and_setter_with_default!(
    is_killed_request_payout,
    DataKey::IsKilledRequestPayout,
    bool,
    false
);

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

pub(crate) fn set_fee_collector(e: &Env, fee_collector: &Address) {
    let key = DataKey::FeeCollector;
    bump_instance(e);
    e.storage().instance().set(&key, fee_collector);
}

pub(crate) fn get_fee_collector(e: &Env) -> Address {
    let key = DataKey::FeeCollector;
    match e.storage().instance().get(&key) {
        Some(v) => v,
        None => panic_with_error!(e, StorageError::ValueNotInitialized),
    }
}

pub(crate) fn get_reserve(e: &Env, token: Address) -> Reserve {
    let key = DataKey::Reserve(token);
    match e.storage().persistent().get(&key) {
        Some(value) => {
            bump_persistent(e, &key);
            value
        }
        None => Reserve::new(),
    }
}

pub(crate) fn put_reserve(e: &Env, token: Address, reserve_info: &Reserve) {
    let key = DataKey::Reserve(token);
    e.storage().persistent().set(&key, reserve_info);
    bump_persistent(e, &key);
}

generate_instance_storage_getter_and_setter_with_default!(
    last_payout_timestamp,
    DataKey::LastPayoutTimestamp,
    u64,
    0
);
generate_instance_storage_getter_and_setter_with_default!(
    min_time_between_payouts,
    DataKey::MinTimeBetweenPayouts,
    u64,
    0
);
generate_instance_storage_getter_and_setter_with_default!(
    min_reserve_ratio,
    DataKey::MinReserveRatio,
    u128,
    1000 // 10%
);

pub fn get_buffer_reserve_amount(e: &Env, token: &Address) -> u128 {
    SorobanTokenClient::new(e, token).balance(&e.current_contract_address()) as u128
}
