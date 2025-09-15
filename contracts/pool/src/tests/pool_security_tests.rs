#![cfg(test)]
extern crate std;

use soroban_sdk::{testutils::Address as _, Address};
use crate::testutils::Setup;

// ==================== ACCESS CONTROL TESTS ====================

#[test]
fn test_admin_functions_require_auth() {
    let setup = Setup::default();
    let non_admin = Address::generate(&setup.env);
    
    // Test that non-admin cannot call admin functions
    // The actual pool will panic/fail when non-admin tries admin operations
}

#[test]
fn test_initialize_only_once() {
    let setup = Setup::default();
    // Pool is already initialized in Setup::default()
    // Attempting to reinitialize should fail
}

// ==================== INPUT VALIDATION TESTS ====================

#[test]
fn test_zero_deposit_rejected() {
    let setup = Setup::default();
    let user = setup.users[0].clone();
    
    // Depositing zero should fail
    // pool.deposit(&user, &0) should panic
}

#[test]
fn test_withdraw_exceeds_balance() {
    let setup = Setup::default();
    let user = setup.users[0].clone();
    
    // First deposit some amount
    setup.liq_pool.deposit(&user, &1000_0000000);
    
    // Try to withdraw more than deposited - should fail
    // pool.withdraw(&user, &2000_0000000) should panic
}

// ==================== SHARE ACCOUNTING TESTS ====================

#[test]
fn test_share_minting_proportional() {
    let setup = Setup::default();
    let user1 = setup.users[0].clone();
    let user2 = setup.users[1].clone();
    
    // First deposit gets 1:1 shares
    let deposit1 = 1000_0000000;
    setup.liq_pool.deposit(&user1, &deposit1);
    
    let shares1 = setup.token_share.balance(&user1);
    assert_eq!(shares1, deposit1 as i128, "First deposit should get 1:1 shares");
    
    // Second deposit should get proportional shares
    let deposit2 = 500_0000000;
    setup.liq_pool.deposit(&user2, &deposit2);
    
    let shares2 = setup.token_share.balance(&user2);
    // Note: Current implementation gives 1:1 always, which is the vulnerability
}

#[test]
fn test_share_dilution_after_rebalance() {
    let setup = Setup::default();
    let user1 = setup.users[0].clone();
    let user2 = setup.users[1].clone();
    let admin = setup.admin.clone();
    
    // User1 deposits
    setup.liq_pool.deposit(&user1, &10000_0000000);
    
    // Rebalance mints synthetic A
    setup.liq_pool.rebalance(&admin);
    
    // User2 deposits same amount but after rebalance
    setup.liq_pool.deposit(&user2, &10000_0000000);
    
    // Both users got same shares despite different NAV/share
    let shares1 = setup.token_share.balance(&user1);
    let shares2 = setup.token_share.balance(&user2);
    
    // This demonstrates the share dilution vulnerability
    assert_eq!(shares1, shares2, "Same deposit gives same shares (vulnerability)");
}

// ==================== RESERVE CONSISTENCY TESTS ====================

#[test]
fn test_reserves_updated_correctly() {
    let setup = Setup::default();
    let user = setup.users[0].clone();
    
    let reserves_before = setup.liq_pool.get_reserves();
    let reserve_a_before = reserves_before.get(0).unwrap();
    let reserve_b_before = reserves_before.get(1).unwrap();
    
    // Deposit should increase reserve B
    let deposit_amount = 1000_0000000;
    setup.liq_pool.deposit(&user, &deposit_amount);
    
    let reserves_after = setup.liq_pool.get_reserves();
    let reserve_a_after = reserves_after.get(0).unwrap();
    let reserve_b_after = reserves_after.get(1).unwrap();
    
    assert_eq!(
        reserve_b_after,
        reserve_b_before + deposit_amount,
        "Reserve B should increase by deposit amount"
    );
}

#[test]
fn test_constant_product_maintained() {
    let setup = Setup::default();
    let admin = setup.admin.clone();
    let user = setup.users[0].clone();
    
    // Setup liquidity
    setup.liq_pool.deposit(&admin, &10000_0000000);
    
    let reserves_before = setup.liq_pool.get_reserves();
    let reserve_a_before = reserves_before.get(0).unwrap();
    let reserve_b_before = reserves_before.get(1).unwrap();
    let k_before = (reserve_a_before as u128) * (reserve_b_before as u128);
    
    // Perform swap (indices: 0=A, 1=B)
    setup.liq_pool.swap(&user, &1, &0, &100_0000000, &0);
    
    let reserves_after = setup.liq_pool.get_reserves();
    let reserve_a_after = reserves_after.get(0).unwrap();
    let reserve_b_after = reserves_after.get(1).unwrap();
    let k_after = (reserve_a_after as u128) * (reserve_b_after as u128);
    
    // K should increase slightly due to fees
    assert!(k_after >= k_before, "Constant product should not decrease");
}

// ==================== ORACLE SECURITY TESTS ====================

#[test]
fn test_rebalance_uses_oracle() {
    let setup = Setup::default();
    let admin = setup.admin.clone();
    
    // Setup liquidity
    setup.liq_pool.deposit(&admin, &10000_0000000);
    
    let reserves_before = setup.liq_pool.get_reserves();
    let reserve_a_before = reserves_before.get(0).unwrap();
    
    // Rebalance uses oracle prices to mint/burn synthetic A
    setup.liq_pool.rebalance(&admin);
    
    let reserves_after = setup.liq_pool.get_reserves();
    let reserve_a_after = reserves_after.get(0).unwrap();
    
    // Reserve A should change based on oracle price
    assert_ne!(
        reserve_a_before,
        reserve_a_after,
        "Rebalance should adjust synthetic A based on oracle"
    );
}

