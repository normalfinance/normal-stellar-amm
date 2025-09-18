// Comprehensive tests for Oracle Registry TWAP calculations
#![cfg(test)]

use soroban_sdk::{Env, Symbol, testutils::Address as _, Address};
use crate::oracle::{calculate_oracle_twap_price_spread_pct, oracle_validity, is_oracle_valid_for_action};
use crate::storage_types::{HistoricalOracleData, OracleValidity};
use utils::state::oracle_registry::{OraclePriceData, NormalAction};
use utils::constant::{PRICE_PRECISION, PRICE_PRECISION_U64, FIVE_MINUTE};
use utils::math::stats::{calculate_new_twap, calculate_weighted_average};

mod twap_calculation_tests {
    use super::*;

    #[test]
    fn test_twap_initial_value() {
        let e = Env::default();
        
        // First price observation
        let current_price = 1000_0000000u128;
        let current_ts = 1000u64;
        let last_twap = 0u128; // No previous TWAP
        let last_ts = 0u64;
        let period = FIVE_MINUTE as u64;
        
        let twap = calculate_new_twap(&e, current_price, current_ts, last_twap, last_ts, period);
        
        // First TWAP should equal current price
        assert_eq!(twap, current_price);
    }

    #[test]
    fn test_twap_no_time_change() {
        let e = Env::default();
        
        let current_price = 1100_0000000u128;
        let last_twap = 1000_0000000u128;
        let current_ts = 1000u64;
        let last_ts = 1000u64; // Same timestamp
        let period = FIVE_MINUTE as u64;
        
        let twap = calculate_new_twap(&e, current_price, current_ts, last_twap, last_ts, period);
        
        // No time passed, TWAP unchanged
        assert_eq!(twap, last_twap);
    }

    #[test]
    fn test_twap_partial_period() {
        let e = Env::default();
        
        let current_price = 1100_0000000u128;
        let last_twap = 1000_0000000u128;
        let current_ts = 1150u64; // 150 seconds later
        let last_ts = 1000u64;
        let period = 300u64; // 5 minutes
        
        let twap = calculate_new_twap(&e, current_price, current_ts, last_twap, last_ts, period);
        
        // Weight: 150s for new price, 150s for old TWAP
        // TWAP = (1100 * 150 + 1000 * 150) / 300 = 1050
        let expected = calculate_weighted_average(&e, current_price, last_twap, 150, 150);
        assert_eq!(twap, expected);
        assert_eq!(twap, 1050_0000000);
    }

    #[test]
    fn test_twap_full_period() {
        let e = Env::default();
        
        let current_price = 1100_0000000u128;
        let last_twap = 1000_0000000u128;
        let current_ts = 1300u64; // 300 seconds later (full period)
        let last_ts = 1000u64;
        let period = 300u64;
        
        let twap = calculate_new_twap(&e, current_price, current_ts, last_twap, last_ts, period);
        
        // Full period passed, TWAP = current price
        assert_eq!(twap, current_price);
    }

    #[test]
    fn test_twap_beyond_period() {
        let e = Env::default();
        
        let current_price = 1100_0000000u128;
        let last_twap = 1000_0000000u128;
        let current_ts = 2000u64; // 1000 seconds later (way beyond period)
        let last_ts = 1000u64;
        let period = 300u64;
        
        let twap = calculate_new_twap(&e, current_price, current_ts, last_twap, last_ts, period);
        
        // Beyond period, TWAP = current price
        assert_eq!(twap, current_price);
    }

    #[test]
    fn test_twap_price_smoothing() {
        let e = Env::default();
        
        // Simulate rapid price changes
        let mut twap = 1000_0000000u128;
        let period = 300u64;
        let mut ts = 1000u64;
        
        // Price spike to 2x
        let spike_price = 2000_0000000u128;
        ts += 30; // 30 seconds later
        twap = calculate_new_twap(&e, spike_price, ts, twap, ts - 30, period);
        
        // TWAP should only partially reflect spike
        assert!(twap < spike_price);
        assert!(twap > 1000_0000000);
        
        // Price crash to 0.5x
        let crash_price = 500_0000000u128;
        ts += 30;
        twap = calculate_new_twap(&e, crash_price, ts, twap, ts - 30, period);
        
        // TWAP should smooth out volatility
        assert!(twap > crash_price);
        assert!(twap < spike_price);
    }

