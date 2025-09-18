// Deep security and edge case tests for Oracle Registry TWAP calculations
#![cfg(test)]

use soroban_sdk::{Env, Symbol, testutils::Address as _, Address};
use crate::oracle::{calculate_oracle_twap_price_spread_pct, oracle_validity, is_oracle_valid_for_action};
use crate::storage_types::{HistoricalOracleData, OracleValidity};
use utils::state::oracle_registry::{OraclePriceData, NormalAction};
use utils::constant::{PRICE_PRECISION, PRICE_PRECISION_U64, FIVE_MINUTE};
use utils::math::stats::{calculate_new_twap, calculate_weighted_average};

mod twap_security_tests {
    use super::*;

    #[test]
    fn test_twap_price_manipulation_attack() {
        let e = Env::default();
        
        // Simulate price manipulation attack on TWAP
        let period = 300u64; // 5 minutes
        let mut twap = 1000_0000000u128; // Starting TWAP
        let mut timestamp = 1000u64;
        
        // Normal price updates
        for i in 1..=10 {
            timestamp += 30; // 30 seconds each
            let normal_price = 1000_0000000 + (i * 1_0000000); // Gradual increase
            twap = calculate_new_twap(&e, normal_price, timestamp, twap, timestamp - 30, period);
        }
        let normal_final_twap = twap;
        
        // Reset for attack scenario
        twap = 1000_0000000u128;
        timestamp = 1000u64;
        
        // Attack: Massive price spike for short duration
        timestamp += 30;
        let attack_price = 10000_0000000u128; // 10x price spike
        twap = calculate_new_twap(&e, attack_price, timestamp, twap, timestamp - 30, period);
        
        // Continue with normal prices
        for i in 2..=10 {
            timestamp += 30;
            let normal_price = 1000_0000000 + (i * 1_0000000);
            twap = calculate_new_twap(&e, normal_price, timestamp, twap, timestamp - 30, period);
        }
        let attack_final_twap = twap;
        
        // Attack should have limited impact due to time weighting
        let manipulation_impact = attack_final_twap as f64 / normal_final_twap as f64;
        assert!(manipulation_impact < 2.0); // Less than 2x impact
        assert!(manipulation_impact > 0.5); // But still some impact
    }

    #[test]
    fn test_twap_flash_loan_resistance() {
        let e = Env::default();
        
        let period = 300u64;
        let base_twap = 1000_0000000u128;
        let timestamp = 1000u64;
        
        // Flash loan attack: extreme price for 1 second
        let flash_price = 100_0000000u128; // 90% price drop
        let flash_duration = 1u64; // 1 second
        
        let post_flash_twap = calculate_new_twap(
            &e, 
            flash_price, 
            timestamp + flash_duration, 
            base_twap, 
            timestamp, 
            period
        );
        
        // Impact should be minimal due to short duration
        let impact = (base_twap as f64 - post_flash_twap as f64) / base_twap as f64;
        assert!(impact < 0.01); // Less than 1% impact for 1-second flash loan
        
        // Test with longer flash loan
        let longer_flash_duration = 60u64; // 1 minute
        let longer_flash_twap = calculate_new_twap(
            &e,
            flash_price,
            timestamp + longer_flash_duration,
            base_twap,
            timestamp,
            period
        );
        
        let longer_impact = (base_twap as f64 - longer_flash_twap as f64) / base_twap as f64;
        assert!(longer_impact > impact); // Longer duration = more impact
        assert!(longer_impact < 0.2); // But still limited to 20%
    }

