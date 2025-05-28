use crate::storage::{ get_pool };
use soroban_sdk::token::TokenClient as Client;
use soroban_sdk::{ xdr::ToXdr, Address, Bytes, BytesN, Env };

pub fn create_contract(
    e: &Env,
    token_wasm_hash: BytesN<32>,
    token_a: &Address,
    token_b: &Address
) -> Address {
    let mut salt = Bytes::new(e);
    salt.append(&token_a.to_xdr(e));
    salt.append(&token_b.to_xdr(e));
    let salt = e.crypto().sha256(&salt);
    e.deployer().with_current_contract(salt).deploy_v2(token_wasm_hash, ())
}

fn transfer(e: &Env, token: Address, to: &Address, amount: i128) {
    Client::new(e, &token).transfer(&e.current_contract_address(), to, &amount);
}

pub fn transfer_a(e: &Env, to: &Address, amount: u128) {
    // TODO: add authorization - i.e. e.require_auth(&e.current_contract_address());
    // TODO: Validate that `amount <= i128::MAX` before casting or use a checked conversion (e.g. `TryFrom`) to prevent overflow and reject out-of-range values.
    let pool = get_pool(e);
    transfer(e, pool.token_a, to, amount as i128);
}

pub fn transfer_b(e: &Env, to: &Address, amount: u128) {
    // TODO: add authorization - i.e. e.require_auth(&e.current_contract_address());
    // TODO: Validate that `amount <= i128::MAX` before casting or use a checked conversion (e.g. `TryFrom`) to prevent overflow and reject out-of-range values.
    let pool = get_pool(e);
    transfer(e, pool.token_b, to, amount as i128);
}
