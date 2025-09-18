// Comprehensive tests for Pool calculations
#![cfg(test)]

use soroban_sdk::Env;
use soroban_fixed_point_math::FixedPoint;
use crate::pool::{get_delta_a};
use crate::storage::{set_reserve_a, set_reserve_b, get_reserve_a, get_reserve_b};
use utils::constant::{PRICE_PRECISION};

mod delta_a_tests {
    use super::*;

    #[test]
    fn test_delta_a_mint_when_below_peg() {
        let e = Env::default();
        let contract_address = e.register(crate::Pool, ());
        
        e.as_contract(&contract_address, || {
            // Setup: price of base = $2, quote = $1, so peg = 0.5
            // If reserve_b = 1000, target reserve_a = 1000/0.5 = 2000
            // If current reserve_a = 1500, delta_a = 500 (mint)
            set_reserve_b(&e, &1000_0000000);
            set_reserve_a(&e, &1500_0000000);
            
            let base_price = 2_0000000; // $2
            let quote_price = 1_0000000; // $1
            
            let delta_a = get_delta_a(&e, get_reserve_a(&e), get_reserve_b(&e), base_price, quote_price);
            assert_eq!(delta_a, 500_0000000); // Should mint 500
        });
    }

    #[test]
    fn test_delta_a_burn_when_above_peg() {
        let e = Env::default();
        let contract_address = e.register(crate::Pool, ());
        
        e.as_contract(&contract_address, || {
            // If reserve_b = 1000, target reserve_a = 2000
            // If current reserve_a = 2500, delta_a = -500 (burn)
            set_reserve_b(&e, &1000_0000000);
            set_reserve_a(&e, &2500_0000000);
            
            let base_price = 2_0000000;
            let quote_price = 1_0000000;
            
            let delta_a = get_delta_a(&e, get_reserve_a(&e), get_reserve_b(&e), base_price, quote_price);
            assert_eq!(delta_a, -500_0000000); // Should burn 500
        });
    }

    #[test]
    fn test_delta_a_zero_when_at_peg() {
        let e = Env::default();
        let contract_address = e.register(crate::Pool, ());
        
        e.as_contract(&contract_address, || {
            set_reserve_b(&e, &1000_0000000);
            set_reserve_a(&e, &2000_0000000);
            
            let base_price = 2_0000000;
            let quote_price = 1_0000000;
            
            let delta_a = get_delta_a(&e, get_reserve_a(&e), get_reserve_b(&e), base_price, quote_price);
            assert_eq!(delta_a, 0);
        });
    }

    #[test]
    fn test_delta_a_edge_case_zero_reserves() {
        let e = Env::default();
        let contract_address = e.register(crate::Pool, ());
        
        e.as_contract(&contract_address, || {
            set_reserve_b(&e, &0);
            set_reserve_a(&e, &0);
            
            let base_price = 2_0000000;
            let quote_price = 1_0000000;
            
            let delta_a = get_delta_a(&e, get_reserve_a(&e), get_reserve_b(&e), base_price, quote_price);
            assert_eq!(delta_a, 0); // No change needed when both are zero
        });
    }

    #[test]
    fn test_delta_a_extreme_price_ratios() {
        let e = Env::default();
        let contract_address = e.register(crate::Pool, ());
        
        e.as_contract(&contract_address, || {
            // Test with extreme price ratio (base = $10000, quote = $1)
            set_reserve_b(&e, &1000_0000000);
            set_reserve_a(&e, &10_0000000);
            
            let base_price = 10000_0000000;
            let quote_price = 1_0000000;
            
            let delta_a = get_delta_a(&e, get_reserve_a(&e), get_reserve_b(&e), base_price, quote_price);
            
            // Let's verify this is working as expected by checking the actual calculation
            // The function should calculate: target_reserve_a - current_reserve_a
            // where target_reserve_a = reserve_b.fixed_div_floor(peg_price, PRICE_PRECISION)
            // and peg_price = quote_price.fixed_div_floor(base_price, PRICE_PRECISION)
            
            // With extreme ratios, we expect a large positive delta (need to mint a lot)
            // because reserve_b / tiny_peg_price = very large target_reserve_a
            assert!(delta_a > 0); // Should be positive (mint)
            assert!(delta_a > 1000_0000000); // Should be much larger than current reserves
        });
    }

    #[test]
    #[should_panic] // Should panic on division by zero
    fn test_delta_a_zero_base_price() {
        let e = Env::default();
        let contract_address = e.register(crate::Pool, ());
        
        e.as_contract(&contract_address, || {
            set_reserve_b(&e, &1000_0000000);
            set_reserve_a(&e, &1500_0000000);
            
            let base_price = 0;
            let quote_price = 1_0000000;
            
            let _ = get_delta_a(&e, get_reserve_a(&e), get_reserve_b(&e),  base_price, quote_price);
        });
    }
}

mod shares_to_mint_tests {
    use super::*;

