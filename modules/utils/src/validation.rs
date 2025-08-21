use soroban_sdk::{Env, panic_with_error};

/// Validates that a denominator value is positive (greater than 0).
/// 
/// This prevents negative values from being cast to unsigned integers in division operations,
/// which would create extremely large denominators and cause division results to approach zero.
/// 
/// # Arguments
/// * `e` - The Soroban environment
/// * `value` - The denominator value to validate
/// * `error` - The error to panic with if validation fails
/// 
/// # Panics
/// Panics with the provided error if the value is negative.
/// Note: Zero values are allowed as they are typically handled separately with default fallbacks.
pub fn validate_positive_denominator<E>(e: &Env, value: u64, error: E) 
where
    E: Into<soroban_sdk::Error>,
{
    if value <= 0 {
        panic_with_error!(e, error);
    }
}
