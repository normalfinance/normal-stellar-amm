use soroban_sdk::token::{ Client as SorobanTokenClient, TokenClient };
use soroban_sdk::{ panic_with_error, Address, Env, Vec };

use crate::errors::validation_errors::ValidationError;

pub fn validate_token_contract(e: &Env, token: &Address) {
    // call token contract to check if token exists & it's alive
    let result = SorobanTokenClient::new(e, &token).try_balance(&e.current_contract_address());

    if result.is_err() {
        panic_with_error!(e, ValidationError::InvalidToken);
    }
}

pub fn validate_token_contracts(e: &Env, tokens: &Vec<Address>) {
    for token in tokens.iter() {
        validate_token_contract(e, &token);
    }
}

pub fn transfer_token(e: &Env, token: &Address, from: &Address, to: &Address, amount: &i128) {
    SorobanTokenClient::new(e, token).transfer(from, to, amount);
}

pub fn transfer_token_from(
    e: &Env,
    token: &Address,
    spender: &Address,
    from: &Address,
    to: &Address,
    amount: &i128
) {
    SorobanTokenClient::new(e, token).transfer_from(spender, from, to, amount);
}

pub fn get_token_balance(e: &Env, token: &Address, account: &Address) -> i128 {
    SorobanTokenClient::new(&e, &token).balance(account)
}
