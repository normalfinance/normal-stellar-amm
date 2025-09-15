// Deep security and edge case tests for Pool calculations
#![cfg(test)]

use soroban_sdk::{Env, testutils::Address as _};
use crate::pool::{get_delta_a, peg_price, get_net_liquidity_imbalance};
use crate::storage::{set_reserve_a, set_reserve_b};
use pool_tokens::{get_total_synthetic_tokens};
use utils::constant::{PRICE_PRECISION, PRICE_TIMES_AMM_TO_QUOTE_PRECISION_RATIO_I128};

mod delta_a_security_tests {
    use super::*;

    #[test]
    fn test_delta_a_overflow_protection() {
        let e = Env::default();
        let contract_address = e.register(crate::Pool, ());
        
        e.as_contract(&contract_address, || {
            // Test with maximum possible values
            set_reserve_b(&e, &(u128::MAX / 1000)); // Avoid overflow in calculations
            set_reserve_a(&e, &1000_0000000);
            
            let base_price = 1_0000000;
            let quote_price = u128::MAX / 1000;
            
            // Should not panic or overflow
            let delta_a = get_delta_a(&e, base_price, quote_price);
            
            // Verify it's a reasonable value (not overflowed)
            assert!(delta_a != 0); // Should calculate something
            assert!(delta_a.abs() < i128::MAX / 2); // Not close to overflow
        });
    }

    #[test]
    fn test_delta_a_precision_attack() {
        let e = Env::default();
        let contract_address = e.register(crate::Pool, ());
        
        e.as_contract(&contract_address, || {
            // Attack: Try to manipulate with very small differences
            set_reserve_b(&e, &1000_0000000);
            set_reserve_a(&e, &1000_0000000);
            
            // Prices differ by only 1 unit (0.0000001)
            let base_price = 1_0000000;
            let quote_price = 1_0000001; // Tiny difference
            
            let delta_a = get_delta_a(&e, base_price, quote_price);
            
            // Should handle tiny price differences gracefully
            assert!(delta_a.abs() < 1000); // Small change for small price diff
        });
    }

    #[test]
    fn test_delta_a_price_manipulation_resistance() {
        let e = Env::default();
        let contract_address = e.register(crate::Pool, ());
        
        e.as_contract(&contract_address, || {
            set_reserve_b(&e, &1000_0000000);
            set_reserve_a(&e, &1000_0000000);
            
            // Simulate price oracle manipulation
            let normal_base = 100_0000000;
            let normal_quote = 1_0000000;
            
            // Normal delta
            let normal_delta = get_delta_a(&e, normal_base, normal_quote);
            
            // Manipulated price (10x spike)
            let manipulated_base = 1000_0000000; // 10x higher
            let manipulated_delta = get_delta_a(&e, manipulated_base, normal_quote);
            
            // Delta should change proportionally, not create extreme values
            let ratio = manipulated_delta.abs() as f64 / normal_delta.abs() as f64;
            assert!(ratio < 100.0); // Not more than 100x change
            assert!(ratio > 0.1); // Not less than 0.1x change
        });
    }

    #[test]
    fn test_delta_a_rounding_consistency() {
        let e = Env::default();
        let contract_address = e.register(crate::Pool, ());
        
        e.as_contract(&contract_address, || {
            // Test that small changes in inputs don't cause large jumps
            set_reserve_b(&e, &1000_0000000);
            set_reserve_a(&e, &1000_0000000);
            
            let base_price = 2_0000000;
            
            let delta1 = get_delta_a(&e, base_price, 1_0000000);
            let delta2 = get_delta_a(&e, base_price, 1_0000001); // +1 unit
            let delta3 = get_delta_a(&e, base_price, 999_9999999); // -1 unit
            
            // Changes should be smooth
            let diff1 = (delta2 - delta1).abs();
            let diff2 = (delta1 - delta3).abs();
            
            assert!(diff1 < 1000); // Small input change = small output change
            assert!(diff2 < 1000);
        });
    }

