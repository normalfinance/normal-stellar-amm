use paste::paste;
use soroban_sdk::token::TokenClient as SorobanTokenClient;
use soroban_sdk::{ panic_with_error, contracttype, Address, Env, Map };
use utils::errors::storage_errors::StorageError;
use utils::bump::{ bump_instance, bump_persistent, bump_temporary };
use utils::{
    generate_instance_storage_getter_and_setter,
    generate_instance_storage_getter_and_setter_with_default,
    generate_instance_storage_getter_with_default,
    generate_instance_storage_setter,
    generate_instance_storage_getter,
};

use crate::reserve::Reserve;

#[derive(Clone)]
#[contracttype]
enum DataKey {
    FeeCollector, // The address of the Fee Collector contract (only one who can call `deposit()`).
    MinTimeBetweenPayouts, // The minimum time between payouts to prevent repeated or too-frequent payouts (rate limiting).

    Reserve(Address), // Map of Buffer reserve state for each token.
    LastPayoutTimestamp, // The last time a payout was executed.
    MinReserveRatio, // The minimum reserve the Buffer must maintain

    IsKilledDeposit,
    IsKilledResolveLiquidityDeficit,
}
generate_instance_storage_getter_and_setter!(fee_collector, DataKey::FeeCollector, Address);
generate_instance_storage_getter_and_setter_with_default!(
    is_killed_deposit,
    DataKey::IsKilledDeposit,
    bool,
    false
);
generate_instance_storage_getter_and_setter_with_default!(
    is_killed_resolve_liquidity_deficit,
    DataKey::IsKilledResolveLiquidityDeficit,
    bool,
    false
);

pub(crate) fn get_reserve(e: &Env, token: &Address) -> Reserve {
    let key = DataKey::Reserve(token.clone());
    match e.storage().persistent().get(&key) {
        Some(value) => {
            bump_persistent(e, &key);
            value
        }
        None => Reserve::new(e.ledger().timestamp()),
    }
}

pub(crate) fn put_reserve(e: &Env, token: &Address, reserve_info: &Reserve) {
    let key = DataKey::Reserve(token.clone());
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
    u32,
    1000 // 10%
);

pub fn get_buffer_reserve_amount(e: &Env, token: &Address) -> u128 {
    SorobanTokenClient::new(e, token).balance(&e.current_contract_address()) as u128
}
