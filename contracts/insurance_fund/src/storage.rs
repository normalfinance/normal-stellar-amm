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
    Token,
    UnstakingPeriod,
    MaxShares,
    TotalShares,
    SharesBase, // exponent for lp shares (for rebasing)

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
generate_instance_storage_getter_and_setter_with_default!(
    unstaking_period,
    DataKey::UnstakingPeriod,
    u64,
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
generate_instance_storage_getter_and_setter_with_default!(max_shares, DataKey::MaxShares, u128, 0);

pub fn get_token(e: &Env) -> Address {
    bump_instance(e);
    match e.storage().instance().get(&DataKey::Token) {
        Some(v) => v,
        None => panic_with_error!(e, StorageError::ValueNotInitialized),
    }
}

pub fn put_token(e: &Env, token: &Address) {
    bump_instance(e);
    e.storage().instance().set(&DataKey::Token, token)
}

pub fn get_insurance_vault_amount(e: &Env) -> u128 {
    SorobanTokenClient::new(e, &get_token(e)).balance(&e.current_contract_address()) as u128
}
