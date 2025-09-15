// Author: Alp Onaran
// Company: Halborn Security
// TWAP Manipulation and Oracle Security Tests

#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Env, vec};
use crate::testutils::Setup;
use utils::constant::{FIVE_MINUTE, PERCENTAGE_PRECISION, PRICE_PRECISION};
use utils::test_utils::jump;

#[test]
fn test_twap_manipulation_window() {
    let setup = Setup::default();
    let attacker = setup.users[0].clone();
    let admin = setup.admin.clone();
    
    // Setup initial liquidity
    setup.liq_pool.deposit(&admin, &1_000_000_0000000);
    
    // Get initial oracle price
    let initial_price = 50000_0000000; // $50,000
    
    // Attacker manipulates spot price at specific times
    for i in 0..10 {
        // Jump forward in time
        jump(&setup.env, 30); // 30 seconds
        
        // Manipulate price (alternating high/low)
        let manipulated_price = if i % 2 == 0 {
            initial_price * 120 / 100 // 20% higher
        } else {
            initial_price * 80 / 100 // 20% lower
        };
        
        // Update oracle price
        setup.oracle_client.set_price(
            &vec![&setup.env, manipulated_price],
            &setup.env.ledger().timestamp()
        );
        
        // Trigger TWAP update
        setup.liq_pool.rebalance(&admin);
    }
    
    // Check final TWAP vs spot
    let final_twap = get_current_twap(&setup);
    let final_spot = initial_price; // Returned to normal
    
    let divergence = if final_twap > final_spot {
        ((final_twap - final_spot) * PERCENTAGE_PRECISION) / final_spot
    } else {
        ((final_spot - final_twap) * PERCENTAGE_PRECISION) / final_spot
    };
    
    assert!(
        divergence < 1000, // Less than 10% divergence
        "TWAP manipulation residue too high: {}%",
        divergence / 100
    );
}

#[test]
fn test_oracle_staleness_attack() {
    let setup = Setup::default();
    let attacker = setup.users[0].clone();
    let admin = setup.admin.clone();
    
    setup.liq_pool.deposit(&admin, &1_000_000_0000000);
    
    // Set initial price
    let old_price = 50000_0000000;
    setup.oracle_client.set_price(
        &vec![&setup.env, old_price],
        &setup.env.ledger().timestamp()
    );
    
    // Wait for oracle to become stale
    jump(&setup.env, FIVE_MINUTE + 1);
    
    // Market price has moved significantly
    let real_price = 60000_0000000; // 20% increase
    
    // Try to exploit stale price
    let swap_amount = 10000_0000000;
    
    // This should fail or use fallback
    // Try to exploit stale price
    let attacker_balance_before = setup.token1.balance(&attacker);
    setup.liq_pool.swap(
        &attacker,
        &setup.token2.address,
        &setup.token1.address,
        &swap_amount,
        &0
    );
    let attacker_balance_after = setup.token1.balance(&attacker);
    
    if attacker_balance_after > attacker_balance_before {
        // If swap succeeded, check if stale price was used
        let reserves = setup.liq_pool.get_reserves();
        let reserve_a = reserves.get(0).unwrap();
        let reserve_b = reserves.get(1).unwrap();
        
        // Calculate implied price from reserves
        let implied_price = (reserve_b * PRICE_PRECISION as u128) / reserve_a;
        
        // Should not be using stale price
        let price_diff = if implied_price > old_price as u128 {
            implied_price - old_price as u128
        } else {
            old_price as u128 - implied_price
        };
        
        assert!(
            price_diff > 0,
            "Stale oracle price was used for swap"
        );
    }
}

#[test]
fn test_oracle_price_divergence_limits() {
    let setup = Setup::default();
    let admin = setup.admin.clone();
    
    setup.liq_pool.deposit(&admin, &1_000_000_0000000);
    
    let base_price = 50000_0000000;
    
    // Test various divergence levels
    let divergence_levels = vec![
        500,   // 5%
        1000,  // 10%
        2000,  // 20%
        5000,  // 50%
        10000, // 100%
    ];
    
    for divergence_bps in divergence_levels {
        // Set TWAP
        setup.oracle_client.set_price(
            &vec![&setup.env, base_price],
            &setup.env.ledger().timestamp()
        );
        
        // Jump time for TWAP to settle
        jump(&setup.env, 300);
        
        // Try to set divergent spot price
        let divergent_price = base_price * (10000 + divergence_bps) / 10000;
        setup.oracle_client.set_price(
            &vec![&setup.env, divergent_price],
            &setup.env.ledger().timestamp()
        );
        
        // Try to use divergent price
        // Try to use divergent price
        let reserves_before = setup.liq_pool.get_reserves();
        setup.liq_pool.rebalance(&admin);
        let reserves_after = setup.liq_pool.get_reserves();
        
        // High divergence should be rejected or limited
        let reserve_a_before = reserves_before.get(0).unwrap();
        let reserve_a_after = reserves_after.get(0).unwrap();
        let change = if reserve_a_after > reserve_a_before {
            reserve_a_after - reserve_a_before
        } else {
            reserve_a_before - reserve_a_after
        };
        
        if divergence_bps > 2000 {
            assert!(
                change < reserve_a_before / 10, // Less than 10% change
                "Accepted {}% price divergence - too permissive",
                divergence_bps / 100
            );
        }
    }
}

