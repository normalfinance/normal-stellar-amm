#![cfg(test)]

use soroban_sdk::{Env, Address};
use utils::state::oracle_registry::{OracleGuardRails, OracleValidity, OraclePriceData, ValidityGuardRails, PriceDivergenceGuardRails};
use utils::temporal::Delay;
use crate::oracle::oracle_validity;
use crate::storage::set_oracle_guard_rails;

// Helper function to test staleness validation using the real oracle_validity function
fn test_staleness_validation(e: &Env, oracle_delay_seconds: u64, stale_threshold_seconds: u64) -> bool {
    use soroban_sdk::testutils::Address as _;
    
    let contract_address = Address::generate(e);
    
    e.as_contract(&contract_address, || {
        // Set up oracle guard rails with the test threshold
        let guard_rails = OracleGuardRails {
            validity: ValidityGuardRails {
                seconds_before_stale_for_pool: stale_threshold_seconds,
                too_volatile_ratio: 200_000, // 20% - not relevant for staleness test
            },
            price_divergence: PriceDivergenceGuardRails {
                oracle_twap_percent_divergence: 100_000, // 10% - not relevant for staleness test
            },
        };
        set_oracle_guard_rails(e, &guard_rails);
        
        // Create oracle price data with the specified delay
        let oracle_price_data = OraclePriceData {
            price: 1_000_000, // $1 - not relevant for staleness test
            delay: Delay::from_seconds(oracle_delay_seconds),
        };
        
        // Call the real oracle_validity function
        let validity = oracle_validity(e, 1_000_000, &oracle_price_data); // TWAP = $1
        
        // Return true if oracle is stale
        matches!(validity, OracleValidity::StaleForPool)
    })
}

#[test]
fn test_oracle_timestamp_validation_boundary() {
    // Test the specific fix for oracle timestamp validation (> to >=)
    
    let test_cases = [
        // (oracle_delay, threshold, should_be_stale)
        (59, 60, false),   // Below threshold - not stale
        (60, 60, true),    // Exactly at threshold - should be stale (this was the bug)
        (61, 60, true),    // Above threshold - stale
        (0, 60, false),    // Zero delay - not stale
        (120, 60, true),   // Well above threshold - stale
    ];
    
    let e = Env::default();
    for (delay, threshold, expected_stale) in test_cases {
        let is_stale = test_staleness_validation(&e, delay, threshold);
        assert_eq!(is_stale, expected_stale,
            "Staleness validation incorrect: delay={}, threshold={}, expected={}, got={}",
            delay, threshold, expected_stale, is_stale);
    }
}

#[test] 
fn test_oracle_boundary_precision() {
    // Test precise boundary conditions that could be exploited
    
    // Test the exact boundary case that was problematic
    let exact_threshold = 60;
    
    let e = Env::default();
    // Before fix: delay > threshold meant 60 > 60 = false (not stale)
    // After fix: delay >= threshold means 60 >= 60 = true (correctly stale)
    assert!(test_staleness_validation(&e, exact_threshold, exact_threshold),
        "Oracle should be considered stale exactly at threshold");
    
    // Test one second before and after
    assert!(!test_staleness_validation(&e, exact_threshold - 1, exact_threshold),
        "Oracle should not be stale one second before threshold");
    
    assert!(test_staleness_validation(&e, exact_threshold + 1, exact_threshold),
        "Oracle should be stale one second after threshold");
}

#[test]
fn test_clock_drift_tolerance() {
    // Test scenarios where small clock drifts could affect validation
    
    let base_threshold = 300; // 5 minutes
    
    // Test small clock drifts that should be handled properly
    let clock_drift_cases = [
        base_threshold - 1,  // Just under
        base_threshold,      // Exactly at
        base_threshold + 1,  // Just over
    ];
    
    let e = Env::default();
    for delay in clock_drift_cases {
        let is_stale = test_staleness_validation(&e, delay, base_threshold);
        
        if delay >= base_threshold {
            assert!(is_stale, "Should be stale when delay >= threshold: {}", delay);
        } else {
            assert!(!is_stale, "Should not be stale when delay < threshold: {}", delay);
        }
    }
}

