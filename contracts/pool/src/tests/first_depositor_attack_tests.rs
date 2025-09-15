// Author: Alp Onaran
// Company: Halborn Security
// Comprehensive First Depositor Attack Tests

#![cfg(test)]

use soroban_sdk::{testutils::Address as _, vec, Env};
use crate::testutils::Setup;
use utils::constant::{MIN_LIQUIDITY, PRICE_PRECISION};

#[test]
fn test_first_depositor_direct_attack() {
    let setup = Setup::default();
    let attacker = setup.users[0].clone();
    let victim = setup.users[1].clone();
    
    // Attacker deposits minimum amount
    let min_deposit = MIN_LIQUIDITY; // 1000
    setup.liq_pool.deposit(&attacker, &min_deposit);
    
    // Attacker gets initial shares
    let attacker_shares = setup.token_share.balance(&attacker);
    assert_eq!(attacker_shares, min_deposit as i128);
    
    // Victim deposits significant amount
    let victim_deposit = 1_000_000_0000000; // 1M tokens
    setup.liq_pool.deposit(&victim, &victim_deposit);
    
    // Check victim's shares - should not be 0
    let victim_shares = setup.token_share.balance(&victim);
    assert!(victim_shares > 0, "Victim received 0 shares - attack successful!");
    
    // Calculate share value ratio
    let share_per_token_attacker = (attacker_shares * PRICE_PRECISION as i128) / min_deposit as i128;
    let share_per_token_victim = (victim_shares * PRICE_PRECISION as i128) / victim_deposit as i128;
    
    // Check for unfair share distribution
    let ratio_difference = if share_per_token_attacker > share_per_token_victim {
        ((share_per_token_attacker - share_per_token_victim) * 100) / share_per_token_victim
    } else {
        0
    };
    
    assert!(
        ratio_difference < 10, // Less than 10% difference
        "Unfair share distribution: {}% difference", 
        ratio_difference
    );
}

#[test]
fn test_first_depositor_donation_attack() {
    let setup = Setup::default();
    let attacker = setup.users[0].clone();
    let victim = setup.users[1].clone();
    
    // Step 1: Attacker deposits minimum
    setup.liq_pool.deposit(&attacker, &MIN_LIQUIDITY);
    
    // Step 2: Attacker "donates" tokens directly to increase reserve
    // This would manipulate the share price for next depositor
    let donation_amount = 1_000_000_0000000;
    
    // Get reserves before
    let reserves_before = setup.liq_pool.get_reserves();
    let reserve_b_before = reserves_before.get(1).unwrap();
    
    // In a real attack, attacker would transfer tokens directly to pool
    // This increases reserves without minting shares
    setup.token2.transfer(&attacker, &setup.liq_pool.address, &donation_amount);
    
    // Step 3: Victim deposits
    let victim_deposit = 100_000_0000000;
    setup.liq_pool.deposit(&victim, &victim_deposit);
    
    // Check victim's shares
    let victim_shares = setup.token_share.balance(&victim);
    // Get total shares from reserves calculation
    let reserves = setup.liq_pool.get_reserves();
    let total_shares = reserves.get(1).unwrap() as i128; // Approximation
    
    // Calculate expected vs actual
    let expected_shares = (victim_deposit as i128 * total_shares) / ((reserve_b_before + donation_amount) as i128);
    
    assert!(
        victim_shares >= expected_shares / 2, // At least 50% of expected
        "Victim severely diluted: got {} shares, expected ~{}", 
        victim_shares, 
        expected_shares
    );
}

#[test]
fn test_minimum_liquidity_protection() {
    let setup = Setup::default();
    let user = setup.users[0].clone();
    
    // First deposit should lock MIN_LIQUIDITY shares
    let first_deposit = 10_000_0000000;
    setup.liq_pool.deposit(&user, &first_deposit);
    
    let user_shares = setup.token_share.balance(&user);
    // Get total shares from reserves calculation
    let reserves = setup.liq_pool.get_reserves();
    let total_shares = reserves.get(1).unwrap() as i128; // Approximation
    
    // Check that MIN_LIQUIDITY is locked (burned to address 0 or kept in pool)
    assert!(
        total_shares >= MIN_LIQUIDITY as i128,
        "MIN_LIQUIDITY not properly locked"
    );
    
    // User should receive shares minus the locked amount
    assert!(
        user_shares <= (first_deposit - MIN_LIQUIDITY) as i128,
        "User received too many shares, MIN_LIQUIDITY not locked"
    );
}

#[test]
fn test_share_manipulation_via_small_deposits() {
    let setup = Setup::default();
    let attacker = setup.users[0].clone();
    let victim = setup.users[1].clone();
    
    // Attacker makes multiple small deposits to manipulate share price
    for i in 1..=10 {
        let deposit = (MIN_LIQUIDITY * i) as u128;
        setup.liq_pool.deposit(&attacker, &deposit);
    }
    
    let attacker_shares_before = setup.token_share.balance(&attacker);
    let reserves = setup.liq_pool.get_reserves();
    let reserve_b = reserves.get(1).unwrap();
    
    // Victim deposits
    let victim_deposit = 1_000_000_0000000;
    setup.liq_pool.deposit(&victim, &victim_deposit);
    
    let victim_shares = setup.token_share.balance(&victim);
    // Get total shares from reserves calculation
    let reserves = setup.liq_pool.get_reserves();
    let total_shares = reserves.get(1).unwrap() as i128; // Approximation
    
    // Calculate victim's ownership percentage
    let victim_ownership_pct = (victim_shares * 100) / total_shares;
    let victim_deposit_pct = ((victim_deposit * 100) / (reserve_b + victim_deposit)) as i128;
    
    // Ownership percentage should roughly match deposit percentage
    let difference =     if victim_ownership_pct > victim_deposit_pct {
        victim_ownership_pct - victim_deposit_pct
    } else {
        victim_deposit_pct - victim_ownership_pct
    };
    
    assert!(
        difference < 5, // Less than 5% difference
        "Share distribution unfair: {} ownership vs {} deposit",
        victim_ownership_pct,
        victim_deposit_pct
    );
}