    #[test]
    fn test_twap_timestamp_manipulation() {
        let e = Env::default();
        
        let period = 300u64;
        let base_twap = 1000_0000000u128;
        let base_timestamp = 1000u64;
        
        // Normal update
        let normal_price = 1100_0000000u128;
        let normal_timestamp = base_timestamp + 60; // 1 minute later
        let normal_twap = calculate_new_twap(&e, normal_price, normal_timestamp, base_twap, base_timestamp, period);
        
        // Attack: Manipulate timestamp to appear much later
        let manipulated_timestamp = base_timestamp + period; // Full period later
        let manipulated_twap = calculate_new_twap(&e, normal_price, manipulated_timestamp, base_twap, base_timestamp, period);
        
        // Manipulated timestamp should make new price have full weight
        assert_eq!(manipulated_twap, normal_price); // Should equal new price exactly
        assert!(manipulated_twap != normal_twap); // Different from normal update
        
        // Test backwards timestamp (should be handled gracefully)
        let backwards_timestamp = base_timestamp - 60; // 1 minute before
        let backwards_twap = calculate_new_twap(&e, normal_price, backwards_timestamp, base_twap, base_timestamp, period);
        
        // Should handle backwards time gracefully (likely no change or minimal change)
        assert!(backwards_twap <= base_twap + (normal_price - base_twap) / 10); // Limited change
    }

    #[test]
    fn test_twap_precision_attacks() {
        let e = Env::default();
        
        // Attack with very small price differences to exploit precision
        let period = 300u64;
        let base_twap = 1000_0000000u128;
        let timestamp = 1000u64;
        
        // Tiny price changes
        for i in 1..=100 {
            let tiny_change = i; // Just 1-100 units difference
            let new_price = base_twap + tiny_change;
            let new_timestamp = timestamp + (i as u64);
            
            let new_twap = calculate_new_twap(&e, new_price, new_timestamp, base_twap, timestamp, period);
            
            // Changes should be proportional and stable
            assert!(new_twap >= base_twap); // Should increase
            assert!(new_twap <= base_twap + tiny_change); // But not by more than the change
        }
        
        // Test with maximum precision values
        let max_price = u128::MAX / 2; // Avoid overflow
        let max_twap = calculate_new_twap(&e, max_price, timestamp + 1, base_twap, timestamp, period);
        
        // Should handle max values without overflow
        assert!(max_twap > base_twap);
        assert!(max_twap < u128::MAX);
    }

    #[test]
    fn test_twap_period_boundary_attacks() {
        let e = Env::default();
        
        let period = 300u64; // 5 minutes
        let base_twap = 1000_0000000u128;
        let base_timestamp = 1000u64;
        
        // Attack: Update exactly at period boundary
        let boundary_price = 2000_0000000u128; // 2x price
        let boundary_timestamp = base_timestamp + period; // Exactly one period
        
        let boundary_twap = calculate_new_twap(&e, boundary_price, boundary_timestamp, base_twap, base_timestamp, period);
        
        // Should give full weight to new price
        assert_eq!(boundary_twap, boundary_price);
        
        // Attack: Update just before period boundary
        let just_before_timestamp = base_timestamp + period - 1; // 1 second before
        let just_before_twap = calculate_new_twap(&e, boundary_price, just_before_timestamp, base_twap, base_timestamp, period);
        
        // Should give almost full weight to new price
        let expected_weight_old = 1u128;
        let expected_weight_new = period as u128 - 1;
        let expected_twap = calculate_weighted_average(&e, boundary_price, base_twap, expected_weight_new, expected_weight_old);
        
        assert_eq!(just_before_twap, expected_twap);
        
        // Attack: Update just after period boundary
        let just_after_timestamp = base_timestamp + period + 1;
        let just_after_twap = calculate_new_twap(&e, boundary_price, just_after_timestamp, base_twap, base_timestamp, period);
        
        // Should also give full weight to new price
        assert_eq!(just_after_twap, boundary_price);
    }

