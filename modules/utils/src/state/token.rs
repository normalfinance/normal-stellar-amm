use soroban_sdk::{contracttype, Address, BytesN, String, Symbol};

#[contracttype]
#[derive(Clone)]
pub struct TokenInitInfo {
    // The hash of the liquidity pool token contract.
    pub token_wasm_hash: BytesN<32>,
    pub name: String,
    pub symbol: String,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AddressAndAmount {
    // Address of the asset
    pub address: Address,
    // The total amount of those tokens in the pool
    pub amount: u128,
}
