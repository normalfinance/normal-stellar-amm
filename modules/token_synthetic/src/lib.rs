#![no_std]

use soroban_sdk::token::{StellarAssetClient, TokenClient};
use soroban_sdk::{contracttype, panic_with_error, Address, Env};
use utils::bump::bump_instance;

#[derive(Clone)]
#[contracttype]
enum DataKey {
    SacAddress,           // SAC address
    TotalSyntheticTokens, // Total token supply
}

use normal_rust_types::StorageError;

pub fn get_sac_address(e: &Env) -> Address {
    bump_instance(e);
    match e.storage().instance().get(&DataKey::SacAddress) {
        Some(v) => v,
        None => panic_with_error!(e, StorageError::ValueNotInitialized),
    }
}

pub fn put_sac_address(e: &Env, contract: Address) {
    bump_instance(e);
    e.storage().instance().set(&DataKey::SacAddress, &contract)
}

pub fn get_user_balance_synthetic(e: &Env, user: &Address) -> u128 {
    TokenClient::new(e, &get_sac_address(e)).balance(user) as u128
}

pub fn get_total_synthetic_tokens(e: &Env) -> u128 {
    bump_instance(e);
    e.storage()
        .instance()
        .get(&DataKey::TotalSyntheticTokens)
        .unwrap_or(0)
}

pub fn put_total_synthetic_tokens(e: &Env, value: u128) {
    bump_instance(e);
    e.storage()
        .instance()
        .set(&DataKey::TotalSyntheticTokens, &value)
}

pub fn burn_synthetic_tokens(e: &Env, from: &Address, amount: u128) {
    let total_share = get_total_synthetic_tokens(e);
    put_total_synthetic_tokens(e, total_share - amount);

    let token_client = StellarAssetClient::new(e, &get_sac_address(e));
    token_client.clawback(from, &(amount as i128));
}

pub fn mint_synthetic_tokens(e: &Env, to: &Address, amount: i128) {
    let total_share = get_total_synthetic_tokens(e);
    put_total_synthetic_tokens(e, total_share + (amount as u128));

    let token_client = StellarAssetClient::new(e, &get_sac_address(e));
    token_client.mint(to, &amount);
}
