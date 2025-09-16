#![cfg(test)]

use crate::pool::{get_delta_a, peg_price};
use soroban_sdk::{Env};
use utils::constant::{PRICE_PRECISION, PERCENTAGE_PRECISION};

#[test]
fn test_delta_a_precision_attack() {
    let e = Env::default();
    
    // Test small price changes near peg that could be exploited
    let base_reserve = 1_000_000 * PRICE_PRECISION;
    let quote_reserve = 1_000_000 * PRICE_PRECISION;
    
    // Base case - exactly at peg
    let base_oracle_price = PRICE_PRECISION;
    let quote_oracle_price = PRICE_PRECISION;
    let delta_baseline = get_delta_a(&e, base_reserve, quote_reserve, base_oracle_price, quote_oracle_price);
    
    // Test tiny price movement (0.01% change)
    let tiny_change = PRICE_PRECISION / 10_000; // 0.01%
    let delta_tiny = get_delta_a(&e, base_reserve, quote_reserve, base_oracle_price + tiny_change, quote_oracle_price);
    
    // The epsilon smoothing should prevent outsized changes for tiny price movements
    let delta_change = if delta_tiny > delta_baseline {
        delta_tiny - delta_baseline
    } else {
        delta_baseline - delta_tiny
    };
    
    // For a 0.01% price change, delta_a change should be minimal due to epsilon smoothing
    let max_expected_change = base_reserve / 10_000; // 0.01% of reserve
    assert!(delta_change <= max_expected_change as i128, 
        "Delta_A changed too much for tiny price movement: {} > {}", 
        delta_change, max_expected_change);
}

#[test]
fn test_delta_a_rounding_consistency() {
    let e = Env::default();
    
    // Test that sequential small changes don't accumulate bias
    let base_reserve = 1_000_000 * PRICE_PRECISION;
    let quote_reserve = 1_000_000 * PRICE_PRECISION;
    let base_oracle_price = PRICE_PRECISION;
    
    let mut cumulative_delta = 0i128;
    let price_step = PRICE_PRECISION / 1000; // 0.1% steps
    
    // Apply small sequential price changes
    for i in 1..=10 {
        let current_price = PRICE_PRECISION + (i * price_step);
        let delta = get_delta_a(&e, base_reserve, quote_reserve, base_oracle_price, current_price);
        cumulative_delta += delta;
    }
    
    // Compare with single large change
    let final_price = PRICE_PRECISION + (10 * price_step);
    let direct_delta = get_delta_a(&e, base_reserve, quote_reserve, base_oracle_price, final_price);
    
    // Cumulative and direct should be similar (within reasonable bounds due to epsilon smoothing)
    let difference = (cumulative_delta - direct_delta).abs();
    let tolerance = base_reserve as i128 / 1000; // 0.1% tolerance
    
    assert!(difference <= tolerance,
        "Rounding accumulation too large: cumulative={}, direct={}, diff={}",
        cumulative_delta, direct_delta, difference);
}

#[test]
fn test_peg_price_calculation_correctness() {
    let e = Env::default();
    
    // Test the corrected peg price calculation (quote/base)
    let test_cases = [
        (PRICE_PRECISION, PRICE_PRECISION, PRICE_PRECISION), // 1:1 ratio
        (PRICE_PRECISION, 2 * PRICE_PRECISION, 2 * PRICE_PRECISION), // 1:2 ratio
        (2 * PRICE_PRECISION, PRICE_PRECISION, PRICE_PRECISION / 2), // 2:1 ratio
    ];
    
    for (base_price, quote_price, expected_peg) in test_cases {
        let actual_peg = peg_price(&e, base_price, quote_price);
        assert_eq!(actual_peg, expected_peg,
            "Peg price calculation incorrect: base={}, quote={}, expected={}, actual={}",
            base_price, quote_price, expected_peg, actual_peg);
    }
}

