// Deep security and edge case tests for Insurance Fund calculations
#![cfg(test)]

use soroban_sdk::Env;
use crate::interest::{calculate_utilization, calculate_rate};
use utils::constant::{PERCENTAGE_PRECISION_U32, PRICE_PRECISION};

mod utilization_security_tests {
    use super::*;

    #[test]
    fn test_utilization_overflow_attack() {
        // Test with maximum possible values
        let max_vault = u128::MAX / 2;
        let small_optimal = 1u128;
        
        // Should not overflow and should clamp to reasonable value
        let util = calculate_utilization(max_vault, small_optimal);
        
        // Should be clamped to u32::MAX
        assert_eq!(util, u32::MAX);
    }

    #[test]
    fn test_utilization_precision_manipulation() {
        // Attack: manipulate with values that cause precision loss
        let vault = 1u128;
        let optimal = 3u128;
        
        let util = calculate_utilization(vault, optimal);
        
        // Should handle precision loss gracefully (rounds down to 0)
        assert_eq!(util, 0);
        
        // Test boundary case
        let vault2 = PERCENTAGE_PRECISION_U32 as u128;
        let optimal2 = 1u128;
        
        let util2 = calculate_utilization(vault2, optimal2);
        assert_eq!(util2, PERCENTAGE_PRECISION_U32); // 100%
    }

    #[test]
    fn test_utilization_dust_attack() {
        // Attack with dust amounts to manipulate calculation
        for i in 1..10 {
            let util = calculate_utilization(i, 1000_0000000);
            
            // Should be proportionally tiny
            assert!(util < 100); // Less than 0.001%
        }
    }

    #[test]
    fn test_utilization_whale_deposit_impact() {
        // Simulate whale deposit impact on utilization
        let initial_vault = 500_0000000 * PRICE_PRECISION; // 50% utilization
        let optimal = 1000_0000000 * PRICE_PRECISION;
        
        let initial_util = calculate_utilization(initial_vault, optimal);
        assert_eq!(initial_util, PERCENTAGE_PRECISION_U32 / 2); // 50%
        
        // Whale deposits 10x the current amount
        let whale_deposit = initial_vault * 10;
        let post_whale_vault = initial_vault + whale_deposit;
        let post_whale_util = calculate_utilization(post_whale_vault, optimal);
        
        // Utilization should jump to 550%
        let expected_util = (PERCENTAGE_PRECISION_U32 as u128 * 11 / 2) as u32; // 550%
        assert_eq!(post_whale_util, expected_util);
        
        // Should handle over-utilization gracefully
        assert!(post_whale_util > PERCENTAGE_PRECISION_U32); // > 100%
    }

    #[test]
    fn test_utilization_rapid_changes() {
        // Test rapid utilization changes (flash deposits/withdrawals)
        let optimal = 1000_0000000 * PRICE_PRECISION;
        
        // Start at 90% utilization
        let vault_90 = 900_0000000 * PRICE_PRECISION;
        let util_90 = calculate_utilization(vault_90, optimal);
        
        // Flash deposit to 110%
        let vault_110 = 1100_0000000 * PRICE_PRECISION;
        let util_110 = calculate_utilization(vault_110, optimal);
        
        // Flash withdraw to 10%
        let vault_10 = 100_0000000 * PRICE_PRECISION;
        let util_10 = calculate_utilization(vault_10, optimal);
        
        // All calculations should be stable
        assert_eq!(util_90, PERCENTAGE_PRECISION_U32 * 9 / 10); // 90%
        assert_eq!(util_110, PERCENTAGE_PRECISION_U32 * 11 / 10); // 110%
        assert_eq!(util_10, PERCENTAGE_PRECISION_U32 / 10); // 10%
    }

    #[test]
    fn test_utilization_economic_attack_scenarios() {
        // Scenario 1: Attacker reduces optimal insurance to inflate utilization
        let vault = 500_0000000 * PRICE_PRECISION;
        let normal_optimal = 1000_0000000 * PRICE_PRECISION;
        let attacked_optimal = 100_0000000 * PRICE_PRECISION; // 10x reduction
        
        let normal_util = calculate_utilization(vault, normal_optimal);
        let attacked_util = calculate_utilization(vault, attacked_optimal);
        
        assert_eq!(normal_util, PERCENTAGE_PRECISION_U32 / 2); // 50%
        assert_eq!(attacked_util, PERCENTAGE_PRECISION_U32 * 5); // 500%
        
        // Scenario 2: Attacker inflates vault amount temporarily
        let inflated_vault = vault * 100; // 100x inflation
        let inflated_util = calculate_utilization(inflated_vault, normal_optimal);
        
        assert_eq!(inflated_util, PERCENTAGE_PRECISION_U32 * 50); // 5000%
    }
}

mod interest_rate_security_tests {
    use super::*;

