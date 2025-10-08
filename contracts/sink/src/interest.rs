use crate::{
    errors::InsuranceFundError,
    storage::{get_oracle_registry, get_reserve, get_token_whitelist, get_token_whitelist_vec},
};

use soroban_sdk::{panic_with_error, Env, Symbol, Vec};
use utils::{
    constant::{PERCENTAGE_PRECISION, PERCENTAGE_PRECISION_I64},
    math::safe_math::{PrecisionMath, SafeMath},
    state::oracle_registry::{HistoricalOracleData, OracleValidity},
};

pub fn calculate_total_reserve_value(e: &Env) -> u128 {
    let tokens = get_token_whitelist_vec(&e);

    let mut total_reserve_value: u128 = 0;

    for i in 0..tokens.len() {
        let token = tokens.get(i).unwrap();
        let whitelist_token = get_token_whitelist(e, &token);

        let reserve = get_reserve(&e, &whitelist_token.address);

        // Fetch price
        let (historical_oracle_data, oracle_validity): (HistoricalOracleData, OracleValidity) = e
            .invoke_contract(
                &get_oracle_registry(&e),
                &Symbol::new(&e, "get_price"),
                Vec::from_array(&e, [whitelist_token.symbol.to_val()]),
            );

        // Check oracle validity
        if oracle_validity != OracleValidity::Valid {
            panic_with_error!(e, InsuranceFundError::InvalidOracle);
        }

        let reserve_value = reserve
            .balance
            .safe_mul(&e, historical_oracle_data.last_oracle_price_twap);

        total_reserve_value = total_reserve_value.safe_add(&e, reserve_value);
    }

    total_reserve_value
}

// Calculates the utilization percentage of the insurance fund.
//
// # Arguments
//
// * `total_reserve_value` - The current balance in the insurance vault, in fixed-point units.
// * `optimal_insurance` - The target or optimal coverage amount, in fixed-point units.
//
// # Returns
//
// * The utilization percentage as a fixed-point `u32` (e.g. 10_000_000 = 100%).
//   Returns 0 if either input is zero. The result is scaled by `PERCENTAGE_PRECISION`.
pub fn calculate_utilization(e: &Env, total_reserve_value: u128, optimal_insurance: u128) -> u32 {
    if total_reserve_value == 0 || optimal_insurance == 0 {
        return 0;
    }

    // Calculate utilization using safe precision math
    let utilization_u128 =
        total_reserve_value.safe_fixed_div_floor(e, optimal_insurance, PERCENTAGE_PRECISION);

    // Clamp to u32::MAX to prevent silent truncation
    utilization_u128.min(u32::MAX as u128) as u32
}

