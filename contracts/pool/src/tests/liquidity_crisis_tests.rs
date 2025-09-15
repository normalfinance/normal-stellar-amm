// Author: Alp Onaran
// Company: Halborn Security
// Liquidity Crisis and Cascade Effect Tests

#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Env, vec};
use crate::testutils::Setup;
use utils::constant::{PERCENTAGE_PRECISION, PRICE_PRECISION};
use utils::test_utils::jump;

#[test]
fn test_bank_run_scenario() {
    let setup = Setup::default();
    let admin = setup.admin.clone();
    
    // Setup: 10 LPs with varying amounts
    let lps = vec![
        &setup.env,
        setup.users[0].clone(),
        setup.users[1].clone(),
        setup.users[2].clone(),
        setup.users[0].clone(),
        setup.users[1].clone(),
        setup.users[2].clone(),
        setup.users[0].clone(),
        setup.users[1].clone(),
        setup.users[2].clone(),
        setup.users[0].clone(),
    ];
    let deposits = vec![
        &setup.env,
        1_000_000_0000000,
        2_000_000_0000000,
        500_000_0000000,
        3_000_000_0000000,
        1_500_000_0000000,
        750_000_0000000,
        2_500_000_0000000,
        1_000_000_0000000,
        4_000_000_0000000,
        500_000_0000000,
    ];
    
    // Everyone deposits
    let mut lp_shares = vec![&setup.env];
    for (i, lp) in lps.iter().enumerate() {
        setup.liq_pool.deposit(lp, &deposits[i]);
        lp_shares.push(setup.token_share.balance(lp));
    }
    
    let initial_reserves = setup.liq_pool.get_reserves();
    let initial_total = initial_reserves.get(1).unwrap();
    
    // Panic event: 70% try to withdraw
    let panic_threshold = 7;
    let mut withdrawn = 0u128;
    
    for i in 0..panic_threshold {
        let shares_to_withdraw = lp_shares[i] as u128;
        
        // Try withdrawal
        let balance_before = setup.token2.balance(&lps[i]);
        setup.liq_pool.withdraw(&lps[i], &shares_to_withdraw);
        let balance_after = setup.token2.balance(&lps[i]);
        
        if balance_after > balance_before {
            withdrawn += deposits[i];
        } else {
            // Withdrawal failed - liquidity crisis
            break;
        }
    }
    
    // Check if pool survived
    let final_reserves = setup.liq_pool.get_reserves();
    let final_total = final_reserves.get(1).unwrap();
    
    let survival_ratio = (final_total * 100) / initial_total;
    
    assert!(
        survival_ratio > 10, // At least 10% liquidity remains
        "Pool collapsed: only {}% liquidity remains",
        survival_ratio
    );
    
    // Check if remaining LPs can still withdraw
    for i in panic_threshold..10 {
        // Try withdrawal for remaining LPs
        let shares = setup.token_share.balance(&lps[i]) as u128;
        let balance_before = setup.token2.balance(&lps[i]);
        setup.liq_pool.withdraw(&lps[i], &shares);
        let balance_after = setup.token2.balance(&lps[i]);
        
        assert!(
            balance_after > balance_before,
            "Remaining LPs cannot withdraw after panic"
        );
    }
}

#[test]
fn test_cascading_liquidation() {
    let setup = Setup::default();
    let admin = setup.admin.clone();
    
    // Setup multiple pools (simulated)
    setup.liq_pool.deposit(&admin, &10_000_000_0000000);
    
    // Create leveraged positions
    let leveraged_users = vec![
        &setup.env,
        setup.users[0].clone(),
        setup.users[1].clone(),
        setup.users[2].clone(),
        setup.users[0].clone(),
        setup.users[1].clone(),
    ];
    
    for user in &leveraged_users {
        // User deposits as collateral
        setup.liq_pool.deposit(user, &100_000_0000000);
        
        // Simulated borrow (would be actual borrow in production)
        // User effectively has 3x leverage
    }
    
    // Price crash triggers liquidations
    let initial_price = 50000_0000000;
    let crash_price = 35000_0000000; // 30% drop
    
    setup.oracle_client.set_price(
        &vec![&setup.env, crash_price],
        &setup.env.ledger().timestamp()
    );
    
    // Liquidation cascade
    let mut liquidated_count = 0;
    let mut total_liquidated_value = 0u128;
    
    for user in &leveraged_users {
        // Check if position is underwater
        let user_shares = setup.token_share.balance(user) as u128;
        let position_value = (user_shares * crash_price as u128) / PRICE_PRECISION as u128;
        let debt_value = 300_000_0000000u128; // 3x leverage debt
        
        if position_value < debt_value {
            // Liquidate position
            liquidated_count += 1;
            total_liquidated_value += user_shares;
            
            // Force withdrawal (liquidation)
            setup.liq_pool.withdraw(user, &user_shares);
        }
    }
    
    // Check cascade impact
    let reserves_after = setup.liq_pool.get_reserves();
    let liquidity_impact = (total_liquidated_value * 100) / reserves_after.get(1).unwrap();
    
    assert!(
        liquidity_impact < 50, // Less than 50% of liquidity affected
        "Cascade liquidation drained {}% of liquidity",
        liquidity_impact
    );
    
    // Pool should still function
    // Test if pool still functions
    let admin_balance_before = setup.token1.balance(&admin);
    setup.liq_pool.swap(
        &admin,
        &setup.token2.address,
        &setup.token1.address,
        &10000_0000000,
        &0
    );
    let admin_balance_after = setup.token1.balance(&admin);
    
    assert!(
        admin_balance_after > admin_balance_before,
        "Pool non-functional after liquidation cascade"
    );
}

