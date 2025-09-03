use soroban_sdk::{
    token::{StellarAssetClient, TokenClient},
    xdr::ToXdr,
    Address, Bytes, BytesN, Env,
};
use utils::token::transfer_token;

use crate::storage::{get_token_a, get_token_b, get_total_synthetic_tokens, set_total_synthetic_tokens};

pub fn create_lp_token_contract(
    e: &Env,
    token_wasm_hash: BytesN<32>,
    token_a: &Address,
    token_b: &Address,
) -> Address {
    let mut salt = Bytes::new(e);
    salt.append(&token_a.to_xdr(e));
    salt.append(&token_b.to_xdr(e));
    let salt = e.crypto().sha256(&salt);
    e.deployer()
        .with_current_contract(salt)
        .deploy_v2(token_wasm_hash, ())
}

pub fn transfer_a(e: &Env, to: &Address, amount: u128) {
    transfer_token(
        e,
        &get_token_a(e),
        &e.current_contract_address(),
        &to,
        &(amount as i128),
    );
}

pub fn transfer_b(e: &Env, to: &Address, amount: u128) {
    transfer_token(
        e,
        &get_token_b(e),
        &e.current_contract_address(),
        &to,
        &(amount as i128),
    );
}

pub fn get_user_balance_synthetic(e: &Env, user: &Address) -> u128 {
    TokenClient::new(e, &get_token_a(e)).balance(user) as u128
}

pub fn burn_synthetic_tokens(e: &Env, from: &Address, amount: u128) {
    let total_share = get_total_synthetic_tokens(e);
    set_total_synthetic_tokens(e, &(total_share - amount));

    let token_client = StellarAssetClient::new(e, &get_token_a(e));
    token_client.clawback(from, &(amount as i128));
}

pub fn mint_synthetic_tokens(e: &Env, to: &Address, amount: i128) {
    let total_share = get_total_synthetic_tokens(e);
    set_total_synthetic_tokens(e, &(total_share + (amount as u128)));

    let token_client = StellarAssetClient::new(e, &get_token_a(e));
    token_client.mint(to, &amount);
}