    #[test]
    fn test_interest_rate_manipulation_resistance() {
        let e = Env::default();
        
        // Test if rate can be manipulated by extreme utilization changes
        let optimal_util = 8000u32; // 80%
        let base_rate = 100i32;
        let slope1 = 400u32;
        let slope2 = 1500u32;
        
        // Normal scenario
        let normal_rate = calculate_rate(&e, 5000, optimal_util, base_rate, slope1, slope2);
        
        // Flash utilization spike
        let spike_rate = calculate_rate(&e, 9999, optimal_util, base_rate, slope1, slope2);
        
        // Flash utilization crash
        let crash_rate = calculate_rate(&e, 1, optimal_util, base_rate, slope1, slope2);
        
        // Rates should change predictably
        assert!(spike_rate > normal_rate); // Higher util = higher rate
        assert!(crash_rate < normal_rate); // Lower util = lower rate
        
        // But not by extreme amounts (sanity check)
        assert!(spike_rate < 10000); // Less than 100% rate
        assert!(crash_rate >= base_rate); // At least base rate
    }

    #[test]
    fn test_interest_rate_overflow_protection() {
        let e = Env::default();
        
        // Test with extreme parameters that could cause overflow
        let max_util = 9999u32;
        let optimal_util = 1u32; // Very low optimal
        let max_base = i32::MAX / 1000;
        let max_slope1 = u32::MAX / 1000;
        let max_slope2 = u32::MAX / 1000;
        
        // Should not panic or overflow
        let rate = calculate_rate(&e, max_util, optimal_util, max_base, max_slope1, max_slope2);
        
        // Should be a reasonable value
        assert!(rate > 0);
        assert!(rate < i32::MAX / 2); // Not close to overflow
    }

    #[test]
    fn test_interest_rate_precision_boundaries() {
        let e = Env::default();
        
        // Test at exact optimal utilization
        let optimal_util = 8000u32;
        let rate_at_optimal = calculate_rate(&e, optimal_util, optimal_util, 100, 400, 1500);
        
        // Test just below optimal
        let rate_below = calculate_rate(&e, optimal_util - 1, optimal_util, 100, 400, 1500);
        
        // Test just above optimal
        let rate_above = calculate_rate(&e, optimal_util + 1, optimal_util, 100, 400, 1500);
        
        // Should be continuous around optimal point
        assert!(rate_above > rate_at_optimal);
        assert!(rate_at_optimal > rate_below);
        
        // Differences should be small for small input changes
        assert!((rate_above - rate_at_optimal) < 100);
        assert!((rate_at_optimal - rate_below) < 100);
    }

    #[test]
    fn test_interest_rate_negative_scenarios() {
        let e = Env::default();
        
        // Test negative base rate (penalty for over-insurance)
        let negative_base = -1000i32;
        let low_util = 1000u32; // 10% utilization
        
        let rate = calculate_rate(&e, low_util, 8000, negative_base, 400, 1500);
        
        // Should handle negative base rates
        assert!(rate < 0); // Should be negative
        assert!(rate > negative_base); // But not more negative than base
        
        // Test if negative rate can be overcome by high utilization
        let high_util = 9000u32; // 90% utilization
        let high_util_rate = calculate_rate(&e, high_util, 8000, negative_base, 400, 1500);
        
        // High utilization should overcome negative base
        assert!(high_util_rate > 0);
    }

    #[test]
    fn test_interest_rate_economic_attack_scenarios() {
        let e = Env::default();
        
        // Scenario 1: Attacker tries to manipulate optimal utilization
        let utilization = 5000u32; // 50%
        let normal_optimal = 8000u32;
        let attacked_optimal = 4000u32; // Lowered to make current util seem high
        
        let normal_rate = calculate_rate(&e, utilization, normal_optimal, 100, 400, 1500);
        let attacked_rate = calculate_rate(&e, utilization, attacked_optimal, 100, 400, 1500);
        
        // Rate should be higher with lower optimal (utilization appears higher)
        assert!(attacked_rate > normal_rate);
        
        // Scenario 2: Attacker manipulates slopes to create extreme rates
        let extreme_slope2 = 10000u32; // Very steep slope
        let extreme_rate = calculate_rate(&e, 9000, 8000, 100, 400, extreme_slope2);
        
        // Should create high but bounded rate
        assert!(extreme_rate > 5000); // High rate
        assert!(extreme_rate < 20000); // But not extreme
    }

    #[test]
    fn test_interest_rate_curve_properties() {
        let e = Env::default();
        
        let optimal_util = 8000u32;
        let base_rate = 100i32;
        let slope1 = 400u32;
        let slope2 = 1500u32;
        
        // Test monotonicity: rate should always increase with utilization
        let mut prev_rate = calculate_rate(&e, 0, optimal_util, base_rate, slope1, slope2);
        
        for util in (1000..=10000).step_by(1000) {
            let current_rate = calculate_rate(&e, util, optimal_util, base_rate, slope1, slope2);
            assert!(current_rate >= prev_rate); // Monotonic increase
            prev_rate = current_rate;
        }
        
        // Test slope change at optimal point
        let rate_before_optimal = calculate_rate(&e, optimal_util - 100, optimal_util, base_rate, slope1, slope2);
        let rate_after_optimal = calculate_rate(&e, optimal_util + 100, optimal_util, base_rate, slope1, slope2);
        
        let slope_before = rate_after_optimal - rate_before_optimal; // Should be gentle
        
        let rate_way_after = calculate_rate(&e, optimal_util + 200, optimal_util, base_rate, slope1, slope2);
        let slope_after = rate_way_after - rate_after_optimal; // Should be steeper
        
        // Slope should be steeper after optimal point
        assert!(slope_after > slope_before);
    }