#[test]
fn test_extreme_timestamp_values() {
    // Test edge cases with extreme timestamp values
    
    let test_cases = [
        (0, 0, true),           // Both zero - should be stale
        (u64::MAX, u64::MAX, true), // Both maximum - should be stale  
        (u64::MAX - 1, u64::MAX, false), // Just under max - not stale
        (1, 0, true),           // Any delay with zero threshold - stale
        (0, u64::MAX, false),   // Zero delay with large threshold - not stale
    ];
    
    let e = Env::default();
    for (delay, threshold, expected_stale) in test_cases {
        let is_stale = test_staleness_validation(&e, delay, threshold);
        assert_eq!(is_stale, expected_stale,
            "Extreme value test failed: delay={}, threshold={}, expected={}, got={}",
            delay, threshold, expected_stale, is_stale);
    }
}

#[test]
fn test_operational_scenarios() {
    // Test realistic operational scenarios
    
    struct TestCase {
        name: &'static str,
        delay_seconds: u64,
        threshold_seconds: u64, 
        expected_stale: bool,
    }
    
    let scenarios = [
        TestCase {
            name: "Fresh oracle (30s delay, 60s threshold)",
            delay_seconds: 30,
            threshold_seconds: 60,
            expected_stale: false,
        },
        TestCase {
            name: "Exactly stale (60s delay, 60s threshold)", 
            delay_seconds: 60,
            threshold_seconds: 60,
            expected_stale: true,
        },
        TestCase {
            name: "Clearly stale (120s delay, 60s threshold)",
            delay_seconds: 120, 
            threshold_seconds: 60,
            expected_stale: true,
        },
        TestCase {
            name: "Long threshold, fresh oracle (30s delay, 300s threshold)",
            delay_seconds: 30,
            threshold_seconds: 300,
            expected_stale: false,
        },
        TestCase {
            name: "Long threshold, stale oracle (300s delay, 300s threshold)",
            delay_seconds: 300,
            threshold_seconds: 300, 
            expected_stale: true,
        },
    ];
    
    let e = Env::default();
    for scenario in scenarios {
        let is_stale = test_staleness_validation(&e, scenario.delay_seconds, scenario.threshold_seconds);
        assert_eq!(is_stale, scenario.expected_stale, 
            "Scenario '{}' failed: expected={}, got={}", 
            scenario.name, scenario.expected_stale, is_stale);
    }
}

#[test]
fn test_timestamp_arithmetic_safety() {
    // Test that timestamp arithmetic doesn't overflow or underflow
    
    // These are conceptual tests since we can't test the actual oracle contract
    // but we can test the logic patterns
    
    let current_time = 1000000_u64;
    let oracle_time = 999940_u64; // 60 seconds ago
    let delay = current_time - oracle_time; // Should be 60
    
    assert_eq!(delay, 60);
    
    // Test potential underflow case (oracle time in future)
    let future_oracle_time = current_time + 60;
    // In real implementation, this should be handled gracefully
    // Here we just verify the math
    assert!(future_oracle_time > current_time);
}

#[test]
fn test_validation_consistency() {
    // Test that validation is consistent across multiple calls
    
    let delay = 60;
    let threshold = 60;
    
    let e = Env::default();
    // Call multiple times to ensure consistency
    for _ in 0..100 {
        let is_stale = test_staleness_validation(&e, delay, threshold);
        assert!(is_stale, "Validation should be consistent across calls");
    }
}

#[test]
fn test_threshold_edge_cases() {
    // Test various threshold values for edge case behavior
    
    let thresholds = [0, 1, 59, 60, 61, 3600, u64::MAX];
    let test_delay = 60;
    
    let e = Env::default();
    for threshold in thresholds {
        let is_stale = test_staleness_validation(&e, test_delay, threshold);
        
        if test_delay >= threshold {
            assert!(is_stale, "Should be stale when delay {} >= threshold {}", test_delay, threshold);
        } else {
            assert!(!is_stale, "Should not be stale when delay {} < threshold {}", test_delay, threshold);
        }
    }
}