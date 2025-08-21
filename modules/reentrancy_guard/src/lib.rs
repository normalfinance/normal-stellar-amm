#![no_std]

use soroban_sdk::{contracttype, panic_with_error, Env};
use utils::errors::validation_errors::ValidationError;

#[derive(Clone)]
#[contracttype]
enum DataKey {
    ReentrancyGuard,
}

pub fn enter(e: &Env) {
    if e.storage()
        .instance()
        .get(&DataKey::ReentrancyGuard)
        .unwrap_or(false)
    {
        panic_with_error!(e, ValidationError::Reentrancy);
    }
    e.storage().instance().set(&DataKey::ReentrancyGuard, &true);
}
pub fn exit(e: &Env) {
    e.storage()
        .instance()
        .set(&DataKey::ReentrancyGuard, &false);
}
