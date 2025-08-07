#![no_std]

use soroban_sdk::token::{
    StellarAssetClient as SorobanTokenAdminClient,
    TokenClient as SorobanTokenClient,
};
use soroban_sdk::{ contracttype, panic_with_error, Address, Env };
use utils::bump::bump_instance;

#[derive(Clone)]
#[contracttype]
enum DataKey {
    TokenSynthetic, // Token address
    TotalSyntheticTokens, // Total token supply
}

pub mod token {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/soroban_token_contract.wasm"
    );
}
pub use token::{ self as token_contract, Client };
use utils::errors::storage_errors::StorageError;

pub fn get_token_synthetic(e: &Env) -> Address {
    bump_instance(e);
    match e.storage().instance().get(&DataKey::TokenSynthetic) {
        Some(v) => v,
        None => panic_with_error!(e, StorageError::ValueNotInitialized),
    }
}

pub fn put_token_synthetic(e: &Env, contract: Address) {
    bump_instance(e);
    e.storage().instance().set(&DataKey::TokenSynthetic, &contract)
}

pub fn get_user_balance_synthetic(e: &Env, user: &Address) -> u128 {
    SorobanTokenClient::new(e, &get_token_synthetic(e)).balance(user) as u128
}

pub fn get_total_synthetic_tokens(e: &Env) -> u128 {
    bump_instance(e);
    e.storage().instance().get(&DataKey::TotalSyntheticTokens).unwrap_or(0)
}

pub fn put_total_synthetic_tokens(e: &Env, value: u128) {
    bump_instance(e);
    e.storage().instance().set(&DataKey::TotalSyntheticTokens, &value)
}

pub fn burn_synthetic_tokens(e: &Env, from: &Address, amount: u128) {
    let total_share = get_total_synthetic_tokens(e);
    put_total_synthetic_tokens(e, total_share - amount);

    let synthetic_contract = get_token_synthetic(e);
    SorobanTokenClient::new(e, &synthetic_contract).burn(from, &(amount as i128));
}

pub fn mint_synthetic_tokens(e: &Env, to: &Address, amount: i128) {
    let total_share = get_total_synthetic_tokens(e);
    put_total_synthetic_tokens(e, total_share + (amount as u128));

    let synthetic_contract_id = get_token_synthetic(e);
    SorobanTokenAdminClient::new(e, &synthetic_contract_id).mint(to, &amount);
}