    #[test]
    fn test_twap_convergence() {
        let e = Env::default();
        
        // Test that TWAP converges to stable price
        let stable_price = 1500_0000000u128;
        let mut twap = 1000_0000000u128;
        let period = 300u64;
        let mut ts = 1000u64;
        
        // Simulate multiple updates at stable price
        for _ in 0..10 {
            ts += 60; // 1 minute intervals
            twap = calculate_new_twap(&e, stable_price, ts, twap, ts - 60, period);
        }
        
        // After sufficient time, TWAP should converge to stable price
        let tolerance = stable_price / 100; // 1% tolerance
        assert!((twap as i128 - stable_price as i128).abs() < tolerance as i128);
    }
}

mod price_spread_tests {
    use super::*;

    #[test]
    fn test_spread_calculation_equal_prices() {
        let e = Env::default();
        
        let reserve_price = 1000_0000000u128;
        let twap = 1000_0000000u128;
        
        let spread = calculate_oracle_twap_price_spread_pct(&e, reserve_price, twap);
        
        assert_eq!(spread, 0);
    }

    #[test]
    fn test_spread_calculation_reserve_higher() {
        let e = Env::default();
        
        let reserve_price = 1100_0000000u128;
        let twap = 1000_0000000u128;
        
        let spread = calculate_oracle_twap_price_spread_pct(&e, reserve_price, twap);
        
        // (1100 - 1000) / 1100 = 0.0909 = 9.09%
        let expected = (100_0000000i64 * PRICE_PRECISION_U64 as i64) / 1100_0000000i64;
        assert_eq!(spread, expected);
        assert!(spread > 0); // Positive spread
    }

    #[test]
    fn test_spread_calculation_reserve_lower() {
        let e = Env::default();
        
        let reserve_price = 900_0000000u128;
        let twap = 1000_0000000u128;
        
        let spread = calculate_oracle_twap_price_spread_pct(&e, reserve_price, twap);
        
        // (900 - 1000) / 900 = -0.111 = -11.11%
        let expected = (-100_0000000i64 * PRICE_PRECISION_U64 as i64) / 900_0000000i64;
        assert_eq!(spread, expected);
        assert!(spread < 0); // Negative spread
    }

    #[test]
    fn test_spread_calculation_large_divergence() {
        let e = Env::default();
        
        let reserve_price = 2000_0000000u128;
        let twap = 1000_0000000u128;
        
        let spread = calculate_oracle_twap_price_spread_pct(&e, reserve_price, twap);
        
        // (2000 - 1000) / 2000 = 0.5 = 50%
        let expected = (1000_0000000i64 * PRICE_PRECISION_U64 as i64) / 2000_0000000i64;
        assert_eq!(spread, expected);
        assert_eq!(spread, PRICE_PRECISION_U64 as i64 / 2); // 50%
    }

    #[test]
    fn test_spread_precision() {
        let e = Env::default();
        
        // Test small percentage differences
        let reserve_price = 1001_0000000u128;
        let twap = 1000_0000000u128;
        
        let spread = calculate_oracle_twap_price_spread_pct(&e, reserve_price, twap);
        
        // 0.1% difference
        assert!(spread > 0);
        assert!(spread < PRICE_PRECISION_U64 as i64 / 100); // Less than 1%
    }
}

mod oracle_validity_tests {
    use super::*;

    #[test]
    fn test_validity_for_swap() {
        // Swap requires strictly valid oracle
        assert!(is_oracle_valid_for_action(
            OracleValidity::Valid,
            Some(NormalAction::Swap)
        ));
        
        assert!(!is_oracle_valid_for_action(
            OracleValidity::StaleForPool,
            Some(NormalAction::Swap)
        ));
        
        assert!(!is_oracle_valid_for_action(
            OracleValidity::StaleForSwap,
            Some(NormalAction::Swap)
        ));
    }

    #[test]
    fn test_validity_for_liquidity() {
        // Add/Remove liquidity allows mildly stale data
        assert!(is_oracle_valid_for_action(
            OracleValidity::Valid,
            Some(NormalAction::AddLiquidity)
        ));
        
        assert!(is_oracle_valid_for_action(
            OracleValidity::StaleForPool,
            Some(NormalAction::AddLiquidity)
        ));
        
        assert!(!is_oracle_valid_for_action(
            OracleValidity::StaleForSwap,
            Some(NormalAction::AddLiquidity)
        ));
    }

    #[test]
    fn test_validity_for_rebalance() {
        // Rebalance requires strictly valid oracle
        assert!(is_oracle_valid_for_action(
            OracleValidity::Valid,
            Some(NormalAction::Rebalance)
        ));
        
        assert!(!is_oracle_valid_for_action(
            OracleValidity::StaleForPool,
            Some(NormalAction::Rebalance)
        ));
    }

