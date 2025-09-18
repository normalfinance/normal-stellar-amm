#![cfg(test)]

use crate::testutils::{Setup, TestConfig};
use soroban_sdk::testutils::Address as _;
use soroban_sdk::token::TokenClient;

#[test]
fn test_basic_deposit_withdraw() {
    let setup = Setup::new_with_config(&TestConfig::default());
    
    let user = &setup.users[0];
    let deposit_amount = 1000_0000000; // 1000 tokens
    
    // Check initial balance
    let initial_balance = setup.token1.balance(user);
    assert!(initial_balance >= deposit_amount);
    
    // Deposit
    setup.liq_pool.deposit(
        user,
        &deposit_amount,
    );
    
    // Check that tokens were transferred
    let balance_after_deposit = setup.token1.balance(user);
    assert_eq!(balance_after_deposit, initial_balance - deposit_amount);
    
    // Check shares were minted
    let shares = setup.token_share.balance(user);
    assert!(shares > 0);
    
    // Withdraw
    setup.liq_pool.withdraw(
        user,
        &shares,
    );
    
    // Check that tokens were returned (approximately)
    let final_balance = setup.token1.balance(user);
    assert!(final_balance >= initial_balance - 1000); // Allow for small rounding
}

#[test]
fn test_basic_swap() {
    let setup = Setup::new_with_config(&TestConfig::default());
    
    let user = &setup.users[0];
    let liquidity_provider = &setup.users[1];
    
    // Add liquidity first
    let lp_amount = 10000_0000000; // 10k tokens
    setup.liq_pool.deposit(
        liquidity_provider,
        &lp_amount,
    );
    
    // Perform swap
    let swap_amount = 100_0000000; // 100 tokens
    let initial_a_balance = setup.token1.balance(user);
    let initial_b_balance = setup.token2.balance(user);
    
    setup.liq_pool.swap(
        user,
        &false, // a_to_b
        &swap_amount,
        &0, // min_amount_out
    );
    
    // Check balances changed
    let final_a_balance = setup.token1.balance(user);
    let final_b_balance = setup.token2.balance(user);
    
    assert_eq!(final_a_balance, initial_a_balance - swap_amount);
    assert!(final_b_balance > initial_b_balance);
}

#[test]
fn test_multiple_users_deposit() {
    let setup = Setup::new_with_config(&TestConfig {
        users_count: 5,
        ..TestConfig::default()
    });
    
    let deposit_amount = 1000_0000000;
    
    // Multiple users deposit
    for i in 0..5 {
        let user = &setup.users[i];
        setup.liq_pool.deposit(
            user,
            &deposit_amount,
        );
        
        // Check shares were minted
        let shares = setup.token_share.balance(user);
        assert!(shares > 0);
    }
    
    // Check total shares
    let mut total_shares = 0i128;
    for i in 0..5 {
        total_shares += setup.token_share.balance(&setup.users[i]);
    }
    
    assert!(total_shares > 0);
}

#[test]
fn test_fee_collection() {
    let setup = Setup::new_with_config(&TestConfig::default());
    
    let user = &setup.users[0];
    let liquidity_provider = &setup.users[1];
    
    // Add liquidity
    let lp_amount = 10000_0000000;
    setup.liq_pool.deposit(
        liquidity_provider,
        &lp_amount,
    );
    
    // Get initial pool reserves
    let pool_info = setup.liq_pool.get_pool();
    let initial_reserve_a = pool_info.reserve_a;
    let initial_reserve_b = pool_info.reserve_b;
    
    // Perform multiple swaps to generate fees
    let swap_amount = 100_0000000;
    for _ in 0..10 {
        setup.liq_pool.swap(
            user,
            &false, // a_to_b
            &swap_amount,
            &0,
        );
        
        // Swap back
        let b_balance = setup.token2.balance(user);
        if b_balance > 0 {
            setup.liq_pool.swap(
                user,
                &true, // b_to_a
                &b_balance,
                &0,
            );
        }
    }
    
    // Check that pool accumulated fees (reserves should be slightly higher)
    let final_pool_info = setup.liq_pool.get_pool();
    let final_reserve_a = final_pool_info.reserve_a;
    let final_reserve_b = final_pool_info.reserve_b;
    
    // Due to fees, total reserves should have grown
    assert!(final_reserve_a + final_reserve_b >= initial_reserve_a + initial_reserve_b);
}

#[test]
fn test_empty_pool_initialization() {
    let setup = Setup::new_with_config(&TestConfig::default());
    
    // Check pool starts empty
    let pool_info = setup.liq_pool.get_info();
    // Note: get_info returns PoolInfo, check if it has the right fields
    // For now, just check that total shares is zero
    
    // Check total shares is zero
    let total_supply = setup.token_share.total_supply();
    assert_eq!(total_supply, 0);
}
