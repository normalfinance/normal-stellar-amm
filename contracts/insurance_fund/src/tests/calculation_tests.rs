// Comprehensive tests for Insurance Fund calculations
#![cfg(test)]

use soroban_sdk::Env;
use crate::interest::{calculate_utilization, calculate_rate};
use utils::constant::{PERCENTAGE_PRECISION_U32, PRICE_PRECISION};

mod utilization_tests {
    use super::*;

    #[test]
    fn test_utilization_zero_vault() {
        // 0 insurance vault = 0% utilization
        let util = calculate_utilization(0, 1000_0000000);
        assert_eq!(util, 0);
    }

    #[test]
    fn test_utilization_zero_optimal() {
        // 0 optimal insurance = 0% (edge case protection)
        let util = calculate_utilization(1000_0000000, 0);
        assert_eq!(util, 0);
    }

    #[test]
    fn test_utilization_50_percent() {
        // 500/1000 = 50%
        let vault = 500_0000000 * PRICE_PRECISION;
        let optimal = 1000_0000000 * PRICE_PRECISION;
        let util = calculate_utilization(vault, optimal);
        assert_eq!(util, PERCENTAGE_PRECISION_U32 / 2);
    }

    #[test]
    fn test_utilization_100_percent() {
        // 1000/1000 = 100%
        let vault = 1000_0000000 * PRICE_PRECISION;
        let optimal = 1000_0000000 * PRICE_PRECISION;
        let util = calculate_utilization(vault, optimal);
        assert_eq!(util, PERCENTAGE_PRECISION_U32);
    }

    #[test]
    fn test_utilization_over_100_percent() {
        // 1500/1000 = 150% (over-insured)
        let vault = 1500_0000000 * PRICE_PRECISION;
        let optimal = 1000_0000000 * PRICE_PRECISION;
        let util = calculate_utilization(vault, optimal);
        assert_eq!(util, PERCENTAGE_PRECISION_U32 * 3 / 2);
    }

    #[test]
    fn test_utilization_precision() {
        // Test small fractions
        let vault = 1 * PRICE_PRECISION;
        let optimal = 1000 * PRICE_PRECISION;
        let util = calculate_utilization(vault, optimal);
        // 0.1% = 10000 (with PERCENTAGE_PRECISION = 10_000_000)
        assert_eq!(util, 10000);
    }
}

mod interest_rate_tests {
    use super::*;

    #[test]
    fn test_rate_zero_utilization() {
        let e = Env::default();
        let rate = calculate_rate(&e, 0, 8000, 100, 400, 1500);
        assert_eq!(rate, 100); // Base rate only
    }

    #[test]
    fn test_rate_at_optimal() {
        let e = Env::default();
        // At 80% utilization (optimal)
        let rate = calculate_rate(&e, 8000, 8000, 100, 400, 1500);
        // rate = base + slope1 = 100 + 400 = 500
        assert_eq!(rate, 500);
    }

    #[test]
    fn test_rate_below_optimal() {
        let e = Env::default();
        // At 40% utilization (half of optimal 80%)
        let rate = calculate_rate(&e, 4000, 8000, 100, 400, 1500);
        // rate = base + (4000/8000) * 400 = 100 + 200 = 300
        assert_eq!(rate, 300);
    }

    #[test]
    fn test_rate_above_optimal() {
        let e = Env::default();
        // At 90% utilization (above optimal 80%)
        let rate = calculate_rate(&e, 9000, 8000, 100, 400, 1500);
        // excess = 1000, remaining = 2000
        // rate = 100 + 400 + (1000/2000) * 1500 = 100 + 400 + 750 = 1250
        assert_eq!(rate, 1250);
    }

    #[test]
    fn test_rate_at_100_percent() {
        let e = Env::default();
        // At 100% utilization
        let rate = calculate_rate(&e, 10000, 8000, 100, 400, 1500);
        // excess = 2000, remaining = 2000
        // rate = 100 + 400 + (2000/2000) * 1500 = 100 + 400 + 1500 = 2000
        assert_eq!(rate, 2000);
    }

    #[test]
    fn test_rate_negative_base() {
        let e = Env::default();
        // Negative base rate (penalty for over-insurance)
        let rate = calculate_rate(&e, 0, 8000, -200, 400, 1500);
        assert_eq!(rate, -200);
        
        // At optimal with negative base
        let rate = calculate_rate(&e, 8000, 8000, -200, 400, 1500);
        assert_eq!(rate, 200); // -200 + 400 = 200
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #21)")]
    fn test_rate_invalid_optimal_zero() {
        let e = Env::default();
        calculate_rate(&e, 5000, 0, 100, 400, 1500);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #21)")]
    fn test_rate_invalid_optimal_over_100() {
        let e = Env::default();
        calculate_rate(&e, 5000, 10001, 100, 400, 1500);
    }

    #[test]
    fn test_rate_curve_continuity() {
        let e = Env::default();
        
        // Test continuity at optimal point
        let rate_just_below = calculate_rate(&e, 7999, 8000, 100, 400, 1500);
        let rate_at_optimal = calculate_rate(&e, 8000, 8000, 100, 400, 1500);
        let rate_just_above = calculate_rate(&e, 8001, 8000, 100, 400, 1500);
        
        // Should be continuous (small jump is acceptable due to integer math)
        assert!(rate_at_optimal >= rate_just_below);
        assert!(rate_just_above >= rate_at_optimal);
        assert!((rate_just_above - rate_just_below) < 10); // Small difference
    }
}

mod aave_v3_comparison_tests {
    use super::*;

    #[test]
    fn test_aave_v3_like_curve() {
        let e = Env::default();
        
        // Typical Aave v3 parameters (scaled to basis points)
        let optimal_util = 8000; // 80%
        let base_rate = 0;
        let slope1 = 400; // 4% at optimal
        let slope2 = 6000; // 60% slope after optimal
        
        // Test key points on the curve
        let rate_0 = calculate_rate(&e, 0, optimal_util, base_rate, slope1, slope2);
        let rate_50 = calculate_rate(&e, 5000, optimal_util, base_rate, slope1, slope2);
        let rate_80 = calculate_rate(&e, 8000, optimal_util, base_rate, slope1, slope2);
        let rate_90 = calculate_rate(&e, 9000, optimal_util, base_rate, slope1, slope2);
        let rate_100 = calculate_rate(&e, 10000, optimal_util, base_rate, slope1, slope2);
        
        assert_eq!(rate_0, 0);
        assert_eq!(rate_50, 250); // 2.5%
        assert_eq!(rate_80, 400); // 4%
        assert_eq!(rate_90, 3400); // 34%
        assert_eq!(rate_100, 6400); // 64%
        
        // Verify curve properties
        assert!(rate_50 < rate_80); // Increasing
        assert!(rate_80 < rate_90); // Increasing
        assert!((rate_90 - rate_80) > (rate_80 - rate_50)); // Steeper after optimal
    }
}