use paste::paste;
use soroban_sdk::token::TokenClient as SorobanTokenClient;
use soroban_sdk::{contracttype, Address, Env};
use utils::bump::{bump_instance, bump_persistent};
use utils::{
    generate_instance_storage_getter_and_setter_with_default,
    generate_instance_storage_getter_with_default, generate_instance_storage_setter,
};

use crate::reserve::Reserve;

#[derive(Clone)]
#[contracttype]
enum DataKey {
    Reserve(Address),      // map of Buffer reserve state for each token.
    MinTimeBetweenPayouts, // the minimum time between payouts to prevent repeated or too-frequent payouts (rate limiting).
    LastPayoutTimestamp,   // the last time a payout was executed.
    MinReserveRatio,       // the minimum reserve the Buffer must maintain.

    // Paused Ops
    IsKilledDeposit,
    IsKilledResolveLiquidityDeficit,
}

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

// Paused Ops
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

// Utils
pub fn get_buffer_reserve_amount(e: &Env, token: &Address) -> u128 {
    SorobanTokenClient::new(e, token).balance(&e.current_contract_address()) as u128
}