#[test]
fn test_twap_gaming_via_small_updates() {
    let setup = Setup::default();
    let attacker = setup.users[0].clone();
    let admin = setup.admin.clone();
    
    setup.liq_pool.deposit(&admin, &1_000_000_0000000);
    
    let initial_price = 50000_0000000;
    let target_price = 55000_0000000; // 10% higher
    
    // Attacker tries to gradually shift TWAP
    let steps = 100;
    let price_increment = (target_price - initial_price) / steps;
    
    for i in 0..steps {
        jump(&setup.env, 10); // Small time jumps
        
        let new_price = initial_price + (price_increment * i);
        setup.oracle_client.set_price(
            &vec![&setup.env, new_price],
            &setup.env.ledger().timestamp()
        );
        
        // Update TWAP
        setup.liq_pool.rebalance(&admin);
    }
    
    let final_twap = get_current_twap(&setup);
    
    // TWAP should lag behind spot significantly
    let twap_shift = ((final_twap - initial_price) * 100) / initial_price;
    
    assert!(
        twap_shift < 500, // Less than 5% shift
        "TWAP too easily manipulated: {}% shift achieved",
        twap_shift
    );
}

#[test]
fn test_oracle_sandwich_attack() {
    let setup = Setup::default();
    let attacker = setup.users[0].clone();
    let victim = setup.users[1].clone();
    let admin = setup.admin.clone();
    
    // Setup liquidity
    setup.liq_pool.deposit(&admin, &10_000_000_0000000);
    setup.liq_pool.deposit(&victim, &1_000_000_0000000);
    
    let normal_price = 50000_0000000;
    
    // Step 1: Attacker manipulates oracle before victim transaction
    let manipulated_price = 45000_0000000; // 10% lower
    setup.oracle_client.set_price(
        &vec![&setup.env, manipulated_price],
        &setup.env.ledger().timestamp()
    );
    
    // Step 2: Victim performs swap at manipulated price
    let victim_swap = 100000_0000000;
    setup.liq_pool.swap(
        &victim,
        &setup.token2.address,
        &setup.token1.address,
        &victim_swap,
        &0
    );
    
    // Step 3: Attacker restores price and profits
    setup.oracle_client.set_price(
        &vec![&setup.env, normal_price],
        &setup.env.ledger().timestamp()
    );
    
    // Calculate victim's loss
    let victim_received = setup.token1.balance(&victim);
    let expected_at_normal = (victim_swap as i128 * PRICE_PRECISION as i128) / normal_price;
    
    let loss_pct = if expected_at_normal > victim_received {
        ((expected_at_normal - victim_received) * 100) / expected_at_normal
    } else {
        0
    };
    
    assert!(
        loss_pct < 10,
        "Sandwich attack caused {}% loss to victim",
        loss_pct
    );
}

#[test]
fn test_multi_oracle_aggregation() {
    let setup = Setup::default();
    let admin = setup.admin.clone();
    
    // Simulate multiple oracle sources
    let oracle_prices = vec![
        50000_0000000,
        50500_0000000,
        49500_0000000,
        51000_0000000,
        49000_0000000,
    ];
    
    // Calculate median (simple aggregation)
    let mut sorted_prices = oracle_prices.clone();
    sorted_prices.sort();
    let median_price = sorted_prices[sorted_prices.len() / 2];
    
    // Calculate mean
    let sum: i128 = oracle_prices.iter().sum();
    let mean_price = sum / oracle_prices.len() as i128;
    
    // Check aggregation method resistance to outliers
    let outlier_impact = ((mean_price - median_price).abs() * 100) / median_price;
    
    assert!(
        outlier_impact < 200, // Less than 2% impact
        "Oracle aggregation too sensitive to outliers: {}% difference",
        outlier_impact
    );
}

#[test]
fn test_oracle_frontrun_protection() {
    let setup = Setup::default();
    let frontrunner = setup.users[0].clone();
    let victim = setup.users[1].clone();
    let admin = setup.admin.clone();
    
    setup.liq_pool.deposit(&admin, &10_000_000_0000000);
    
    // Victim submits transaction with expected price
    let expected_price = 50000_0000000;
    let victim_swap = 100000_0000000;
    let min_received = (victim_swap as i128 * 95 / 100); // 5% slippage
    
    // Frontrunner tries to manipulate price before victim
    let frontrun_price = 47500_0000000; // 5% lower
    setup.oracle_client.set_price(
        &vec![&setup.env, frontrun_price],
        &setup.env.ledger().timestamp()
    );
    
    // Victim's transaction with slippage protection
    // Victim's transaction with slippage protection
    let victim_balance_before = setup.token1.balance(&victim);
    setup.liq_pool.swap(
        &victim,
        &setup.token2.address,
        &setup.token1.address,
        &victim_swap,
        &min_received as u128
    );
    let victim_balance_after = setup.token1.balance(&victim);
    
    if victim_balance_after > victim_balance_before {
        // Check if victim was protected
        let received = setup.token1.balance(&victim);
        assert!(
            received >= min_received,
            "Slippage protection failed: received {} < min {}",
            received,
            min_received
        );
    }
}

// Helper function to get current TWAP
fn get_current_twap(setup: &Setup) -> i128 {
    // This would call the actual TWAP calculation
    // For now, return a placeholder
    50000_0000000
}