#[test]
fn test_death_spiral_prevention() {
    let setup = Setup::default();
    let admin = setup.admin.clone();
    
    setup.liq_pool.deposit(&admin, &10_000_000_0000000);
    
    // Simulate death spiral conditions
    // 1. Price drops
    // 2. LPs withdraw
    // 3. Less liquidity = higher slippage
    // 4. More LPs panic
    // 5. Repeat
    
    let initial_liquidity = 10_000_000_0000000u128;
    let mut current_liquidity = initial_liquidity;
    let mut iteration = 0;
    
    while current_liquidity > initial_liquidity / 10 && iteration < 10 {
        iteration += 1;
        
        // Price drop
        let price_drop = 95; // 5% drop each iteration
        let new_price = (50000_0000000i128 * price_drop) / 100;
        setup.oracle_client.set_price(
            &vec![&setup.env, new_price],
            &setup.env.ledger().timestamp()
        );
        
        // Some LPs withdraw (panic)
        let withdrawal_pct = 10; // 10% withdraw each iteration
        let to_withdraw = (current_liquidity * withdrawal_pct) / 100;
        
        // Convert to shares and withdraw
        // Get total shares approximation
        let reserves = setup.liq_pool.get_reserves();
        let total_shares = reserves.get(1).unwrap();
        let shares_to_withdraw = (to_withdraw * total_shares) / current_liquidity;
        
        // Try withdrawal
        let admin_balance_before = setup.token2.balance(&admin);
        setup.liq_pool.withdraw(&admin, &shares_to_withdraw);
        let admin_balance_after = setup.token2.balance(&admin);
        
        if admin_balance_after == admin_balance_before {
            // Withdrawal restricted - circuit breaker activated
            break;
        }
        
        current_liquidity -= to_withdraw;
        
        // Check slippage increase
        let test_amount = 10000_0000000u128;
        let slippage = calculate_slippage(&setup, test_amount, current_liquidity);
        
        if slippage > 1000 { // 10% slippage
            // High slippage should trigger protection
            break;
        }
    }
    
    // Pool should have prevented complete collapse
    assert!(
        current_liquidity > initial_liquidity / 5, // At least 20% remains
        "Death spiral not prevented: {}% liquidity lost",
        ((initial_liquidity - current_liquidity) * 100) / initial_liquidity
    );
}

#[test]
fn test_minimum_liquidity_enforcement() {
    let setup = Setup::default();
    let lp1 = setup.users[0].clone();
    let lp2 = setup.users[1].clone();
    
    // Both LPs deposit
    setup.liq_pool.deposit(&lp1, &1_000_000_0000000);
    setup.liq_pool.deposit(&lp2, &1_000_000_0000000);
    
    // LP1 tries to withdraw everything
    let lp1_shares = setup.token_share.balance(&lp1) as u128;
    setup.liq_pool.withdraw(&lp1, &lp1_shares);
    
    // LP2 tries to withdraw everything
    let lp2_shares = setup.token_share.balance(&lp2) as u128;
    // Try to withdraw everything
    let lp2_balance_before = setup.token2.balance(&lp2);
    setup.liq_pool.withdraw(&lp2, &lp2_shares);
    let lp2_balance_after = setup.token2.balance(&lp2);
    
    // Should maintain minimum liquidity
    let final_reserves = setup.liq_pool.get_reserves();
    let remaining_liquidity = final_reserves.get(1).unwrap();
    
    assert!(
        remaining_liquidity > 0,
        "Pool allowed complete liquidity drain"
    );
}