#[test]
fn test_peg_price_division_order() {
    let e = Env::default();
    
    // Ensure we're calculating quote/base, not base/quote
    let base_oracle_price = PRICE_PRECISION; // $1
    let quote_oracle_price = 2 * PRICE_PRECISION; // $2
    
    let peg = peg_price(&e, base_oracle_price, quote_oracle_price);
    
    // Should be quote/base = 2/1 = 2
    assert_eq!(peg, 2 * PRICE_PRECISION);
    
    // Verify the inverse gives the reciprocal
    let inverse_peg = peg_price(&e, quote_oracle_price, base_oracle_price);
    assert_eq!(inverse_peg, PRICE_PRECISION / 2);
}

#[test]
fn test_delta_a_epsilon_threshold() {
    let e = Env::default();
    
    // Test that very small changes result in zero delta due to epsilon threshold
    let reserve_a = 1_000_000 * PRICE_PRECISION;
    let reserve_b = 1_000_000 * PRICE_PRECISION;
    let base_oracle_price = PRICE_PRECISION;
    let quote_oracle_price = PRICE_PRECISION;
    
    // Baseline delta
    let delta_baseline = get_delta_a(&e, reserve_a, reserve_b, base_oracle_price, quote_oracle_price);
    
    // Tiny price change that should be below epsilon threshold
    let micro_change = 1; // Smallest possible change
    let delta_micro = get_delta_a(&e, reserve_a, reserve_b, base_oracle_price + micro_change, quote_oracle_price);
    
    // Due to epsilon smoothing, micro changes should not affect delta_a
    assert_eq!(delta_baseline, delta_micro,
        "Epsilon smoothing failed: micro price changes should not affect delta_a");
}

#[test]
fn test_delta_a_bounds_checking() {
    let e = Env::default();
    
    // Test extreme values that could cause overflow in calculations
    let test_cases = [
        (u128::MAX / 2, u128::MAX / 2, PRICE_PRECISION, PRICE_PRECISION),
        (1, u128::MAX / 2, PRICE_PRECISION, PRICE_PRECISION * 2),
        (u128::MAX / 2, 1, PRICE_PRECISION * 2, PRICE_PRECISION),
    ];
    
    for (reserve_a, reserve_b, base_price, quote_price) in test_cases {
        // Should not panic on extreme but valid inputs
        let _delta = get_delta_a(&e, reserve_a, reserve_b, base_price, quote_price);
        // If we reach here, bounds checking worked
    }
}

#[test]
fn test_conversion_safety_in_delta_a() {
    let e = Env::default();
    
    // Test that all conversions in get_delta_a are safe
    let large_reserve = u64::MAX as u128;
    let normal_price = PRICE_PRECISION;
    
    // This should not panic due to safe conversions
    let _delta = get_delta_a(&e, large_reserve, large_reserve, normal_price, normal_price);
    
    // Test with maximum safe values
    let max_safe_reserve = i128::MAX as u128;
    let _delta_max = get_delta_a(&e, max_safe_reserve / 2, max_safe_reserve / 2, normal_price, normal_price);
}

