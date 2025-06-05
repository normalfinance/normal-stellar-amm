use soroban_sdk::{ Address, Env, Vec };
use soroban_sdk::token::Client as SorobanTokenClient;

pub fn validate_tokens_contracts(e: &Env, tokens: &Vec<Address>) {
    // call token contract to check if token exists & it's alive
    for token in tokens.iter() {
        SorobanTokenClient::new(e, &token).balance(&e.current_contract_address());
    }
}

pub fn transfer_token(e: &Env, token: &Address, from: &Address, to: &Address, amount: &i128) {
    let token_client = SorobanTokenClient::new(e, token);
    token_client.transfer(from, to, amount);
}
