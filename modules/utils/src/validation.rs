use soroban_sdk::{panic_with_error, Env, Vec};

use crate::{constant::PERCENTAGE_PRECISION_I32, errors::validation_errors::ValidationError};

pub fn validate_percentages(e: &Env, values: &Vec<i32>) {
    for value in values.iter() {
        if value > PERCENTAGE_PRECISION_I32 || value < -PERCENTAGE_PRECISION_I32 {
            panic_with_error!(e, ValidationError::InvalidPercentage);
        }
    }
}

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

/// Validates that an amount is not zero, panicking with the specified error if it is.
///
/// This is a generic function that works with any numeric type that implements
/// `PartialEq` and can be converted from a `u8` zero value.
///
/// # Arguments
/// * `env` - The Soroban environment reference
/// * `amount` - The amount to validate
/// * `error` - The error to panic with if amount is zero
///
/// # Panics
/// Panics with the provided error if `amount == 0`
///
/// # Examples
/// ```rust
/// use utils::validation::ensure_non_zero;
/// use crate::errors::PoolValidationError;
///
/// ensure_non_zero(&e, token_amount, PoolValidationError::ZeroAmount);
/// ```
#[inline]
pub fn ensure_non_zero<T: PartialEq + From<u8>, E: Copy>(env: &Env, amount: T, error: E) {
    if amount == T::from(0u8) {
        panic_with_error!(env, error);
    }
}

/// Validates that a u128 amount is not zero, panicking with the specified error if it is.
///
/// This is a specialized version for u128 values, providing better ergonomics
/// and performance for the most common use case.
///
/// # Arguments
/// * `env` - The Soroban environment reference
/// * `amount` - The u128 amount to validate
/// * `error` - The error to panic with if amount is zero
///
/// # Panics
/// Panics with the provided error if `amount == 0`
///
/// # Examples
/// ```rust
/// use utils::validation::ensure_non_zero_u128;
/// use crate::errors::PoolValidationError;
///
/// ensure_non_zero_u128(&e, token_b_amount, PoolValidationError::ZeroAmount);
/// ```
#[inline]
pub fn ensure_non_zero_u128<E: Copy>(env: &Env, amount: u128, error: E) {
    if amount == 0 {
        panic_with_error!(env, error);
    }
}

/// Validates that an i128 amount is not zero, panicking with the specified error if it is.
///
/// This is a specialized version for i128 values, providing better ergonomics
/// and performance for signed integer use cases.
///
/// # Arguments
/// * `env` - The Soroban environment reference
/// * `amount` - The i128 amount to validate
/// * `error` - The error to panic with if amount is zero
///
/// # Panics
/// Panics with the provided error if `amount == 0`
///
/// # Examples
/// ```rust
/// use utils::validation::ensure_non_zero_i128;
/// use crate::errors::PoolValidationError;
///
/// ensure_non_zero_i128(&e, signed_amount, PoolValidationError::ZeroAmount);
/// ```
#[inline]
pub fn ensure_non_zero_i128<E: Copy>(env: &Env, amount: i128, error: E) {
    if amount == 0 {
        panic_with_error!(env, error);
    }
}
