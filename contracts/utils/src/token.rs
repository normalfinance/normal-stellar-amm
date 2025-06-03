use soroban_sdk::{ Address, Env, Vec };
use soroban_sdk::token::Client as SorobanTokenClient;

pub fn validate_tokens_contracts(e: &Env, tokens: &Vec<Address>) {
    // call token contract to check if token exists & it's alive
    for token in tokens.iter() {
        SorobanTokenClient::new(e, &token).balance(&e.current_contract_address());
    }
}
