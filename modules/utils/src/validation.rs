use soroban_sdk::{ panic_with_error, Env, Vec };

use crate::{ constant::PERCENTAGE_PRECISION_I32, errors::validation_errors::ValidationError };

pub fn validate_percentages(e: &Env, values: &Vec<i32>) {
    for value in values.iter() {
        if value > PERCENTAGE_PRECISION_I32 || value < -PERCENTAGE_PRECISION_I32 {
            panic_with_error!(e, ValidationError::InvalidPercentage);
        }
    }
}