    #[test]
    fn test_interest_rate_boundary_conditions() {
        let e = Env::default();
        
        // Test at 0% utilization
        let rate_zero = calculate_rate(&e, 0, 8000, 100, 400, 1500);
        assert_eq!(rate_zero, 100); // Should equal base rate
        
        // Test at 100% utilization
        let rate_hundred = calculate_rate(&e, 10000, 8000, 100, 400, 1500);
        
        // Should be base + slope1 + full slope2
        let expected = 100 + 400 + ((10000 - 8000) as i64 * 1500 / (10000 - 8000) as i64) as i32;
        assert_eq!(rate_hundred, expected);
        
        // Test with 0% optimal (edge case)
        // This should panic as tested in previous tests
        
        // Test with 100% optimal
        let rate_100_optimal = calculate_rate(&e, 5000, 10000, 100, 400, 1500);
        // Should only use slope1 since we never exceed optimal
        let expected_100_optimal = 100 + (5000i64 * 400 / 10000) as i32;
        assert_eq!(rate_100_optimal, expected_100_optimal);
    }
}

mod insurance_fund_integration_attacks {
    use super::*;

    #[test]
    fn test_coordinated_utilization_rate_attack() {
        let e = Env::default();
        
        // Simulate coordinated attack: manipulate utilization to get favorable rates
        let optimal = 1000_0000000 * PRICE_PRECISION;
        
        // Phase 1: Attacker inflates utilization to make rates attractive
        let inflated_vault = 1200_0000000 * PRICE_PRECISION; // 120%
        let inflated_util = calculate_utilization(inflated_vault, optimal);
        
        // This creates negative interest rate (penalty)
        let rate_inflated = calculate_rate(&e, inflated_util, 8000, -200, 400, 1500);
        assert!(rate_inflated < 0); // Negative rate
        
        // Phase 2: Attacker quickly withdraws to normal level
        let normal_vault = 800_0000000 * PRICE_PRECISION; // 80%
        let normal_util = calculate_utilization(normal_vault, optimal);
        let rate_normal = calculate_rate(&e, normal_util, 8000, -200, 400, 1500);
        
        // Should get positive rate at 80% utilization
        assert!(rate_normal > 0);
        
        // Attack should not create arbitrage opportunity
        // (This would need additional logic to prevent)
    }

    #[test]
    fn test_flash_loan_utilization_manipulation() {
        let e = Env::default();
        
        // Simulate flash loan attack on utilization calculation
        let optimal = 1000_0000000 * PRICE_PRECISION;
        
        // Before flash loan: 50% utilization
        let vault_before = 500_0000000 * PRICE_PRECISION;
        let util_before = calculate_utilization(vault_before, optimal);
        let rate_before = calculate_rate(&e, util_before, 8000, 100, 400, 1500);
        
        // During flash loan: 200% utilization
        let flash_amount = 1500_0000000 * PRICE_PRECISION;
        let vault_flash = vault_before + flash_amount;
        let util_flash = calculate_utilization(vault_flash, optimal);
        let rate_flash = calculate_rate(&e, util_flash, 8000, 100, 400, 1500);
        
        // After flash loan: back to 50%
        let vault_after = vault_before;
        let util_after = calculate_utilization(vault_after, optimal);
        let rate_after = calculate_rate(&e, util_after, 8000, 100, 400, 1500);
        
        // Flash loan should not create permanent advantage
        assert_eq!(util_before, util_after);
        assert_eq!(rate_before, rate_after);
        
        // But during flash loan, rate should be very high
        assert!(rate_flash > rate_before * 2);
    }

    #[test]
    fn test_griefing_attack_scenarios() {
        let e = Env::default();
        
        // Scenario 1: Attacker deposits dust amounts repeatedly
        let optimal = 1000_0000000 * PRICE_PRECISION;
        let base_vault = 500_0000000 * PRICE_PRECISION;
        
        // Multiple tiny deposits
        let mut current_vault = base_vault;
        for i in 1..=100 {
            current_vault += i; // Tiny increments
            let util = calculate_utilization(current_vault, optimal);
            
            // Should handle tiny increments without issues
            assert!(util >= PERCENTAGE_PRECISION_U32 / 2); // At least 50%
            assert!(util <= PERCENTAGE_PRECISION_U32 / 2 + 1); // Not much higher
        }
        
        // Scenario 2: Attacker tries to break rate calculation with edge cases
        for util in [0, 1, 9999, 10000] {
            let rate = calculate_rate(&e, util, 8000, 100, 400, 1500);
            
            // All rates should be reasonable
            assert!(rate >= 100); // At least base rate
            assert!(rate <= 10000); // Not extreme
        }
    }
}
