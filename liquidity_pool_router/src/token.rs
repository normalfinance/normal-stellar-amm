use sep_40_oracle::Asset;
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

pub fn create_synthetic_token_contract(
    e: &Env,
    token_wasm_hash: BytesN<32>,
    asset: &Asset,
) -> Address {
    let mut salt = Bytes::new(e);
    salt.append(&asset.clone().to_xdr(e));
    let salt = e.crypto().sha256(&salt);
    e.deployer()
        .with_current_contract(salt)
        .deploy_v2(token_wasm_hash, ())
}