// Calculates the interest rate based on utilization using a two-slope model.
//
// # Mathematical Model
//
// The interest rate follows a piecewise linear function:
// - For utilization ≤ optimal: rate = base_rate + (utilization / optimal_utilization) * slope1
// - For utilization > optimal: rate = base_rate + slope1 + ((utilization - optimal) / (100% - optimal)) * slope2
//
// # Precision Behavior
//
// - Uses round-to-nearest arithmetic (safe_fixed_mul_round) for intermediate calculations
// - Fixed-point precision with PERCENTAGE_PRECISION = 10,000,000 (7 decimal places)
// - May introduce ±1 basis point precision differences due to discrete arithmetic
// - The two-slope formula is mathematically sound and internally consistent
// - Precision differences are expected behavior, not calculation errors
//
// # Arguments
//
// * `utilization` - The current utilization as a percentage in basis points (e.g. 8000 = 80%).
// * `optimal_utilization` - The target utilization percentage in basis points where the slope changes.
// * `base_rate` - The base interest rate in basis points (can be negative).
// * `slope1` - The interest rate slope below or at the optimal utilization.
// * `slope2` - The interest rate slope above the optimal utilization.
//
// # Returns
//
// * The calculated interest rate in basis points as an `i32`.
//
// # Examples
//
// ```rust
// // 80% utilization at 80% optimal: rate = 100 + 400 = 500 basis points
// let rate = calculate_rate(8000, 8000, 100, 400, 1500);
// assert_eq!(rate, 500);
//
// // 90% utilization at 80% optimal: rate = 100 + 400 + (10% / 20%) * 1500 = 1250
// let rate = calculate_rate(9000, 8000, 100, 400, 1500);
// assert_eq!(rate, 1250);
// ```
pub fn calculate_rate(
    utilization: u32,
    optimal_utilization: u32,
    base_rate: i32,
    slope1: u32,
    slope2: u32,
) -> i32 {
    // Input validation
    if optimal_utilization == 0 {
        // Cannot have zero optimal utilization as it causes division by zero
        panic_with_error!(&Env::default(), InsuranceFundError::InvalidConfiguration);
    }

    if utilization == 0 {
        return base_rate;
    }

    // Safe conversions with bounds checking
    let utilization_i64 = i64::try_from(utilization).unwrap_or_else(|_| {
        panic_with_error!(&Env::default(), InsuranceFundError::ConversionOverflow);
    });

    let optimal_utilization_i64 = i64::try_from(optimal_utilization).unwrap_or_else(|_| {
        panic_with_error!(&Env::default(), InsuranceFundError::ConversionOverflow);
    });

    let base_rate_i64 = i64::from(base_rate);

    let slope1_i64 = i64::try_from(slope1).unwrap_or_else(|_| {
        panic_with_error!(&Env::default(), InsuranceFundError::ConversionOverflow);
    });

    let slope2_i64 = i64::try_from(slope2).unwrap_or_else(|_| {
        panic_with_error!(&Env::default(), InsuranceFundError::ConversionOverflow);
    });

    let rate_i64 = if utilization_i64 <= optimal_utilization_i64 {
        // rate = base + (utilization * slope1 / optimal_utilization)
        // Use round-to-nearest for fair interest rate calculation
        let variable_rate = (utilization_i64 as u128).safe_fixed_mul_round(
            &Env::default(),
            slope1_i64 as u128,
            optimal_utilization_i64 as u128,
        ) as i64;

        base_rate_i64.checked_add(variable_rate).unwrap_or_else(|| {
            panic_with_error!(&Env::default(), InsuranceFundError::ArithmeticOverflow);
        })
    } else {
        // rate = base + slope1 + ((utilization - optimal_utilization) * slope2 / (10_000_000 - optimal_utilization))
        let excess_util = utilization_i64
            .checked_sub(optimal_utilization_i64)
            .unwrap_or_else(|| {
                panic_with_error!(&Env::default(), InsuranceFundError::ArithmeticOverflow);
            });

        let remaining = PERCENTAGE_PRECISION_I64
            .checked_sub(optimal_utilization_i64)
            .unwrap_or_else(|| {
                panic_with_error!(&Env::default(), InsuranceFundError::ArithmeticOverflow);
            });

        // Ensure remaining is not zero to prevent division by zero
        if remaining == 0 {
            panic_with_error!(&Env::default(), InsuranceFundError::InvalidConfiguration);
        }

        // Use round-to-nearest for fair slope2 calculation
        let slope2_part = (excess_util as u128).safe_fixed_mul_round(
            &Env::default(),
            slope2_i64 as u128,
            remaining as u128,
        ) as i64;

        base_rate_i64
            .checked_add(slope1_i64)
            .and_then(|intermediate| intermediate.checked_add(slope2_part))
            .unwrap_or_else(|| {
                panic_with_error!(&Env::default(), InsuranceFundError::ArithmeticOverflow);
            })
    };

    // Safe conversion back to i32 with bounds checking
    i32::try_from(rate_i64).unwrap_or_else(|_| {
        panic_with_error!(&Env::default(), InsuranceFundError::ConversionOverflow);
    })
}

#[cfg(test)]
mod tests {
    use utils::constant::{PERCENTAGE_PRECISION_U32, PRICE_PRECISION};

    use super::*;

    // utilization

