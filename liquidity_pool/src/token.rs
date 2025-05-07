use crate::storage::{get_token_a, get_token_b};
use soroban_sdk::token::{StellarAssetClient as AssetClient, TokenClient as Client};
use soroban_sdk::{xdr::ToXdr, Address, Bytes, BytesN, Env};

pub fn create_contract(
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

fn transfer(e: &Env, token: Address, to: &Address, amount: i128) {
    Client::new(e, &token).transfer(&e.current_contract_address(), to, &amount);
}

pub fn transfer_a(e: &Env, to: &Address, amount: u128) {
    transfer(e, get_token_a(e), to, amount as i128);
}

pub fn transfer_b(e: &Env, to: &Address, amount: u128) {
    transfer(e, get_token_b(e), to, amount as i128);
}

pub fn mint_a(e: &Env, to: &Address, amount: u128) {
    AssetClient::new(e, &get_token_a(e)).mint(to, &(amount as i128));
}

pub fn burn_a(e: &Env, from: &Address, amount: u128) {
    Client::new(e, &get_token_a(e)).burn(from, &(amount as i128));
}