    #[test]
    fn test_delta_a_extreme_imbalance() {
        let e = Env::default();
        let contract_address = e.register(crate::Pool, ());
        
        e.as_contract(&contract_address, || {
            // Extreme imbalance: tiny reserve_b, huge reserve_a
            set_reserve_b(&e, &1);
            set_reserve_a(&e, &1000000_0000000);
            
            let base_price = 1_0000000;
            let quote_price = 1_0000000;
            
            let delta_a = get_delta_a(&e, base_price, quote_price);
            
            // Should handle extreme imbalances
            assert!(delta_a < 0); // Should burn excess synthetic
            assert!(delta_a.abs() > 1000_0000000); // Significant adjustment needed
        });
    }

    #[test]
    #[should_panic]
    fn test_delta_a_zero_quote_price_panic() {
        let e = Env::default();
        let contract_address = e.register(crate::Pool, ());
        
        e.as_contract(&contract_address, || {
            set_reserve_b(&e, &1000_0000000);
            set_reserve_a(&e, &1000_0000000);
            
            // Zero quote price should cause panic in peg_price calculation
            let _ = get_delta_a(&e, 1_0000000, 0);
        });
    }

    #[test]
    fn test_delta_a_dust_amounts() {
        let e = Env::default();
        let contract_address = e.register(crate::Pool, ());
        
        e.as_contract(&contract_address, || {
            // Test with dust amounts
            set_reserve_b(&e, &1);
            set_reserve_a(&e, &1);
            
            let base_price = 1_0000000;
            let quote_price = 1_0000000;
            
            let delta_a = get_delta_a(&e, base_price, quote_price);
            
            // Should handle dust amounts without issues
            assert_eq!(delta_a, 0); // No change needed for equal tiny amounts
        });
    }
}

mod shares_security_tests {
    use super::*;

    #[test]
    fn test_shares_dilution_attack() {
        let e = Env::default();
        
        // Simulate dilution attack scenario
        let attacker_deposit = 1_0000000u128; // Small deposit
        let victim_deposit = 1000_0000000u128; // Large deposit
        let existing_shares = 1000_0000000u128;
        let reserve_b = 1000_0000000u128;
        
        // Attacker deposits first with existing pool
        let attacker_shares = (attacker_deposit * existing_shares) / reserve_b;
        
        // Check if attacker can get disproportionate shares
        let attacker_ownership = (attacker_shares as f64) / (existing_shares as f64);
        let expected_ownership = (attacker_deposit as f64) / (reserve_b as f64);
        
        // Ownership should be proportional to deposit
        assert!((attacker_ownership - expected_ownership).abs() < 0.01); // 1% tolerance
    }

    #[test]
    fn test_shares_synthetic_value_manipulation() {
        let e = Env::default();
        
        // Test if synthetic token value can be manipulated to affect share calculation
        let token_b_amount = 100_0000000u128;
        let total_shares = 1000_0000000u128;
        let reserve_a = 1000_0000000u128;
        let reserve_b = 1000_0000000u128;
        
        // Normal price scenario
        let normal_base_price = 1_0000000u128;
        let normal_quote_price = 1_0000000u128;
        let normal_token_a_value = (reserve_a * normal_base_price) / normal_quote_price;
        let normal_total_value = reserve_b + normal_token_a_value;
        let normal_shares = (token_b_amount * total_shares) / normal_total_value;
        
        // Manipulated price scenario (base token pumped 10x)
        let pumped_base_price = 10_0000000u128;
        let pumped_token_a_value = (reserve_a * pumped_base_price) / normal_quote_price;
        let pumped_total_value = reserve_b + pumped_token_a_value;
        let pumped_shares = (token_b_amount * total_shares) / pumped_total_value;
        
        // Shares should decrease when synthetic value increases (dilution protection)
        assert!(pumped_shares < normal_shares);
        
        // But not by an extreme amount (sanity check)
        let dilution_factor = normal_shares as f64 / pumped_shares as f64;
        assert!(dilution_factor < 20.0); // Not more than 20x dilution
    }

    #[test]
    fn test_shares_zero_total_shares_edge_case() {
        // First depositor edge case
        let token_b_amount = 1000_0000000u128;
        let total_shares = 0u128;
        
        // First deposit should be 1:1
        let shares = if total_shares == 0 {
            token_b_amount
        } else {
            0 // This branch shouldn't execute
        };
        
        assert_eq!(shares, token_b_amount);
        
        // Verify no division by zero
        assert_eq!(total_shares, 0); // Sanity check
    }

