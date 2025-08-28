use normal_rust_types::types::{InsuranceFundReserve, WhitelistToken};
use crate::reserve::InsuranceFundReserveExt;
use paste::paste;
use soroban_sdk::token::TokenClient as SorobanTokenClient;
use soroban_sdk::{contracttype, panic_with_error, Address, Env, Symbol, Vec};
use utils::bump::{bump_instance, bump_persistent};
use utils::constant::THIRTEEN_DAY;
use normal_rust_types::StorageError;
use utils::{
    generate_instance_storage_getter, generate_instance_storage_getter_and_setter,
    generate_instance_storage_getter_and_setter_with_default,
    generate_instance_storage_getter_with_default, generate_instance_storage_setter,
};

// TODO: must we track the interest paid to avoid counting uncollected interest as insurance?

#[derive(Clone)]
#[contracttype]
enum DataKey {
    OracleRegistry, // the address of the Oracle Registry.
    PoolRouter,     // the address of the Pool Router.

    PremiumToken,           // the address of the token used to pay premiums.
    PremiumPayers(Address), // list of accounts allowed to pay premium.

    TokenWhitelistVec,
    TokenWhitelist(Address), // map of token address to WhitelistTokenStatus.

    Reserve(Address), // map of token address to InsuranceFundReserve.

    UnstakingPeriod, // a period of time stakers must wait once requesting withdrawal to actually withdraw.
    OptimalInsurance, // the maximum amount of insurance to adequately insure the protocol.
    OptimalUtilization, // the optimal utilization point (utilization = current insurance / optimal insurance)
    BaseRate,           // the base interest rate when utilization is 0%
    RateSlopeA,         // the slope before hitting optimal utilization (gradual increase)
    RateSlopeB,         // the slope after optimal utilization (steep increase)

    // paused ops
    IsKilledDeposit,
    IsKilledRequestWithdraw,
    IsKilledWithdraw,
}

// Addresses
generate_instance_storage_getter_and_setter_with_default!(
    oracle_registry,
    DataKey::OracleRegistry,
    Address,
    Address::from_str(&Env::default(), "")
);
generate_instance_storage_getter_and_setter_with_default!(
    pool_router,
    DataKey::PoolRouter,
    Address,
    Address::from_str(&Env::default(), "")
);
generate_instance_storage_getter_and_setter_with_default!(
    premium_token,
    DataKey::PremiumToken,
    Address,
    Address::from_str(&Env::default(), "")
);

// Reserve
pub(crate) fn get_reserve(e: &Env, token: &Address) -> InsuranceFundReserve {
    let key = DataKey::Reserve(token.clone());
    match e.storage().persistent().get(&key) {
        Some(value) => {
            bump_persistent(e, &key);
            value
        }
        None => InsuranceFundReserve::new(token.clone(), e.ledger().timestamp()),
    }
}

pub(crate) fn put_reserve(e: &Env, token: &Address, reserve_info: &InsuranceFundReserve) {
    let key = DataKey::Reserve(token.clone());
    e.storage().persistent().set(&key, reserve_info);
    bump_persistent(e, &key);
}

// Config
generate_instance_storage_getter_and_setter_with_default!(
    unstaking_period,
    DataKey::UnstakingPeriod,
    u64,
    THIRTEEN_DAY
);
generate_instance_storage_getter_and_setter_with_default!(
    optimal_insurance,
    DataKey::OptimalInsurance,
    u128,
    0
);

// Interest
generate_instance_storage_getter_and_setter!(optimal_utilization, DataKey::OptimalUtilization, u32);
generate_instance_storage_getter_and_setter!(base_rate, DataKey::BaseRate, i32);
generate_instance_storage_getter_and_setter!(rate_slope_a, DataKey::RateSlopeA, u32);
generate_instance_storage_getter_and_setter!(rate_slope_b, DataKey::RateSlopeB, u32);

// Paused Ops
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

// Utils
pub fn get_contract_token_balance(e: &Env, token: &Address) -> u128 {
    SorobanTokenClient::new(e, token).balance(&e.current_contract_address()) as u128
}

// Premium Payers

/// Checks if an address is whitelisted
/// Returns true if whitelisted, false if not (missing entries are treated as not whitelisted)
pub fn get_premium_payer_status(e: &Env, address: &Address) -> bool {
    let key = DataKey::PremiumPayers(address.clone());
    match e.storage().persistent().get::<DataKey, Address>(&key) {
        Some(_) => {
            bump_persistent(e, &key);
            true
        }
        None => false,
    }
}

/// Sets whitelist status for an address
/// If status is true, adds the address to whitelist; if false, removes it
pub fn set_premium_payer_status(e: &Env, address: &Address, status: bool) {
    let key = DataKey::PremiumPayers(address.clone());
    if status {
        e.storage().persistent().set(&key, address);
        bump_persistent(e, &key);
    } else {
        e.storage().persistent().remove(&key);
    }
}

// Token Whitelist

/// Checks if an address is whitelisted
/// Returns true if whitelisted, false if not (missing entries are treated as not whitelisted)
pub fn get_token_whitelist_status(e: &Env, address: &Address) -> bool {
    let key = DataKey::TokenWhitelist(address.clone());
    match e
        .storage()
        .persistent()
        .get::<DataKey, WhitelistToken>(&key)
    {
        Some(token) => {
            bump_persistent(e, &key);
            token.active
        }
        None => false,
    }
}

pub fn get_token_whitelist(e: &Env, address: &Address) -> WhitelistToken {
    let key = DataKey::TokenWhitelist(address.clone());
    match e
        .storage()
        .persistent()
        .get::<DataKey, WhitelistToken>(&key)
    {
        Some(token) => {
            bump_persistent(e, &key);
            token
        }
        None => panic_with_error!(e, StorageError::ValueNotInitialized),
    }
}

/// Sets whitelist status for an address
/// If status is true, adds the address to whitelist; if false, removes it
pub fn set_token_whitelist(e: &Env, token: &WhitelistToken) {
    let key = DataKey::TokenWhitelist(token.address.clone());
    e.storage().persistent().set(&key, token);
    bump_persistent(e, &key);
}

pub fn remove_token_whitelist(e: &Env, token: &Address) {
    let key = DataKey::TokenWhitelist(token.clone());
    e.storage().persistent().remove(&key);
}

pub fn get_token_whitelist_vec(e: &Env) -> Vec<Address> {
    let key = DataKey::TokenWhitelistVec;
    match e.storage().persistent().get(&key) {
        Some(v) => {
            bump_persistent(e, &key);
            v
        }
        None => Vec::new(e),
    }
}

pub fn set_token_whitelist_vec(e: &Env, pools: &Vec<Address>) {
    let key = DataKey::TokenWhitelistVec;
    e.storage().persistent().set(&key, pools);
    bump_persistent(e, &key);
}
