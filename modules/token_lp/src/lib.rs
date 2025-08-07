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
    TokenLP, // Token address
    TotalLPTokens, // Total token supply
}

pub mod lp_token {
    soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/lp_token.wasm");
}
pub use lp_token::{ self as token_contract, Client };
use utils::errors::storage_errors::StorageError;

pub fn get_token_lp(e: &Env) -> Address {
    bump_instance(e);
    match e.storage().instance().get(&DataKey::TokenLP) {
        Some(v) => v,
        None => panic_with_error!(e, StorageError::ValueNotInitialized),
    }
}

pub fn put_token_lp(e: &Env, contract: Address) {
    bump_instance(e);
    e.storage().instance().set(&DataKey::TokenLP, &contract)
}

pub fn get_user_balance_lp(e: &Env, user: &Address) -> u128 {
    SorobanTokenClient::new(e, &get_token_lp(e)).balance(user) as u128
}

pub fn get_total_lp_tokens(e: &Env) -> u128 {
    bump_instance(e);
    e.storage().instance().get(&DataKey::TotalLPTokens).unwrap_or(0)
}

pub fn put_total_lp_tokens(e: &Env, value: u128) {
    bump_instance(e);
    e.storage().instance().set(&DataKey::TotalLPTokens, &value)
}

pub fn burn_lp_tokens(e: &Env, from: &Address, amount: u128) {
    let total_lp = get_total_lp_tokens(e);
    put_total_lp_tokens(e, total_lp - amount);

    let lp_contract = get_token_lp(e);
    SorobanTokenClient::new(e, &lp_contract).burn(from, &(amount as i128));
}

pub fn mint_lp_tokens(e: &Env, to: &Address, amount: i128) {
    let total_lp = get_total_lp_tokens(e);
    put_total_lp_tokens(e, total_lp + (amount as u128));

    let lp_contract_id = get_token_lp(e);
    SorobanTokenAdminClient::new(e, &lp_contract_id).mint(to, &amount);
}
