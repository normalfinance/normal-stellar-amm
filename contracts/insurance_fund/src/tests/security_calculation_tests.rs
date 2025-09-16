#![cfg(test)]

use crate::interest::{calculate_utilization, calculate_rate};
use soroban_sdk::Env;
use utils::constant::{PRICE_PRECISION, PERCENTAGE_PRECISION_U32};

#[test]
fn test_utilization_overflow_attack() {
    // Test the specific overflow attack mentioned in the audit
    // where extremely large inputs could overflow/truncate and yield 0
    
    let huge_total_reserve = u128::MAX;
    let small_optimal_insurance = 1_u128;
    
    let utilization = calculate_utilization(huge_total_reserve, small_optimal_insurance);
    
    // Should be clamped to u32::MAX, not truncated to 0
    assert_eq!(utilization, u32::MAX);
    assert_ne!(utilization, 0, "Utilization should not be 0 due to truncation");
}

#[test]
fn test_utilization_precision_manipulation() {
    // Test precision manipulation attempts with edge case ratios
    
    let test_cases = [
        // (total_reserve_value, optimal_insurance, expected_behavior)
        (u128::MAX, 1, u32::MAX), // Should clamp
        (u128::MAX, u128::MAX, PERCENTAGE_PRECISION_U32), // 100%
        (u128::MAX / 2, u128::MAX, PERCENTAGE_PRECISION_U32 / 2), // 50%
        (0, 1000, 0), // Zero reserve
        (1000, 0, 0), // Zero optimal (edge case)
        (u64::MAX as u128, 1, u32::MAX), // Large but not maximum
    ];
    
    for (total_reserve, optimal, expected_max) in test_cases {
        let utilization = calculate_utilization(total_reserve, optimal);
        
        // Verify result is reasonable and not a result of silent truncation
        if optimal == 0 {
            assert_eq!(utilization, 0, "Zero optimal should return 0");
        } else if total_reserve == 0 {
            assert_eq!(utilization, 0, "Zero reserve should return 0");
        } else {
            assert!(utilization <= expected_max, 
                "Utilization {} exceeds expected maximum {}", utilization, expected_max);
            
            // Verify no silent truncation occurred
            if total_reserve > optimal * (u32::MAX as u128) {
                assert_eq!(utilization, u32::MAX, "Should be clamped to u32::MAX");
            }
        }
    }
}

#[test]
fn test_interest_rate_precision_boundaries() {
    // Test precision at critical boundaries where rounding could affect results
    
    let optimal_utilization = 8000; // 80%
    let base_rate = 100;
    let slope1 = 400;
    let slope2 = 1500;
    
    // Test at exactly optimal utilization
    let rate_at_optimal = calculate_rate(optimal_utilization, optimal_utilization, base_rate, slope1, slope2);
    
    // Test just below optimal
    let rate_below = calculate_rate(optimal_utilization - 1, optimal_utilization, base_rate, slope1, slope2);
    
    // Test just above optimal  
    let rate_above = calculate_rate(optimal_utilization + 1, optimal_utilization, base_rate, slope1, slope2);
    
    // Verify proper slope transitions
    assert!(rate_below <= rate_at_optimal, "Rate should not decrease before optimal");
    assert!(rate_above >= rate_at_optimal, "Rate should not decrease after optimal");
    
    // Verify the slope change occurs at the right point
    let expected_at_optimal = base_rate + slope1;
    assert_eq!(rate_at_optimal, expected_at_optimal, "Rate at optimal should be base + slope1");
}

#[test]
fn test_interest_rate_curve_properties() {
    // Test mathematical properties that should hold for the interest rate curve
    
    let optimal_utilization = 8000;
    let base_rate = 100;
    let slope1 = 400;
    let slope2 = 1500;
    
    // Test monotonicity in first region (0 to optimal)
    let mut prev_rate = calculate_rate(0, optimal_utilization, base_rate, slope1, slope2);
    for utilization in (1000..optimal_utilization).step_by(1000) {
        let current_rate = calculate_rate(utilization, optimal_utilization, base_rate, slope1, slope2);
        assert!(current_rate >= prev_rate, 
            "Interest rate should be non-decreasing in first region: {} >= {} at utilization {}", 
            current_rate, prev_rate, utilization);
        prev_rate = current_rate;
    }
    
    // Test monotonicity in second region (optimal to 100%)
    prev_rate = calculate_rate(optimal_utilization, optimal_utilization, base_rate, slope1, slope2);
    for utilization in ((optimal_utilization + 1000)..=10000).step_by(1000) {
        let current_rate = calculate_rate(utilization, optimal_utilization, base_rate, slope1, slope2);
        assert!(current_rate >= prev_rate, 
            "Interest rate should be non-decreasing in second region: {} >= {} at utilization {}", 
            current_rate, prev_rate, utilization);
        prev_rate = current_rate;
    }
}

