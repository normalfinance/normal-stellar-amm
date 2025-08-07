use crate::storage::get_pool;
use pool_tokens::get_token_synthetic;
use soroban_sdk::{xdr::ToXdr, Address, Bytes, BytesN, Env, Symbol};
use utils::token::transfer_token;

pub fn create_synthetic_token_contract(
    e: &Env,
    token_wasm_hash: BytesN<32>,
    asset: &Symbol,
) -> Address {
    let mut salt = Bytes::new(e);
    salt.append(&asset.to_xdr(e));
    let salt = e.crypto().sha256(&salt);
    e.deployer()
        .with_current_contract(salt)
        .deploy_v2(token_wasm_hash, ())
}

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
        &get_token_synthetic(e),
        &e.current_contract_address(),
        &to,
        &(amount as i128),
    );
}

pub fn transfer_b(e: &Env, to: &Address, amount: u128) {
    let pool = get_pool(e);
    transfer_token(
        e,
        &pool.token_b,
        &e.current_contract_address(),
        &to,
        &(amount as i128),
    );
}