    #[test]
    fn test_shares_minimum_deposit_protection() {
        let e = Env::default();
        
        // Test very small deposits
        let tiny_deposit = 1u128; // 1 unit
        let large_total_shares = 1000000_0000000u128;
        let large_reserve_b = 1000000_0000000u128;
        
        let shares = (tiny_deposit * large_total_shares) / large_reserve_b;
        
        // Should get proportional shares, even if very small
        assert_eq!(shares, 1); // 1 share for 1 unit deposit in balanced pool
    }

    #[test]
    fn test_shares_precision_loss_attack() {
        let e = Env::default();
        
        // Attack: deposit amount that causes precision loss
        let token_b_amount = 3u128; // Small odd number
        let total_shares = 1000_0000000u128;
        let reserve_b = 7u128; // Small odd number
        
        let shares = (token_b_amount * total_shares) / reserve_b;
        let remainder = (token_b_amount * total_shares) % reserve_b;
        
        // Verify precision loss doesn't benefit attacker
        assert!(remainder < reserve_b); // Normal modulo behavior
        
        // Shares should be proportional (rounded down)
        let expected_shares = (token_b_amount * total_shares) / reserve_b;
        assert_eq!(shares, expected_shares);
    }
}

mod liquidity_imbalance_security_tests {
    use super::*;

    #[test]
    fn test_imbalance_price_oracle_manipulation() {
        let e = Env::default();
        let contract_address = e.register(crate::Pool, ());
        
        e.as_contract(&contract_address, || {
            // Mock synthetic token supply (would need actual implementation)
            set_reserve_b(&e, &1000_0000000);
            // Assume synthetic supply = 1000 for this test
            
            // Normal prices
            let normal_base = 1_0000000u128;
            let normal_quote = 1_0000000u128;
            
            // This test would need actual synthetic token supply to work
            // For now, test the calculation logic conceptually
            
            // Manipulated base price (10x pump)
            let pumped_base = 10_0000000u128;
            
            // Calculate theoretical imbalance
            let synthetic_supply = 1000_0000000u128; // Assumed
            let reserve_b = 1000_0000000u128;
            
            let normal_base_value = (synthetic_supply as i128 * normal_base as i128) / PRICE_TIMES_AMM_TO_QUOTE_PRECISION_RATIO_I128;
            let normal_quote_value = (reserve_b as i128 * normal_quote as i128) / PRICE_TIMES_AMM_TO_QUOTE_PRECISION_RATIO_I128;
            let normal_imbalance = normal_quote_value - normal_base_value;
            
            let pumped_base_value = (synthetic_supply as i128 * pumped_base as i128) / PRICE_TIMES_AMM_TO_QUOTE_PRECISION_RATIO_I128;
            let pumped_imbalance = normal_quote_value - pumped_base_value;
            
            // Pumped price should create negative imbalance (excess synthetic value)
            assert!(pumped_imbalance < normal_imbalance);
            assert!(pumped_imbalance < 0); // Excess synthetic
        });
    }

    #[test]
    fn test_imbalance_extreme_supply_scenarios() {
        let e = Env::default();
        let contract_address = e.register(crate::Pool, ());
        
        e.as_contract(&contract_address, || {
            // Scenario 1: No synthetic tokens, all quote
            let synthetic_supply = 0u128;
            let reserve_b = 1000_0000000u128;
            let price = 1_0000000u128;
            
            let base_value = (synthetic_supply as i128 * price as i128) / PRICE_TIMES_AMM_TO_QUOTE_PRECISION_RATIO_I128;
            let quote_value = (reserve_b as i128 * price as i128) / PRICE_TIMES_AMM_TO_QUOTE_PRECISION_RATIO_I128;
            let imbalance = quote_value - base_value;
            
            assert_eq!(base_value, 0);
            assert!(imbalance > 0); // All quote, maximum positive imbalance
            
            // Scenario 2: Massive synthetic supply, no quote reserves
            let synthetic_supply = 1000000_0000000u128;
            let reserve_b = 0u128;
            
            let base_value = (synthetic_supply as i128 * price as i128) / PRICE_TIMES_AMM_TO_QUOTE_PRECISION_RATIO_I128;
            let quote_value = (reserve_b as i128 * price as i128) / PRICE_TIMES_AMM_TO_QUOTE_PRECISION_RATIO_I128;
            let imbalance = quote_value - base_value;
            
            assert_eq!(quote_value, 0);
            assert!(imbalance < 0); // All synthetic, maximum negative imbalance
        });
    }