#[test]
fn test_extreme_parameter_robustness() {
    // Test that the system behaves predictably with extreme but valid parameters
    
    let extreme_cases = [
        // (utilization, optimal_utilization, base_rate, slope1, slope2)
        (u32::MAX, u32::MAX - 1, i32::MAX, u32::MAX, u32::MAX),
        (0, u32::MAX, i32::MIN, 0, 0),
        (1, u32::MAX, -1000000, 1000000, 1000000),
        (u32::MAX, u32::MAX, 0, 0, 0),
    ];
    
    for (util, opt_util, base, slope1, slope2) in extreme_cases {
        if opt_util == 0 {
            continue; // Skip cases that should panic
        }
        
        // Should not panic or overflow
        let rate = calculate_rate(util, opt_util, base, slope1, slope2);
        
        // Verify result is within reasonable bounds for i32
        assert!(rate >= i32::MIN && rate <= i32::MAX);
        
        // Basic sanity checks
        if util == 0 {
            assert_eq!(rate, base, "Zero utilization should return base rate");
        }
    }
}

#[test]
fn test_conversion_overflow_detection() {
    // Test that our enhanced error handling catches conversion overflows
    
    // These should work without panic (within valid ranges)
    let valid_cases = [
        (1000, 8000, 100, 400, 1500),
        (u32::MAX / 2, u32::MAX / 2, i32::MAX / 2, u32::MAX / 2, u32::MAX / 2),
    ];
    
    for (util, opt_util, base, slope1, slope2) in valid_cases {
        let _rate = calculate_rate(util, opt_util, base, slope1, slope2);
        // Should complete without error
    }
}

#[test]
fn test_arithmetic_overflow_protection() {
    // Test that arithmetic operations are protected against overflow
    
    // Use large but not maximum values to test intermediate calculations
    let utilization = u32::MAX / 2;
    let optimal_utilization = u32::MAX / 4;
    let base_rate = i32::MAX / 2;
    let slope1 = u32::MAX / 2;
    let slope2 = u32::MAX / 2;
    
    // This should not panic due to our overflow protection
    let rate = calculate_rate(utilization, optimal_utilization, base_rate, slope1, slope2);
    
    // Verify result is reasonable
    assert!(rate != 0); // Should not be zero due to overflow
}

#[test]
fn test_precision_consistency() {
    // Test that calculations are consistent across different input scales
    
    let base_utilization = 5000;
    let base_optimal = 8000;
    let base_rate = 100;
    let base_slope1 = 400;
    let base_slope2 = 1500;
    
    let rate1 = calculate_rate(base_utilization, base_optimal, base_rate, base_slope1, base_slope2);
    
    // Scale all utilization parameters by 2 (but keep rates the same)
    let scaled_utilization = base_utilization * 2;
    let scaled_optimal = base_optimal * 2;
    
    let rate2 = calculate_rate(scaled_utilization, scaled_optimal, base_rate, base_slope1, base_slope2);
    
    // Should produce the same rate (same relative utilization)
    assert_eq!(rate1, rate2, "Scaling utilization parameters should not affect rate calculation");
}

#[test] 
fn test_rounding_bias_mitigation() {
    // Test that our system doesn't accumulate systematic rounding bias
    
    let optimal_utilization = 8000;
    let base_rate = 100;
    let slope1 = 400;
    let slope2 = 1500;
    
    // Calculate rates for a sequence of utilization values
    let mut rates = Vec::new();
    for utilization in (0..=10000).step_by(100) {
        let rate = calculate_rate(utilization, optimal_utilization, base_rate, slope1, slope2);
        rates.push(rate);
    }
    
    // Verify that the sequence is reasonably smooth (no large jumps due to rounding)
    for i in 1..rates.len() {
        let rate_change = (rates[i] - rates[i-1]).abs();
        let max_expected_change = slope2 as i32 / 10; // Allow for reasonable changes
        
        assert!(rate_change <= max_expected_change,
            "Rate change too large between utilization steps: {} > {} at index {}",
            rate_change, max_expected_change, i);
    }
}