#[test]
fn test_precision_consistency_across_operations() {
    let e = Env::default();
    
    // Test that precision is maintained across multiple operations
    let base_reserve = 1_000_000 * PRICE_PRECISION;
    let quote_reserve = 1_000_000 * PRICE_PRECISION;
    
    // Series of price movements
    let prices = [
        (PRICE_PRECISION, PRICE_PRECISION),
        (PRICE_PRECISION * 105 / 100, PRICE_PRECISION), // 5% increase
        (PRICE_PRECISION * 95 / 100, PRICE_PRECISION),  // 5% decrease  
        (PRICE_PRECISION, PRICE_PRECISION),             // Back to baseline
    ];
    
    let baseline_delta = get_delta_a(&e, base_reserve, quote_reserve, prices[0].0, prices[0].1);
    
    for (base_price, quote_price) in prices.iter().skip(1) {
        let delta = get_delta_a(&e, base_reserve, quote_reserve, *base_price, *quote_price);
        
        // Verify delta is within reasonable bounds
        let max_reasonable_delta = (base_reserve as i128) / 2; // 50% of reserve
        assert!(delta.abs() <= max_reasonable_delta,
            "Delta_A outside reasonable bounds: {} > {}", delta.abs(), max_reasonable_delta);
    }
    
    // Final delta should be back to baseline (accounting for epsilon smoothing)
    let final_delta = get_delta_a(&e, base_reserve, quote_reserve, prices[3].0, prices[3].1);
    let epsilon = (base_reserve as i128) / 10000; // 0.01% tolerance
    
    assert!((final_delta - baseline_delta).abs() <= epsilon,
        "Round-trip precision loss too large: {} vs {} (diff: {})",
        final_delta, baseline_delta, (final_delta - baseline_delta).abs());
}

#[test]
fn test_bounded_drift_regression() {
    let e = Env::default();
    
    // Test for bounded drift over multiple sequential operations
    let mut reserve_a = 1_000_000 * PRICE_PRECISION;
    let reserve_b = 1_000_000 * PRICE_PRECISION;
    let base_price_start = PRICE_PRECISION;
    let quote_price = PRICE_PRECISION;
    
    let mut cumulative_drift = 0i128;
    let price_changes = [1, -1, 2, -2, 1, -1]; // Small oscillations
    
    for &price_change in &price_changes {
        let current_base_price = (base_price_start as i128 + price_change) as u128;
        let delta = get_delta_a(&e, reserve_a, reserve_b, current_base_price, quote_price);
        
        // Track cumulative drift
        cumulative_drift += delta;
        
        // Apply the change (simulate rebalancing)
        if delta > 0 {
            reserve_a += delta as u128;
        } else {
            reserve_a = reserve_a.saturating_sub((-delta) as u128);
        }
        
        // Verify delta is bounded per the cap we implemented
        let max_expected_delta = (reserve_a as i128) / 20; // 5% cap
        assert!(delta.abs() <= max_expected_delta,
            "Delta exceeds per-ledger cap: {} > {}", delta.abs(), max_expected_delta);
    }
    
    // Total drift should be bounded over the entire sequence
    let max_total_drift = (reserve_a as i128) / 100; // 1% max total drift
    assert!(cumulative_drift.abs() <= max_total_drift,
        "Cumulative drift too large: {} > {}", cumulative_drift.abs(), max_total_drift);
}

#[test]
fn test_rounding_mode_effectiveness() {
    let e = Env::default();
    
    // Test that round-to-nearest reduces bias compared to floor division
    let reserve_a = 1_000_000 * PRICE_PRECISION;
    let reserve_b = 1_000_000 * PRICE_PRECISION;
    
    let mut floor_bias = 0i128;
    let mut round_bias = 0i128;
    
    // Test with many small price variations
    for price_offset in -100..=100 {
        let base_price = PRICE_PRECISION + (price_offset as u128);
        let quote_price = PRICE_PRECISION;
        
        // Our implementation uses round-to-nearest
        let delta_round = get_delta_a(&e, reserve_a, reserve_b, base_price, quote_price);
        round_bias += delta_round;
        
        // Simulate what floor-only would have produced (for comparison)
        let peg_floor = quote_price * PRICE_PRECISION / base_price; // floor division
        let target_floor = reserve_b * PRICE_PRECISION / peg_floor; // floor division
        let delta_floor = (target_floor as i128) - (reserve_a as i128);
        floor_bias += delta_floor;
    }
    
    // Round-to-nearest should have significantly less bias than floor-only
    assert!(round_bias.abs() < floor_bias.abs() / 2,
        "Round-to-nearest bias {} should be less than half of floor bias {}",
        round_bias.abs(), floor_bias.abs());
}