    #[test]
    fn test_twap_weighted_average_manipulation() {
        let e = Env::default();
        
        // Test manipulation of weighted average calculation
        let current_price = 2000_0000000u128;
        let last_twap = 1000_0000000u128;
        
        // Normal weighting
        let normal_since_last = 100u64;
        let normal_from_start = 200u64;
        let normal_avg = calculate_weighted_average(&e, current_price, last_twap, normal_since_last, normal_from_start);
        
        // Expected: (2000 * 100 + 1000 * 200) / 300 = 400000 / 300 = 1333.33
        let expected = (current_price * normal_since_last as u128 + last_twap * normal_from_start as u128) / (normal_since_last + normal_from_start) as u128;
        assert_eq!(normal_avg, expected);
        
        // Attack: Manipulate weights to favor current price
        let attack_since_last = 1000u64; // High weight for current
        let attack_from_start = 1u64; // Low weight for historical
        let attack_avg = calculate_weighted_average(&e, current_price, last_twap, attack_since_last, attack_from_start);
        
        // Should heavily favor current price
        assert!(attack_avg > normal_avg);
        assert!(attack_avg > 1900_0000000); // Close to current price
        
        // Reverse attack: Favor historical price
        let reverse_since_last = 1u64;
        let reverse_from_start = 1000u64;
        let reverse_avg = calculate_weighted_average(&e, current_price, last_twap, reverse_since_last, reverse_from_start);
        
        // Should heavily favor historical price
        assert!(reverse_avg < normal_avg);
        assert!(reverse_avg < 1100_0000000); // Close to historical price
    }

    #[test]
    fn test_twap_convergence_attacks() {
        let e = Env::default();
        
        // Test if attacker can prevent TWAP convergence
        let period = 300u64;
        let target_price = 1500_0000000u128;
        let mut twap = 1000_0000000u128;
        let mut timestamp = 1000u64;
        
        // Honest scenario: Consistent price updates
        for _ in 0..20 {
            timestamp += 30; // 30 seconds each
            twap = calculate_new_twap(&e, target_price, timestamp, twap, timestamp - 30, period);
        }
        let honest_convergence = twap;
        
        // Reset for attack
        twap = 1000_0000000u128;
        timestamp = 1000u64;
        
        // Attack: Alternate between high and low prices
        for i in 0..20 {
            timestamp += 30;
            let attack_price = if i % 2 == 0 {
                target_price + 500_0000000 // High
            } else {
                target_price - 500_0000000 // Low
            };
            twap = calculate_new_twap(&e, attack_price, timestamp, twap, timestamp - 30, period);
        }
        let attack_convergence = twap;
        
        // Attack should still converge towards average (target_price)
        let honest_distance = (honest_convergence as i128 - target_price as i128).abs();
        let attack_distance = (attack_convergence as i128 - target_price as i128).abs();
        
        // Both should be reasonably close to target
        assert!(honest_distance < 100_0000000); // Within 10%
        assert!(attack_distance < 200_0000000); // Attack creates more noise but still converges
    }
}

mod price_spread_security_tests {
    use super::*;

    #[test]
    fn test_price_spread_manipulation_detection() {
        let e = Env::default();
        
        // Test detection of price manipulation through spread calculation
        let honest_reserve_price = 1000_0000000u128;
        let honest_twap = 1000_0000000u128;
        
        let honest_spread = calculate_oracle_twap_price_spread_pct(&e, honest_reserve_price, honest_twap);
        assert_eq!(honest_spread, 0); // No spread for honest prices
        
        // Manipulation: Reserve price manipulated up
        let manipulated_reserve = 1200_0000000u128; // 20% higher
        let manipulation_spread = calculate_oracle_twap_price_spread_pct(&e, manipulated_reserve, honest_twap);
        
        // Should detect 20% positive spread
        let expected_spread = (200_0000000i64 * PRICE_PRECISION_U64 as i64) / 1200_0000000i64;
        assert_eq!(manipulation_spread, expected_spread);
        assert!(manipulation_spread > 0); // Positive spread
        
        // Manipulation: Reserve price manipulated down
        let dumped_reserve = 800_0000000u128; // 20% lower
        let dump_spread = calculate_oracle_twap_price_spread_pct(&e, dumped_reserve, honest_twap);
        
        // Should detect negative spread
        assert!(dump_spread < 0); // Negative spread
        assert_eq!(dump_spread, -((200_0000000i64 * PRICE_PRECISION_U64 as i64) / 800_0000000i64));
    }

