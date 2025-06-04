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
    MaxBalance, //
}

generate_instance_storage_getter_and_setter_with_default!(
    max_balance,
    DataKey::MaxBalance,
    u128,
    0
);

pub(crate) fn set_deposit_token(e: &Env, deposit_token: &Address) {
    let key = DataKey::TokenDeposit;
    bump_instance(e);
    e.storage().instance().set(&key, deposit_token);
}

pub(crate) fn get_deposit_token(e: &Env) -> Address {
    let key = DataKey::TokenDeposit;
    match e.storage().instance().get(&key) {
        Some(v) => v,
        None => panic_with_error!(e, StorageError::ValueNotInitialized),
    }
}

pub fn get_buffer_vault_amount(e: &Env) -> u128 {
    SorobanTokenClient::new(e, &get_deposit_token(e)).balance(&e.current_contract_address()) as u128
}