    #[test]
    fn test_validity_for_update_twap() {
        // UpdateTwap allows anything except non-positive
        assert!(is_oracle_valid_for_action(
            OracleValidity::Valid,
            Some(NormalAction::UpdateTwap)
        ));
        
        assert!(is_oracle_valid_for_action(
            OracleValidity::StaleForSwap,
            Some(NormalAction::UpdateTwap)
        ));
        
        assert!(is_oracle_valid_for_action(
            OracleValidity::TooVolatile,
            Some(NormalAction::UpdateTwap)
        ));
        
        assert!(!is_oracle_valid_for_action(
            OracleValidity::NonPositive,
            Some(NormalAction::UpdateTwap)
        ));
    }

    #[test]
    fn test_validity_for_claim_insurance() {
        // ClaimInsurance blocked by non-positive or volatile
        assert!(is_oracle_valid_for_action(
            OracleValidity::Valid,
            Some(NormalAction::ClaimInsurance)
        ));
        
        assert!(is_oracle_valid_for_action(
            OracleValidity::StaleForSwap,
            Some(NormalAction::ClaimInsurance)
        ));
        
        assert!(!is_oracle_valid_for_action(
            OracleValidity::NonPositive,
            Some(NormalAction::ClaimInsurance)
        ));
        
        assert!(!is_oracle_valid_for_action(
            OracleValidity::TooVolatile,
            Some(NormalAction::ClaimInsurance)
        ));
    }

    #[test]
    fn test_validity_default_action() {
        // No action specified defaults to requiring valid
        assert!(is_oracle_valid_for_action(OracleValidity::Valid, None));
        
        assert!(!is_oracle_valid_for_action(OracleValidity::StaleForPool, None));
        assert!(!is_oracle_valid_for_action(OracleValidity::StaleForSwap, None));
        assert!(!is_oracle_valid_for_action(OracleValidity::TooVolatile, None));
    }
}

mod price_sanitization_tests {
    use super::*;

    fn sanitize_new_price(
        e: &Env,
        new_price: u128,
        last_twap: u128,
        clamp_denominator: i64,
    ) -> u128 {
        // Simplified sanitization logic
        if last_twap == 0 || clamp_denominator == 0 {
            return new_price;
        }
        
        let max_change = last_twap / (clamp_denominator.abs() as u128);
        let upper_bound = last_twap + max_change;
        let lower_bound = last_twap.saturating_sub(max_change);
        
        new_price.min(upper_bound).max(lower_bound)
    }

    #[test]
    fn test_price_sanitization_within_bounds() {
        let e = Env::default();
        
        let last_twap = 1000_0000000u128;
        let new_price = 1050_0000000u128; // 5% increase
        let clamp_denominator = 10i64; // 10% max change
        
        let sanitized = sanitize_new_price(&e, new_price, last_twap, clamp_denominator);
        
        // Within bounds, should not change
        assert_eq!(sanitized, new_price);
    }

    #[test]
    fn test_price_sanitization_above_upper_bound() {
        let e = Env::default();
        
        let last_twap = 1000_0000000u128;
        let new_price = 1500_0000000u128; // 50% increase
        let clamp_denominator = 10i64; // 10% max change
        
        let sanitized = sanitize_new_price(&e, new_price, last_twap, clamp_denominator);
        
        // Should be clamped to 1100 (10% increase)
        assert_eq!(sanitized, 1100_0000000);
    }

    #[test]
    fn test_price_sanitization_below_lower_bound() {
        let e = Env::default();
        
        let last_twap = 1000_0000000u128;
        let new_price = 500_0000000u128; // 50% decrease
        let clamp_denominator = 10i64; // 10% max change
        
        let sanitized = sanitize_new_price(&e, new_price, last_twap, clamp_denominator);
        
        // Should be clamped to 900 (10% decrease)
        assert_eq!(sanitized, 900_0000000);
    }

    #[test]
    fn test_price_sanitization_zero_twap() {
        let e = Env::default();
        
        let last_twap = 0u128;
        let new_price = 1000_0000000u128;
        let clamp_denominator = 10i64;
        
        let sanitized = sanitize_new_price(&e, new_price, last_twap, clamp_denominator);
        
        // No previous TWAP, accept new price
        assert_eq!(sanitized, new_price);
    }

    #[test]
    fn test_price_sanitization_zero_clamp() {
        let e = Env::default();
        
        let last_twap = 1000_0000000u128;
        let new_price = 2000_0000000u128;
        let clamp_denominator = 0i64;
        
        let sanitized = sanitize_new_price(&e, new_price, last_twap, clamp_denominator);
        
        // No clamping, accept new price
        assert_eq!(sanitized, new_price);
    }
}