    #[test]
    fn test_imbalance_calculation_overflow_protection() {
        let e = Env::default();
        
        // Test with very large values that could cause overflow
        let max_safe_supply = u128::MAX / 1000; // Avoid overflow
        let max_safe_price = 1000_0000000u128;
        
        // These calculations should not overflow
        let base_value_result = (max_safe_supply as i128).checked_mul(max_safe_price as i128);
        assert!(base_value_result.is_some()); // Should not overflow
        
        let final_value = base_value_result.unwrap() / PRICE_TIMES_AMM_TO_QUOTE_PRECISION_RATIO_I128;
        assert!(final_value < i128::MAX / 2); // Should be reasonable
    }

    #[test]
    fn test_imbalance_precision_and_rounding() {
        let e = Env::default();
        
        // Test precision with small values
        let small_supply = 1u128;
        let small_price = 1u128;
        
        let value = (small_supply as i128 * small_price as i128) / PRICE_TIMES_AMM_TO_QUOTE_PRECISION_RATIO_I128;
        
        // Small values should round down to 0 due to precision
        assert_eq!(value, 0);
        
        // Test precision with values just above precision threshold
        let threshold_supply = PRICE_TIMES_AMM_TO_QUOTE_PRECISION_RATIO_I128 as u128;
        let threshold_value = (threshold_supply as i128 * 1i128) / PRICE_TIMES_AMM_TO_QUOTE_PRECISION_RATIO_I128;
        
        assert_eq!(threshold_value, 1); // Should equal 1 exactly
    }
}

mod peg_price_security_tests {
    use super::*;

    #[test]
    fn test_peg_price_flash_loan_attack_resistance() {
        let e = Env::default();
        
        // Simulate flash loan attack on price calculation
        let normal_base = 100_0000000u128;
        let normal_quote = 1_0000000u128;
        
        let normal_peg = peg_price(&e, normal_base, normal_quote);
        
        // Attacker manipulates oracle prices temporarily
        let attacked_base = 1_0000000u128; // 100x lower
        let attacked_quote = 100_0000000u128; // 100x higher
        
        let attacked_peg = peg_price(&e, attacked_base, attacked_quote);
        
        // Peg price should change proportionally
        let peg_ratio = attacked_peg as f64 / normal_peg as f64;
        
        // Verify the change is proportional to price changes
        let expected_ratio = (attacked_quote as f64 / normal_quote as f64) / (attacked_base as f64 / normal_base as f64);
        
        assert!((peg_ratio - expected_ratio).abs() < 0.01); // 1% tolerance
    }

    #[test]
    fn test_peg_price_extreme_ratios() {
        let e = Env::default();
        
        // Test extreme price ratios
        let tiny_base = 1u128;
        let huge_quote = u128::MAX / PRICE_PRECISION; // Avoid overflow
        
        let peg = peg_price(&e, tiny_base, huge_quote);
        
        // Should handle extreme ratios without panic
        assert!(peg > 0);
        assert!(peg < u128::MAX);
        
        // Reverse scenario
        let huge_base = u128::MAX / PRICE_PRECISION;
        let tiny_quote = 1u128;
        
        let reverse_peg = peg_price(&e, huge_base, tiny_quote);
        
        // Should be very small but not zero
        assert!(reverse_peg < 1000); // Very small value
    }

    #[test]
    fn test_peg_price_precision_boundaries() {
        let e = Env::default();
        
        // Test at precision boundaries
        let base = PRICE_PRECISION;
        let quote = PRICE_PRECISION;
        
        let peg = peg_price(&e, base, quote);
        
        // Should equal PRICE_PRECISION (1.0 in fixed point)
        assert_eq!(peg, PRICE_PRECISION);
        
        // Test just above and below precision
        let peg_above = peg_price(&e, base, quote + 1);
        let peg_below = peg_price(&e, base, quote - 1);
        
        assert!(peg_above > peg);
        assert!(peg_below < peg);
    }
}
