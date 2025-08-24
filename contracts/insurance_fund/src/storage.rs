use paste::paste;
use soroban_sdk::token::TokenClient as SorobanTokenClient;
use soroban_sdk::{contracttype, panic_with_error, Address, Env};
use utils::bump::{bump_instance, bump_persistent};
use utils::constant::THIRTEEN_DAY;
use utils::errors::storage_errors::StorageError;
use utils::{
    generate_instance_storage_getter, generate_instance_storage_getter_and_setter,
    generate_instance_storage_getter_and_setter_with_default,
    generate_instance_storage_getter_with_default, generate_instance_storage_setter,
};

// TODO: must we track the interest paid to avoid counting uncollected interest as insurance?

#[derive(Clone)]
#[contracttype]
enum DataKey {
    OracleRegistry,
    Token,      // the token address of supported deposits.
    PoolRouter, // the address of the Pool Router.

    UnstakingPeriod, // a period of time stakers must wait once requesting withdrawal to actually withdraw.
    OptimalInsurance, // the maximum amount of insurance (in Token amount) to adequately insure the protocol.
    TotalShares,      // the total amount of issued shares.
    SharesBase,       // exponent for lp shares (for rebasing).
    OptimalUtilization, // the optimal utilization point (utilization = current insurance / optimal insurance)
    BaseRate,           // the base interest rate when utilization is 0%
    RateSlopeA,         // the slope before hitting optimal utilization (gradual increase)
    RateSlopeB,         // the slope after optimal utilization (steep increase)

    PremiumWhitelist(Address), // List of accounts explicitly allowed to pay premium

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
    token,
    DataKey::Token,
    Address,
    Address::from_str(&Env::default(), "")
);
generate_instance_storage_getter_and_setter_with_default!(
    pool_router,
    DataKey::PoolRouter,
    Address,
    Address::from_str(&Env::default(), "")
);

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
generate_instance_storage_getter_and_setter_with_default!(
    total_shares,
    DataKey::TotalShares,
    u128,
    0
);
generate_instance_storage_getter_and_setter_with_default!(
    shares_base,
    DataKey::SharesBase,
    u128,
    0
);
generate_instance_storage_getter_and_setter!(optimal_utilization, DataKey::OptimalUtilization, u32);
generate_instance_storage_getter_and_setter!(base_rate, DataKey::BaseRate, i32);
generate_instance_storage_getter_and_setter!(rate_slope_a, DataKey::RateSlopeA, u32);
generate_instance_storage_getter_and_setter!(rate_slope_b, DataKey::RateSlopeB, u32);

// paused ops
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
pub fn get_insurance_vault_amount(e: &Env) -> u128 {
    SorobanTokenClient::new(e, &get_token(e)).balance(&e.current_contract_address()) as u128
}

// Whitelist functions
// Note: These use manual implementation (not macros) because they are keyed storage patterns
// that require persistent storage, custom TTL management, and Address-based keys.
// This follows the same pattern as Component(Address) and ComponentBalance(Address) storage.

/// Checks if an address is whitelisted
/// Returns true if whitelisted, false if not (missing entries are treated as not whitelisted)
pub fn get_premium_whitelist_status(e: &Env, address: &Address) -> bool {
    let key = DataKey::PremiumWhitelist(address.clone());
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
pub fn set_premium_whitelist_status(e: &Env, address: &Address, status: bool) {
    let key = DataKey::PremiumWhitelist(address.clone());
    if status {
        e.storage().persistent().set(&key, address);
        e.storage().persistent().extend_ttl(&key, 100000, 100000);
    } else {
        e.storage().persistent().remove(&key);
    }
}
