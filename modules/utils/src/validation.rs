use soroban_sdk::{ panic_with_error, Env };

use crate::errors::validation_errors::ValidationError;

pub fn check_positive_amount(e: &Env, amount: u128) {
    if amount == 0 {
        panic_with_error!(&e, ValidationError::AmountMustBePositive);
    }
}