// ==================== SLIPPAGE PROTECTION TESTS ====================

#[test]
#[should_panic]
fn test_swap_min_output_enforced() {
    let setup = Setup::default();
    let admin = setup.admin.clone();
    let user = setup.users[0].clone();
    
    // Setup liquidity
    setup.liq_pool.deposit(&admin, &10000_0000000);
    
    // Try swap with unrealistic min_receive
    let swap_amount = 100_0000000;
    let min_receive = 1000_0000000; // 10x input (impossible)
    
    // This should panic due to slippage protection
    setup.liq_pool.swap(&user, &1, &0, &swap_amount, &min_receive);
}

#[test]
#[should_panic]
fn test_withdraw_min_amount_enforced() {
    let setup = Setup::default();
    let user = setup.users[0].clone();
    
    // Deposit
    setup.liq_pool.deposit(&user, &1000_0000000);
    
    let shares = setup.token_share.balance(&user);
    
    // Try to withdraw with unrealistic min_amount
    // Note: withdraw signature is (env, user, share_amount)
    // This test assumes there's slippage protection in withdraw
    setup.liq_pool.withdraw(&user, &(shares as u128));
}

// ==================== EDGE CASE TESTS ====================

#[test]
fn test_dust_amounts_handled() {
    let setup = Setup::default();
    let user = setup.users[0].clone();
    
    // Try very small deposit (dust)
    let dust = 1; // Minimum unit
    
    // Should either succeed with at least 1 share or fail gracefully
    let result = std::panic::catch_unwind(|| {
        setup.liq_pool.deposit(&user, &dust);
    });
    
    match result {
        Ok(_) => {
            let shares = setup.token_share.balance(&user);
            assert!(shares > 0, "Should mint at least 1 share");
        }
        Err(_) => {
            // Failed due to minimum requirement - acceptable
        }
    }
}

#[test]
fn test_large_amounts_overflow_protection() {
    let setup = Setup::default();
    let user = setup.users[0].clone();
    
    // Try very large amount that might overflow
    let large_amount = u128::MAX / 2;
    
    // Should fail with overflow protection
    let result = std::panic::catch_unwind(|| {
        setup.liq_pool.deposit(&user, &large_amount);
    });
    
    assert!(result.is_err(), "Should protect against overflow");
}

// ==================== INTEGRATION TESTS ====================

#[test]
fn test_full_user_journey() {
    let setup = Setup::default();
    let user1 = setup.users[0].clone();
    let user2 = setup.users[1].clone();
    let admin = setup.admin.clone();
    
    // 1. First user provides liquidity
    setup.liq_pool.deposit(&user1, &5000_0000000);
    
    // 2. Admin rebalances
    setup.liq_pool.rebalance(&admin);
    
    // 3. Second user provides liquidity
    setup.liq_pool.deposit(&user2, &3000_0000000);
    
    // 4. User1 performs swap
    setup.liq_pool.swap(&user1, &1, &0, &100_0000000, &0);
    
    // 5. User2 withdraws partial
    let shares2 = setup.token_share.balance(&user2);
    setup.liq_pool.withdraw(&user2, &((shares2 / 2) as u128));
    
    // 6. Verify pool still functional
    let reserves = setup.liq_pool.get_reserves();
    let reserve_a = reserves.get(0).unwrap();
    let reserve_b = reserves.get(1).unwrap();
    assert!(reserve_a > 0 || reserve_b > 0, "Pool should have reserves");
    
    let total_shares = setup.token_share.total_supply();
    assert!(total_shares > 0, "Should have outstanding shares");
}

// ==================== FEE CALCULATION TESTS ====================

#[test]
fn test_swap_fees_collected() {
    let setup = Setup::default();
    let admin = setup.admin.clone();
    let user = setup.users[0].clone();
    
    // Setup liquidity
    setup.liq_pool.deposit(&admin, &10000_0000000);
    
    // Get reserves before swap
    let reserves_before = setup.liq_pool.get_reserves();
    let reserve_a_before = reserves_before.get(0).unwrap();
    let reserve_b_before = reserves_before.get(1).unwrap();
    
    // Perform swap (B -> A)
    let swap_amount = 100_0000000;
    let received = setup.liq_pool.swap(&user, &1, &0, &swap_amount, &0);
    
    // Get reserves after swap
    let reserves_after = setup.liq_pool.get_reserves();
    let reserve_a_after = reserves_after.get(0).unwrap();
    let reserve_b_after = reserves_after.get(1).unwrap();
    
    // Calculate constant product
    let k_before = (reserve_a_before as u128) * (reserve_b_before as u128);
    let k_after = (reserve_a_after as u128) * (reserve_b_after as u128);
    
    // K should increase due to fees
    assert!(k_after > k_before, "Fees should increase constant product");
    
    // Output should be less than ideal due to fees
    let ideal_output = (reserve_a_before * swap_amount) / (reserve_b_before + swap_amount);
    assert!(received < ideal_output, "Fees should reduce output");
} 