#[test]
fn test_withdrawal_after_first_depositor_attack() {
    let setup = Setup::default();
    let attacker = setup.users[0].clone();
    let victim = setup.users[1].clone();
    
    // Setup attack scenario
    setup.liq_pool.deposit(&attacker, &MIN_LIQUIDITY);
    
    // Donate to manipulate price
    let donation = 100_000_0000000;
    setup.token2.transfer(&attacker, &setup.liq_pool.address, &donation);
    
    // Victim deposits
    let victim_deposit = 100_000_0000000;
    setup.liq_pool.deposit(&victim, &victim_deposit);
    
    let victim_shares = setup.token_share.balance(&victim);
    
    // Victim tries to withdraw immediately
    setup.liq_pool.withdraw(&victim, &(victim_shares as u128));
    
    let victim_balance_after = setup.token2.balance(&victim);
    
    // Calculate loss
    let initial_balance = 10_000_000_0000000i128; // From setup
    let loss = initial_balance - victim_balance_after;
    let loss_pct = (loss * 100) / victim_deposit as i128;
    
    assert!(
        loss_pct < 10, // Less than 10% loss
        "Victim lost {}% of deposit due to attack",
        loss_pct
    );
}

#[test]
fn test_virtual_shares_protection() {
    let setup = Setup::default();
    let user1 = setup.users[0].clone();
    let user2 = setup.users[1].clone();
    
    // First deposit with virtual shares protection
    let first_deposit = 1000_0000000;
    setup.liq_pool.deposit(&user1, &first_deposit);
    
    // Check if virtual shares are implemented
    // Get total shares from reserves calculation
    let reserves = setup.liq_pool.get_reserves();
    let total_shares = reserves.get(1).unwrap() as i128; // Approximation
    let user1_shares = setup.token_share.balance(&user1);
    
    // If virtual shares exist, total should be > user shares
    let has_virtual_shares = total_shares > user1_shares;
    
    if has_virtual_shares {
        // Second depositor should get fair shares
        let second_deposit = 1000_0000000;
        setup.liq_pool.deposit(&user2, &second_deposit);
        
        let user2_shares = setup.token_share.balance(&user2);
        
        // With same deposit, should get similar shares
        let share_ratio = if user1_shares > user2_shares {
            (user1_shares * 100) / user2_shares
        } else {
            (user2_shares * 100) / user1_shares
        };
        
        assert!(
            share_ratio < 110 && share_ratio > 90,
            "Virtual shares not providing fair distribution"
        );
    }
}

#[test]
fn test_extreme_first_deposit_scenarios() {
    let setup = Setup::default();
    let attacker = setup.users[0].clone();
    let victim = setup.users[1].clone();
    
    // Test 1: Extremely small first deposit
    let tiny_deposit = 1; // 1 unit
    
    // This might fail if MIN_LIQUIDITY is enforced
    // In no_std, we can't use catch_unwind, so we just try it
    let initial_balance = setup.token2.balance(&attacker);
    setup.liq_pool.deposit(&attacker, &tiny_deposit);
    let after_balance = setup.token2.balance(&attacker);
    
    if after_balance < initial_balance {
        // If it succeeds, check for attack potential
        let attacker_shares = setup.token_share.balance(&attacker);
        
        // Large donation
        setup.token2.transfer(&attacker, &setup.liq_pool.address, &1_000_000_0000000);
        
        // Victim deposit
        setup.liq_pool.deposit(&victim, &100_000_0000000);
        let victim_shares = setup.token_share.balance(&victim);
        
        assert!(
            victim_shares > 0,
            "Extreme first deposit enabled total share theft"
        );
    }
}

#[test]
fn test_pool_initialization_share_distribution() {
    let setup = Setup::default();
    
    // Test share distribution for first 10 depositors
    let deposits = [
        1000_0000000u128,
        5000_0000000,
        10000_0000000,
        2000_0000000,
        8000_0000000,
        3000_0000000,
        15000_0000000,
        1000_0000000,
        7000_0000000,
        4000_0000000,
    ];
    
    let mut total_deposited = 0u128;
    let mut user_shares = vec![&setup.env];
    
    for (i, deposit) in deposits.iter().enumerate() {
        let user = setup.users[i % setup.users.len()].clone();
        setup.liq_pool.deposit(&user, deposit);
        
        let shares = setup.token_share.balance(&user);
        user_shares.push_back(shares);
        total_deposited += deposit;
    }
    
    // Check share distribution fairness
    // Get total shares from reserves calculation
    let reserves = setup.liq_pool.get_reserves();
    let total_shares = reserves.get(1).unwrap() as i128; // Approximation
    
    for (i, shares) in user_shares.iter().enumerate() {
        let expected_ownership = ((deposits[i] * 100) / total_deposited) as i128;
        let actual_ownership = (*shares * 100) / total_shares;
        
        let difference = if expected_ownership > actual_ownership {
            expected_ownership - actual_ownership
        } else {
            actual_ownership - expected_ownership
        };
        
        assert!(
            difference < 10, // Less than 10% difference
            "Depositor {} has unfair share distribution: {}% difference",
            i,
            difference
        );
    }
}
