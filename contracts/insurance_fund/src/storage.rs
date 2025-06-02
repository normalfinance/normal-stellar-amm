use soroban_sdk::{ contracttype, panic_with_error, Address, Env };
use soroban_sdk::token::{ TokenClient as SorobanTokenClient };
use utils::bump::{ bump_instance, bump_persistent, bump_temporary };
use utils::errors::storage_errors::StorageError;
use utils::{
    generate_instance_storage_getter_and_setter_with_default,
    generate_instance_storage_getter_with_default,
    generate_instance_storage_setter,
};
use paste::paste;

#[derive(Clone)]
#[contracttype]
enum DataKey {
    TokenDeposit,
    UnstakingPeriod,
    MaxShares,

    TotalShares,
    UserShares,
    SharesBase, // exponent for lp shares (for rebasing)
    LastRevenueSettleTs,
    TotalFactor, // percentage of interest for total insurance
    UserFactor, // percentage of interest for user staked insurance

    IsKilledDeposit,
    IsKilledRequestWithdraw,
    IsKilledWithdraw,
}

generate_instance_storage_getter_and_setter_with_default!(
    is_killed_deposit,
    DataKey::IsKilledDeposit,
    bool,
    false
);
generate_instance_storage_getter_and_setter_with_default!(
    is_killed_request_withdraw,
    DataKey::IsKilledRequestWithdraw,
    bool,
    false
);
generate_instance_storage_getter_and_setter_with_default!(
    is_killed_withdraw,
    DataKey::IsKilledWithdraw,
    bool,
    false
);

// 
generate_instance_storage_getter_and_setter_with_default!(
    last_revenue_settle_ts,
    DataKey::LastRevenueSettleTs,
    u64,
    0
);
generate_instance_storage_getter_and_setter_with_default!(
    unstaking_period,
    DataKey::UnstakingPeriod,
    u64,
    0
);

pub fn get_unstaking_period(e: &Env) -> u64 {
    bump_instance(e);
    match e.storage().instance().get(&DataKey::UnstakingPeriod) {
        Some(v) => v,
        None => panic_with_error!(e, StorageError::ValueNotInitialized),
    }
}

pub fn put_unstaking_period(e: &Env, amount: u64) {
    bump_instance(e);
    e.storage().instance().set(&DataKey::UnstakingPeriod, &amount)
}

pub fn get_deposit_token(e: &Env) -> Address {
    bump_instance(e);
    match e.storage().instance().get(&DataKey::TokenDeposit) {
        Some(v) => v,
        None => panic_with_error!(e, StorageError::ValueNotInitialized),
    }
}

pub fn put_deposit_token(e: &Env, amount: Address) {
    bump_instance(e);
    e.storage().instance().set(&DataKey::TokenDeposit, &amount)
}

pub fn get_total_shares(e: &Env) -> u128 {
    bump_instance(e);
    match e.storage().instance().get(&DataKey::TotalShares) {
        Some(v) => v,
        None => panic_with_error!(e, StorageError::ValueNotInitialized),
    }
}

pub fn put_total_shares(e: &Env, amount: u128) {
    bump_instance(e);
    e.storage().instance().set(&DataKey::TotalShares, &amount)
}

pub fn get_user_shares(e: &Env) -> u128 {
    bump_instance(e);
    match e.storage().instance().get(&DataKey::UserShares) {
        Some(v) => v,
        None => panic_with_error!(e, StorageError::ValueNotInitialized),
    }
}

pub fn put_user_shares(e: &Env, amount: u128) {
    bump_instance(e);
    e.storage().instance().set(&DataKey::UserShares, &amount)
}

pub fn get_shares_base(e: &Env) -> u128 {
    bump_instance(e);
    match e.storage().instance().get(&DataKey::SharesBase) {
        Some(v) => v,
        None => panic_with_error!(e, StorageError::ValueNotInitialized),
    }
}

pub fn put_shares_base(e: &Env, amount: u128) {
    bump_instance(e);
    e.storage().instance().set(&DataKey::SharesBase, &amount)
}

pub fn get_insurance_vault_amount(e: &Env) -> u128 {
    SorobanTokenClient::new(e, &get_deposit_token(e)).balance(&e.current_contract_address()) as u128
}