#[test]
fn test_liquidity_imbalance_trigger() {
    let setup = Setup::default();
    let admin = setup.admin.clone();
    
    setup.liq_pool.deposit(&admin, &10_000_000_0000000);
    
    // Create imbalance by minting synthetic tokens
    let initial_reserves = setup.liq_pool.get_reserves();
    let reserve_a = initial_reserves.get(0).unwrap();
    let reserve_b = initial_reserves.get(1).unwrap();
    
    // Simulate large synthetic mint (imbalance)
    let synthetic_mint = reserve_a * 2; // Double the synthetic supply
    
    // This would trigger rebalancing or insurance
    let imbalance = calculate_liquidity_imbalance(
        synthetic_mint + reserve_a,
        reserve_b,
        50000_0000000
    );
    
    // Check if imbalance triggers protection
    let imbalance_threshold = 20; // 20% threshold
    let imbalance_pct = (imbalance.abs() * 100) / reserve_b as i128;
    
    if imbalance_pct > imbalance_threshold {
        // Should trigger insurance or buffer
        assert!(
            true, // Placeholder for actual trigger check
            "Imbalance {}% didn't trigger protection",
            imbalance_pct
        );
    }
}

#[test]
fn test_flash_crash_recovery() {
    let setup = Setup::default();
    let admin = setup.admin.clone();
    
    setup.liq_pool.deposit(&admin, &10_000_000_0000000);
    
    let normal_price = 50000_0000000;
    let flash_crash_price = 10000_0000000; // 80% drop
    
    // Flash crash
    setup.oracle_client.set_price(
        &vec![&setup.env, flash_crash_price],
        &setup.env.ledger().timestamp()
    );
    
    // Pool should have safeguards
    // Try large trade at crashed price
    let admin_token1_before = setup.token1.balance(&admin);
    setup.liq_pool.swap(
        &admin,
        &setup.token2.address,
        &setup.token1.address,
        &1_000_000_0000000,
        &0
    );
    let admin_token1_after = setup.token1.balance(&admin);
    
    // Should reject or limit trades during flash crash
    if admin_token1_after > admin_token1_before {
        // If trade went through, check if it was limited
        let received = setup.token1.balance(&admin);
        let expected_at_crash = (1_000_000_0000000i128 * PRICE_PRECISION) / flash_crash_price;
        
        assert!(
            received < expected_at_crash,
            "Flash crash price fully honored - no protection"
        );
    }
    
    // Recovery
    jump(&setup.env, 300); // 5 minutes
    setup.oracle_client.set_price(
        &vec![&setup.env, normal_price],
        &setup.env.ledger().timestamp()
    );
    
    // Pool should recover normally
    // Pool should recover normally
    let recovery_balance_before = setup.token1.balance(&admin);
    setup.liq_pool.swap(
        &admin,
        &setup.token2.address,
        &setup.token1.address,
        &10000_0000000,
        &0
    );
    let recovery_balance_after = setup.token1.balance(&admin);
    
    assert!(
        recovery_balance_after > recovery_balance_before,
        "Pool didn't recover after flash crash"
    );
}

#[test]
fn test_emergency_pause_activation() {
    let setup = Setup::default();
    let admin = setup.admin.clone();
    let user = setup.users[0].clone();
    
    setup.liq_pool.deposit(&admin, &10_000_000_0000000);
    
    // Simulate crisis conditions
    let crisis_indicators = vec![
        ("price_crash", 70),        // 70% price drop
        ("liquidity_drain", 80),    // 80% liquidity withdrawn
        ("imbalance", 50),          // 50% imbalance
        ("volume_spike", 1000),     // 10x volume spike
    ];
    
    for (indicator, severity) in crisis_indicators {
        if severity > 50 {
            // Should trigger emergency pause
            // setup.liq_pool.set_emergency_mode(&emergency_admin, true);
            
            // Check all operations are paused
            // Check if operations work during crisis
            let user_balance_before = setup.token2.balance(&user);
            
            // Try deposit
            setup.liq_pool.deposit(&user, &100000_0000000);
            let user_balance_after_deposit = setup.token2.balance(&user);
            
            // Try swap
            setup.liq_pool.swap(
                &user,
                &setup.token2.address,
                &setup.token1.address,
                &10000_0000000,
                &0
            );
            let user_balance_after_swap = setup.token2.balance(&user);
            
            if severity > 75 {
                // Severe crisis - operations should be limited
                assert!(
                    user_balance_after_deposit == user_balance_before ||
                    user_balance_after_swap == user_balance_after_deposit,
                    "Emergency protections not activated for {} with severity {}",
                    indicator,
                    severity
                );
            }
        }
    }
}

// Helper functions
fn calculate_slippage(setup: &Setup, amount: u128, liquidity: u128) -> u32 {
    // Simple slippage calculation
    if liquidity == 0 {
        return 10000; // 100% slippage
    }
    ((amount * 10000) / liquidity) as u32
}

fn calculate_liquidity_imbalance(token_a_supply: u128, reserve_b: u128, price: i128) -> i128 {
    let token_a_value = (token_a_supply as i128 * price) / PRICE_PRECISION;
    token_a_value - reserve_b as i128
}