    #[test]
    fn test_price_spread_extreme_scenarios() {
        let e = Env::default();
        
        // Extreme scenario 1: Reserve price near zero
        let tiny_reserve = 1u128;
        let normal_twap = 1000_0000000u128;
        
        let extreme_negative_spread = calculate_oracle_twap_price_spread_pct(&e, tiny_reserve, normal_twap);
        
        // Should handle extreme negative spread
        assert!(extreme_negative_spread < -PRICE_PRECISION_U64 as i64); // More than -100%
        
        // Extreme scenario 2: TWAP near zero
        let normal_reserve = 1000_0000000u128;
        let tiny_twap = 1u128;
        
        let extreme_positive_spread = calculate_oracle_twap_price_spread_pct(&e, normal_reserve, tiny_twap);
        
        // Should handle extreme positive spread
        assert!(extreme_positive_spread > PRICE_PRECISION_U64 as i64 * 100); // More than 10000%
        
        // Extreme scenario 3: Both very large
        let huge_reserve = u128::MAX / 2;
        let huge_twap = u128::MAX / 3;
        
        let large_spread = calculate_oracle_twap_price_spread_pct(&e, huge_reserve, huge_twap);
        
        // Should handle large values without overflow
        assert!(large_spread > 0); // Reserve > TWAP
        assert!(large_spread < i64::MAX); // No overflow
    }

    #[test]
    fn test_price_spread_precision_attacks() {
        let e = Env::default();
        
        // Attack: Use tiny differences to exploit precision
        let base_price = 1000_0000000u128;
        
        for i in 1..=10 {
            let reserve_price = base_price + i; // Tiny increase
            let spread = calculate_oracle_twap_price_spread_pct(&e, reserve_price, base_price);
            
            // Should detect even tiny differences
            assert!(spread > 0);
            assert!(spread < 1000); // But should be proportionally tiny
        }
        
        // Test precision boundaries
        let precision_reserve = base_price + PRICE_PRECISION_U64 as u128; // +1 in fixed point
        let precision_spread = calculate_oracle_twap_price_spread_pct(&e, precision_reserve, base_price);
        
        // Should be exactly PRICE_PRECISION in spread
        let expected = (PRICE_PRECISION_U64 as i64 * PRICE_PRECISION_U64 as i64) / precision_reserve as i64;
        assert_eq!(precision_spread, expected);
    }

    #[test]
    fn test_price_spread_sandwich_attack_detection() {
        let e = Env::default();
        
        // Simulate sandwich attack detection
        let normal_twap = 1000_0000000u128; // Stable TWAP
        
        // Before sandwich: Normal reserve price
        let before_reserve = 1000_0000000u128;
        let before_spread = calculate_oracle_twap_price_spread_pct(&e, before_reserve, normal_twap);
        assert_eq!(before_spread, 0);
        
        // During sandwich: Manipulated reserve price
        let sandwich_reserve = 1100_0000000u128; // 10% pump
        let sandwich_spread = calculate_oracle_twap_price_spread_pct(&e, sandwich_reserve, normal_twap);
        
        // Should detect the manipulation
        assert!(sandwich_spread > 0);
        let spread_percent = sandwich_spread as f64 / PRICE_PRECISION_U64 as f64;
        assert!((spread_percent - 0.09).abs() < 0.01); // ~9% spread
        
        // After sandwich: Back to normal (if TWAP hasn't moved)
        let after_reserve = 1000_0000000u128;
        let after_spread = calculate_oracle_twap_price_spread_pct(&e, after_reserve, normal_twap);
        assert_eq!(after_spread, 0);
        
        // The spread calculation should make the sandwich attack detectable
    }
}

mod oracle_validity_security_tests {
    use super::*;

