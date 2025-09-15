#![cfg(test)]
extern crate std;

use crate::testutils::Setup;
use soroban_sdk::{testutils::Address as _, Address};

// Basic functionality tests that should compile and run

#[test]
fn test_pool_initialization() {
    let setup = Setup::default();
    assert!(setup.liq_pool.address != Address::generate(&setup.env));
}

#[test]
fn test_deposit_increases_reserves() {
    let setup = Setup::default();
    let user = setup.users[0].clone();
    
    let reserves_before = setup.liq_pool.get_reserves();
    
    // Deposit some amount
    let deposit_amount = 1000_0000000;
    setup.liq_pool.deposit(&user, &deposit_amount);
    
    let reserves_after = setup.liq_pool.get_reserves();
    
    // Reserves should increase
    assert!(reserves_after.get(1).unwrap() > reserves_before.get(1).unwrap_or(0));
}

#[test]
fn test_share_minting() {
    let setup = Setup::default();
    let user = setup.users[0].clone();
    
    // Check initial balance
    let initial_shares = setup.token_share.balance(&user);
    assert_eq!(initial_shares, 0);
    
    // Deposit
    setup.liq_pool.deposit(&user, &1000_0000000);
    
    // Check shares were minted
    let shares_after = setup.token_share.balance(&user);
    assert!(shares_after > 0);
}

#[test]
fn test_withdrawal() {
    let setup = Setup::default();
    let user = setup.users[0].clone();
    
    // First deposit
    setup.liq_pool.deposit(&user, &1000_0000000);
    
    // Get shares
    let shares = setup.token_share.balance(&user);
    assert!(shares > 0);
    
    // Withdraw half
    let withdraw_amount = (shares / 2) as u128;
    setup.liq_pool.withdraw(&user, &withdraw_amount);
    
    // Check shares were burned
    let remaining_shares = setup.token_share.balance(&user);
    assert_eq!(remaining_shares, shares - (withdraw_amount as i128));
}

#[test]
fn test_swap() {
    let setup = Setup::default();
    let admin = setup.admin.clone();
    let user = setup.users[0].clone();
    
    // Setup liquidity
    setup.liq_pool.deposit(&admin, &10000_0000000);
    
    // Get initial token balance
    let token1_before = setup.token1.balance(&user);
    
    // Perform swap (B -> A): in_idx=1, out_idx=0
    let swap_amount = 100_0000000;
    let received = setup.liq_pool.swap(&user, &1, &0, &swap_amount, &0);
    
    // Check received tokens
    assert!(received > 0);
    
    let token1_after = setup.token1.balance(&user);
    assert!(token1_after > token1_before);
}

#[test]
fn test_rebalance() {
    let setup = Setup::default();
    let admin = setup.admin.clone();
    
    // Setup liquidity
    setup.liq_pool.deposit(&admin, &10000_0000000);
    
    let reserves_before = setup.liq_pool.get_reserves();
    
    // Rebalance
    setup.liq_pool.rebalance(&admin);
    
    let reserves_after = setup.liq_pool.get_reserves();
    
    // Reserves should change (synthetic A minted or burned)
    assert!(
        reserves_after.get(0) != reserves_before.get(0),
        "Rebalance should affect synthetic token reserves"
    );
}

#[test]
fn test_multiple_depositors() {
    let setup = Setup::default();
    let user1 = setup.users[0].clone();
    let user2 = setup.users[1].clone();
    
    // First deposit
    setup.liq_pool.deposit(&user1, &1000_0000000);
    let shares1 = setup.token_share.balance(&user1);
    
    // Second deposit
    setup.liq_pool.deposit(&user2, &500_0000000);
    let shares2 = setup.token_share.balance(&user2);
    
    // Both should have shares
    assert!(shares1 > 0);
    assert!(shares2 > 0);
    
    // First depositor should have more shares (deposited more)
    assert!(shares1 > shares2);
}

#[test]
fn test_share_dilution_scenario() {
    let setup = Setup::default();
    let user1 = setup.users[0].clone();
    let user2 = setup.users[1].clone();
    let admin = setup.admin.clone();
    
    // User1 deposits before rebalance
    setup.liq_pool.deposit(&user1, &10000_0000000);
    let shares1 = setup.token_share.balance(&user1);
    
    // Rebalance (may mint synthetic tokens)
    setup.liq_pool.rebalance(&admin);
    
    // User2 deposits same amount after rebalance
    setup.liq_pool.deposit(&user2, &10000_0000000);
    let shares2 = setup.token_share.balance(&user2);
    
    // In current implementation, both get same shares (vulnerability)
    // This demonstrates the share dilution issue
    assert_eq!(
        shares1, shares2,
        "Same deposit amount gives same shares regardless of NAV (vulnerability)"
    );
}

#[test]
#[should_panic]
fn test_zero_deposit_fails() {
    let setup = Setup::default();
    let user = setup.users[0].clone();
    
    // This should panic
    setup.liq_pool.deposit(&user, &0);
}

#[test]
#[should_panic]
fn test_withdraw_more_than_balance() {
    let setup = Setup::default();
    let user = setup.users[0].clone();
    
    // Deposit some
    setup.liq_pool.deposit(&user, &1000_0000000);
    
    let shares = setup.token_share.balance(&user);
    
    // Try to withdraw more than we have
    setup.liq_pool.withdraw(&user, &((shares * 2) as u128));
}

#[test]
fn test_constant_product_with_fees() {
    let setup = Setup::default();
    let admin = setup.admin.clone();
    let user = setup.users[0].clone();
    
    // Setup liquidity
    setup.liq_pool.deposit(&admin, &10000_0000000);
    
    let reserves_before = setup.liq_pool.get_reserves();
    let reserve_a_before = reserves_before.get(0).unwrap_or(0);
    let reserve_b_before = reserves_before.get(1).unwrap_or(0);
    let k_before = reserve_a_before * reserve_b_before;
    
    // Perform swap
    setup.liq_pool.swap(&user, &1, &0, &100_0000000, &0);
    
    let reserves_after = setup.liq_pool.get_reserves();
    let reserve_a_after = reserves_after.get(0).unwrap_or(0);
    let reserve_b_after = reserves_after.get(1).unwrap_or(0);
    let k_after = reserve_a_after * reserve_b_after;
    
    // K should increase due to fees
    assert!(k_after >= k_before, "Constant product should not decrease");
} 