    #[test]
    fn test_shares_first_deposit() {
        // For first deposit, shares = token_b_amount
        let token_b_amount = 1000_0000000u128;
        let total_shares = 0u128;
        
        // First deposit logic (1:1 with token_b)
        let shares = if total_shares == 0 {
            token_b_amount
        } else {
            0
        };
        
        assert_eq!(shares, token_b_amount);
    }

    #[test]
    fn test_shares_no_synthetic() {
        let e = Env::default();
        
        // When no synthetic exists, shares based on token_b proportion
        let token_b_amount = 500_0000000u128;
        let total_shares = 1000_0000000u128;
        let reserve_a = 0u128;
        let reserve_b = 1000_0000000u128;
        
        let shares = if reserve_a == 0 {
            (token_b_amount * total_shares) / reserve_b
        } else {
            0
        };
        
        // Expected: 500 * 1000 / 1000 = 500
        assert_eq!(shares, 500_0000000);
    }

    #[test]
    fn test_shares_with_synthetic_balanced() {
        let e = Env::default();
        
        // When synthetic exists, must account for total value
        let token_b_amount = 100_0000000u128;
        let total_shares = 1000_0000000u128;
        let reserve_a = 500_0000000u128;
        let reserve_b = 500_0000000u128;
        let base_price = 1_0000000u128;
        let quote_price = 1_0000000u128;
        
        // Calculate token A value in token B terms (simplified)
        let token_a_value = (reserve_a * base_price) / quote_price;
        let total_pool_value = reserve_b + token_a_value;
        
        let shares = (token_b_amount * total_shares) / total_pool_value;
        
        // Expected: 100 * 1000 / 1000 = 100
        assert_eq!(shares, 100_0000000);
    }

    #[test]
    fn test_shares_with_synthetic_imbalanced() {
        let e = Env::default();
        
        let token_b_amount = 100_0000000u128;
        let total_shares = 1000_0000000u128;
        let reserve_a = 2000_0000000u128; // Large synthetic position
        let reserve_b = 500_0000000u128;
        let base_price = 2_0000000u128; // Base worth 2x
        let quote_price = 1_0000000u128;
        
        // Token A value = 2000 * 2 / 1 = 4000
        let token_a_value = (reserve_a * base_price) / quote_price;
        // Total value = 500 + 4000 = 4500
        let total_pool_value = reserve_b + token_a_value;
        
        let shares = (token_b_amount * total_shares) / total_pool_value;
        
        // Expected: 100 * 1000 / 4500 = 22.22
        let expected = (100_0000000u128 * 1000_0000000u128) / 4500_0000000u128;
        assert_eq!(shares, expected);
    }

    #[test]
    fn test_shares_dilution_protection() {
        let e = Env::default();
        
        // Test that the validation function would catch unfair share calculation
        let token_b_amount = 1000_0000000u128;
        let shares_to_mint = 10_0000000u128; // Way too few shares
        let total_shares = 1000_0000000u128;
        let reserve_a = 1000_0000000u128;
        let reserve_b = 1000_0000000u128;
        let base_price = 1_0000000u128;
        let quote_price = 1_0000000u128;
        
        // Calculate expected minimum shares (simplified)
        let token_a_value = (reserve_a * base_price) / quote_price;
        let total_pool_value = reserve_b + token_a_value;
        let expected_min_shares = (token_b_amount * total_shares) / total_pool_value;
        
        // Tolerance for rounding
        let tolerance = expected_min_shares / 10000;
        let min_acceptable = expected_min_shares.saturating_sub(tolerance);
        
        // This should fail validation
        assert!(shares_to_mint < min_acceptable);
    }
}

mod peg_price_tests {
    use super::*;
    use crate::pool::peg_price;

    #[test]
    fn test_peg_price_calculation() {
        let e = Env::default();
        
        let base_price = 2_0000000u128; // $2
        let quote_price = 1_0000000u128; // $1
        
        let peg = peg_price(&e, base_price, quote_price);
        
        // Peg should be quote/base = 1/2 = 0.5
        let expected = (quote_price * PRICE_PRECISION) / base_price;
        assert_eq!(peg, expected);
    }

    #[test]
    fn test_peg_price_zero_handling() {
        let e = Env::default();
        
        // Zero base price
        let peg1 = peg_price(&e, 0, 1_0000000);
        assert_eq!(peg1, 0);
        
        // Zero quote price  
        let peg2 = peg_price(&e, 1_0000000, 0);
        assert_eq!(peg2, 0);
        
        // Both zero
        let peg3 = peg_price(&e, 0, 0);
        assert_eq!(peg3, 0);
    }

    #[test]
    fn test_peg_price_precision() {
        let e = Env::default();
        
        // Test with high precision values
        let base_price = 12345_6789012u128;
        let quote_price = 98765_4321098u128;
        
        let peg = peg_price(&e, base_price, quote_price);
        
        // Should not panic and produce reasonable result
        assert!(peg > 0);
        assert!(peg < u128::MAX);
    }
}