    #[test]
    fn test_oracle_validity_bypass_attempts() {
        // Test attempts to bypass oracle validity checks
        
        // Scenario 1: Try to use stale oracle for swap (should fail)
        let stale_validity = OracleValidity::StaleForSwap;
        assert!(!is_oracle_valid_for_action(stale_validity, Some(NormalAction::Swap)));
        
        // Scenario 2: Try to use volatile oracle for insurance claim (should fail)
        let volatile_validity = OracleValidity::TooVolatile;
        assert!(!is_oracle_valid_for_action(volatile_validity, Some(NormalAction::ClaimInsurance)));
        
        // Scenario 3: Try to use non-positive oracle for any action (should fail most)
        let non_positive_validity = OracleValidity::NonPositive;
        assert!(!is_oracle_valid_for_action(non_positive_validity, Some(NormalAction::Swap)));
        assert!(!is_oracle_valid_for_action(non_positive_validity, Some(NormalAction::UpdateTwap)));
        assert!(!is_oracle_valid_for_action(non_positive_validity, Some(NormalAction::ClaimInsurance)));
        
        // But should allow some actions
        assert!(is_oracle_valid_for_action(non_positive_validity, Some(NormalAction::AddLiquidity)));
    }

    #[test]
    fn test_oracle_validity_edge_cases() {
        // Test edge cases in oracle validity logic
        
        // Valid oracle should work for all actions
        let valid = OracleValidity::Valid;
        for action in [
            NormalAction::Swap,
            NormalAction::AddLiquidity,
            NormalAction::RemoveLiquidity,
            NormalAction::UpdateTwap,
            NormalAction::Rebalance,
            NormalAction::ClaimInsurance,
        ] {
            assert!(is_oracle_valid_for_action(valid, Some(action)));
        }
        
        // Test with no action specified (should require valid)
        assert!(is_oracle_valid_for_action(valid, None));
        assert!(!is_oracle_valid_for_action(OracleValidity::StaleForPool, None));
    }

    #[test]
    fn test_oracle_validity_attack_scenarios() {
        // Scenario 1: Attacker tries to force stale oracle acceptance
        let stale_pool = OracleValidity::StaleForPool;
        
        // Should work for liquidity operations
        assert!(is_oracle_valid_for_action(stale_pool, Some(NormalAction::AddLiquidity)));
        assert!(is_oracle_valid_for_action(stale_pool, Some(NormalAction::RemoveLiquidity)));
        
        // But not for sensitive operations
        assert!(!is_oracle_valid_for_action(stale_pool, Some(NormalAction::Swap)));
        assert!(!is_oracle_valid_for_action(stale_pool, Some(NormalAction::Rebalance)));
        
        // Scenario 2: Attacker tries to use volatile oracle
        let volatile = OracleValidity::TooVolatile;
        
        // Should work for TWAP updates (to eventually stabilize)
        assert!(is_oracle_valid_for_action(volatile, Some(NormalAction::UpdateTwap)));
        
        // But not for financial operations
        assert!(!is_oracle_valid_for_action(volatile, Some(NormalAction::Swap)));
        assert!(!is_oracle_valid_for_action(volatile, Some(NormalAction::ClaimInsurance)));
        
        // Scenario 3: Attacker tries to exploit action-specific rules
        let stale_swap = OracleValidity::StaleForSwap;
        
        // Should allow less sensitive operations
        assert!(is_oracle_valid_for_action(stale_swap, Some(NormalAction::AddLiquidity)));
        assert!(is_oracle_valid_for_action(stale_swap, Some(NormalAction::UpdateTwap)));
        assert!(is_oracle_valid_for_action(stale_swap, Some(NormalAction::ClaimInsurance)));
        
        // But block swaps and rebalancing
        assert!(!is_oracle_valid_for_action(stale_swap, Some(NormalAction::Swap)));
        assert!(!is_oracle_valid_for_action(stale_swap, Some(NormalAction::Rebalance)));
    }
}