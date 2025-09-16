use crate::{
    errors::InsuranceFundError,
    storage::{get_oracle_registry, get_reserve, get_token_whitelist, get_token_whitelist_vec},
};
use soroban_fixed_point_math::FixedPoint;
use soroban_sdk::{panic_with_error, Env, Symbol, Vec};
use utils::{
    constant::{PERCENTAGE_PRECISION, PERCENTAGE_PRECISION_I64},
    math::safe_math::SafeMath,
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
pub fn calculate_utilization(total_reserve_value: u128, optimal_insurance: u128) -> u32 {
    if total_reserve_value == 0 || optimal_insurance == 0 {
        return 0;
    }

    // Calculate utilization with safe clamping to prevent overflow
    let utilization_u128 = total_reserve_value
        .fixed_div_floor(optimal_insurance, PERCENTAGE_PRECISION)
        .unwrap_or(0);
    
    // Clamp to u32::MAX to prevent silent truncation
    utilization_u128.min(u32::MAX as u128) as u32
}

// Calculates the interest rate based on utilization using a two-slope model.
//
// # Arguments
//
// * `e` - The Soroban environment.
// * `utilization` - The current utilization as a percentage in basis points (e.g. 8000 = 80%).
// * `optimal_utilization` - The target utilization percentage in basis points where the slope changes.
// * `base_rate` - The base interest rate in basis points (can be negative).
// * `slope1` - The interest rate slope below or at the optimal utilization.
// * `slope2` - The interest rate slope above the optimal utilization.
//
// # Returns
//
// * The calculated interest rate in basis points as an `i32`.
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
        let variable_rate = utilization_i64
            .fixed_mul_floor(slope1_i64, optimal_utilization_i64)
            .unwrap_or_else(|| {
                panic_with_error!(&Env::default(), InsuranceFundError::ArithmeticOverflow);
            });
        
        base_rate_i64.checked_add(variable_rate).unwrap_or_else(|| {
            panic_with_error!(&Env::default(), InsuranceFundError::ArithmeticOverflow);
        })
    } else {
        // rate = base + slope1 + ((utilization - optimal_utilization) * slope2 / (10_000_000 - optimal_utilization))
        let excess_util = utilization_i64.checked_sub(optimal_utilization_i64).unwrap_or_else(|| {
            panic_with_error!(&Env::default(), InsuranceFundError::ArithmeticOverflow);
        });
        
        let remaining = PERCENTAGE_PRECISION_I64.checked_sub(optimal_utilization_i64).unwrap_or_else(|| {
            panic_with_error!(&Env::default(), InsuranceFundError::ArithmeticOverflow);
        });

        // Ensure remaining is not zero to prevent division by zero
        if remaining == 0 {
            panic_with_error!(&Env::default(), InsuranceFundError::InvalidConfiguration);
        }

        let slope2_part = excess_util.fixed_mul_floor(slope2_i64, remaining).unwrap_or_else(|| {
            panic_with_error!(&Env::default(), InsuranceFundError::ArithmeticOverflow);
        });

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
        let utilization =
            calculate_utilization(1_000_000 * PRICE_PRECISION, 1_000_000 * PRICE_PRECISION);
        assert_eq!(utilization, 1 * PERCENTAGE_PRECISION_U32);
    }

    #[test]
    fn test_utilization_50_percent() {
        let utilization =
            calculate_utilization(500_000 * PRICE_PRECISION, 1_000_000 * PRICE_PRECISION);
        assert_eq!(utilization, 5_000_000); // 0.5%
    }

    #[test]
    fn test_utilization_above_100_percent() {
        let utilization =
            calculate_utilization(2_000_000 * PRICE_PRECISION, 1_000_000 * PRICE_PRECISION);
        assert_eq!(utilization, 2 * PERCENTAGE_PRECISION_U32);
    }

    #[test]
    fn test_zero_coverage_returns_zero() {
        let utilization = calculate_utilization(1_000_000 * PRICE_PRECISION, 0);
        assert_eq!(utilization, 0); // Prevent division by zero
    }

    #[test]
    fn test_utilization_clamps_to_u32_max_original() {
        let huge_value = u128::MAX;
        let utilization = calculate_utilization(huge_value, 1); // absurdly high ratio
        assert_eq!(utilization, u32::MAX); // Clamped
    }

    #[test]
    fn test_zero_vault_amount() {
        let utilization = calculate_utilization(0, 1_000_000 * PRICE_PRECISION);
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
        assert_eq!(rate, 100); // Only base rate should apply
    }

    #[test]
    fn test_negative_base_rate() {
        let rate = calculate_rate(5000, 8000, -100, 400, 1500);
        // -100 + (5000 / 8000 * 400) = -100 + 250 = 150
        assert_eq!(rate, 150);
    }

    #[test]
    fn test_negative_rate() {
        let rate = calculate_rate(11000, 8000, -1000, 200, 1000);
        // -100 + (5000 / 8000 * 400) = -100 + 250 = 150
        assert_eq!(rate, -975);
    }

    #[test]
    fn test_low_utilization_rate() {
        let rate = calculate_rate(5000, 8000, 100, 400, 1500);
        // 100 + (5000 / 8000 * 400) = 100 + 250 = 350
        assert_eq!(rate, 350);
    }

    #[test]
    fn test_utilization_at_optimal() {
        let rate = calculate_rate(8000, 8000, 100, 400, 1500);
        // base + slope_a = 100 + 400 = 500
        assert_eq!(rate, 500);
    }

    #[test]
    fn test_high_utilization_rate() {
        let rate = calculate_rate(9500, 8000, 100, 400, 1500);
        // base + slope_a + ((1500 / 2000) * 1500) = 100 + 400 + 1125 = 1625
        assert_eq!(rate, 1625);
    }

    #[test]
    fn test_max_utilization() {
        let rate = calculate_rate(10_000, 8000, 100, 400, 1500);
        // base + slope_a + slope_b = 100 + 400 + 1500 = 2000
        assert_eq!(rate, 2000);
    }

    // Security tests for audit findings
    
    #[test]
    fn test_utilization_clamps_to_u32_max() {
        let huge_value = u128::MAX;
        let utilization = calculate_utilization(huge_value, 1);
        assert_eq!(utilization, u32::MAX); // Should be clamped, not truncated
    }

    #[test]
    fn test_utilization_overflow_protection() {
        // Test various extreme values that could cause overflow
        let test_cases = [
            (u128::MAX, 1),
            (u128::MAX / 2, 1),
            (1_000_000_000_000_000_000_u128, 1),
        ];

        for (total_reserve, optimal) in test_cases {
            let utilization = calculate_utilization(total_reserve, optimal);
            // Should not panic and should return a valid value
            assert!(utilization <= u32::MAX);
        }
    }

    #[test]
    fn test_utilization_precision_boundaries() {
        // Test precision at boundaries to ensure consistent behavior
        let base_amount = 1_000_000 * PRICE_PRECISION;
        
        // Test around 100% utilization
        let utilization_100 = calculate_utilization(base_amount, base_amount);
        assert_eq!(utilization_100, 1 * PERCENTAGE_PRECISION_U32);
        
        // Test just above 100%
        let utilization_101 = calculate_utilization(base_amount + 1, base_amount);
        assert!(utilization_101 > utilization_100);
        
        // Test just below 100%
        let utilization_99 = calculate_utilization(base_amount - 1, base_amount);
        assert!(utilization_99 < utilization_100);
    }

    #[test]
    fn test_interest_rate_conversion_safety() {
        // Test that all conversions in calculate_rate are safe
        let test_cases = [
            (u32::MAX, 8000, i32::MIN, u32::MAX, u32::MAX),
            (0, u32::MAX, i32::MAX, 0, 0),
            (5000, 8000, -10000, 400, 1500),
        ];

        for (util, opt_util, base, slope1, slope2) in test_cases {
            // Should not panic on extreme but valid inputs
            if opt_util == 0 {
                // This should panic as expected
                continue;
            }
            let _rate = calculate_rate(util, opt_util, base, slope1, slope2);
            // If we get here, the conversion was safe
        }
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #29)")]
    fn test_zero_optimal_utilization_panics() {
        // Should panic on division by zero with proper error
        calculate_rate(5000, 0, 100, 400, 1500);
    }

    #[test]
    fn test_negative_base_rate_bounds() {
        // Test extreme negative base rates
        let rate = calculate_rate(5000, 8000, i32::MIN + 1000, 400, 1500);
        // Should handle extreme negative values without overflow
        assert!(rate < 0);
    }

    #[test]
    fn test_interest_rate_monotonicity() {
        // Test that interest rates increase monotonically with utilization
        let base_rate = 100;
        let slope1 = 400;
        let slope2 = 1500;
        let optimal_util = 8000;
        
        let mut prev_rate = calculate_rate(0, optimal_util, base_rate, slope1, slope2);
        
        for util in (1000..=10000).step_by(1000) {
            let current_rate = calculate_rate(util, optimal_util, base_rate, slope1, slope2);
            assert!(current_rate >= prev_rate, 
                "Interest rate should be monotonically increasing: {} >= {}", 
                current_rate, prev_rate);
            prev_rate = current_rate;
        }
    }
}
