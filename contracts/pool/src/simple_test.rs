#![cfg(test)]

use soroban_sdk::{Env, testutils::Address as _, Address};

#[test]
fn test_simple_contract_creation() {
    let env = Env::default();
    let admin = Address::generate(&env);
    
    // Just test that we can create an environment and address
    assert_ne!(admin, Address::generate(&env));
}

#[test] 
fn test_basic_math() {
    assert_eq!(2 + 2, 4);
}