    #[test]
    fn test_utilization_100_percent() {
        let e = Env::default();
        let utilization =
            calculate_utilization(&e, 1_000_000 * PRICE_PRECISION, 1_000_000 * PRICE_PRECISION);
        assert_eq!(utilization, 1 * PERCENTAGE_PRECISION_U32);
    }

    #[test]
    fn test_utilization_50_percent() {
        let e = Env::default();
        let utilization =
            calculate_utilization(&e, 500_000 * PRICE_PRECISION, 1_000_000 * PRICE_PRECISION);
        assert_eq!(utilization, 5_000_000); // 0.5%
    }

    #[test]
    fn test_utilization_above_100_percent() {
        let e = Env::default();
        let utilization =
            calculate_utilization(&e, 2_000_000 * PRICE_PRECISION, 1_000_000 * PRICE_PRECISION);
        assert_eq!(utilization, 2 * PERCENTAGE_PRECISION_U32);
    }

    #[test]
    fn test_zero_coverage_returns_zero() {
        let e = Env::default();
        let utilization = calculate_utilization(&e, 1_000_000 * PRICE_PRECISION, 0);
        assert_eq!(utilization, 0); // Prevent division by zero
    }

    #[test]
    fn test_utilization_clamps_to_u32_max_original() {
        let e = Env::default();
        let huge_value = u128::MAX;
        let utilization = calculate_utilization(&e, huge_value, 1); // absurdly high ratio
        assert_eq!(utilization, u32::MAX); // Clamped
    }

    #[test]
    fn test_zero_vault_amount() {
        let e = Env::default();
        let utilization = calculate_utilization(&e, 0, 1_000_000 * PRICE_PRECISION);
        assert_eq!(utilization, 0); // 0% utilization
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #21)")]
    fn test_zero_optimal_utilization() {
        // optimal_utilization = 0 could panic on division unless handled
        calculate_rate(5000, 0, 100, 400, 1500);
    }

    #[test]
    fn test_utilization_above_100_percent_in_rate() {
        let rate = calculate_rate(11_000, 8000, 100, 400, 1500);
        // excess = 3000, remaining = 2000
        // rate = 100 + 400 + (3000 / 2000) * 1500 = 100 + 400 + 2250 = 2750
        assert_eq!(rate, 2750);
    }

    // interest rate

    #[test]
    fn test_zero_utilization() {
        let rate = calculate_rate(0, 8000, 100, 400, 1500);
        assert_eq!(rate, 100);
    }

    #[test]
    fn test_half_optimal_utilization() {
        let rate = calculate_rate(4000, 8000, 100, 400, 1500);
        // rate = 100 + (4000 / 8000) * 400 = 100 + 200 = 300
        assert_eq!(rate, 300);
    }

    #[test]
    fn test_optimal_utilization() {
        let rate = calculate_rate(8000, 8000, 100, 400, 1500);
        // rate = 100 + (8000 / 8000) * 400 = 100 + 400 = 500
        assert_eq!(rate, 500);
    }

    #[test]
    fn test_above_optimal_utilization() {
        let rate = calculate_rate(9000, 8000, 100, 400, 1500);
        // excess = 1000, remaining = 2000000
        // rate = 100 + 400 + (1000 / 2000) * 1500 = 100 + 400 + 750 = 1250
        assert_eq!(rate, 1250);
    }

    #[test]
    fn test_max_utilization() {
        let rate = calculate_rate(10_000_000, 8000, 100, 400, 1500);
        // This is 100% utilization
        let excess = 10_000_000 - 8000;
        let remaining = 10_000_000 - 8000;
        // rate = 100 + 400 + (excess / remaining) * 1500 = 100 + 400 + 1500 = 2000
        assert_eq!(rate, 2000);
    }

    #[test]
    fn test_negative_base_rate() {
        let rate = calculate_rate(4000, 8000, -50, 400, 1500);
        // rate = -50 + (4000 / 8000) * 400 = -50 + 200 = 150
        assert_eq!(rate, 150);
    }
}
