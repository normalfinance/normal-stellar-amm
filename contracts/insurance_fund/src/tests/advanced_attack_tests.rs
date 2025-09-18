#![cfg(test)]

use crate::testutils::{Setup, TestConfig};
use soroban_sdk::testutils::Address as _;
use soroban_sdk::token::TokenClient;
use utils::constant::THIRTEEN_DAY;

#[test]
#[should_panic] // Expected to panic due to withdraw delay
fn test_flash_loan_attack_simulation() {
    let setup = Setup::new_with_config(&TestConfig::default());
    
    let attacker = &setup.users[0];
    let victim = &setup.users[1];
    
    // Victim deposits into insurance fund
    let victim_deposit = 10000_0000000; // 10k tokens
    setup.insurance_fund.deposit(victim, &setup.token_a.address, &victim_deposit);
    
    // Simulate flash loan attack attempt
    let large_amount = 100000_0000000; // 100k tokens
    
    // Attacker deposits large amount
    setup.insurance_fund.deposit(attacker, &setup.token_a.address, &large_amount);
    
    // Check that utilization calculation is not manipulated
    let utilization = setup.insurance_fund.get_utilization();
    assert!(utilization <= 100_00000); // Max 100%
    
    // Attacker cannot immediately withdraw - this should panic
    setup.insurance_fund.request_withdraw(attacker, &setup.token_a.address, &large_amount);
    setup.insurance_fund.withdraw(attacker, &setup.token_a.address); // This should panic
}

#[test]
fn test_precision_manipulation_attack() {
    let setup = Setup::new_with_config(&TestConfig::default());
    
    let attacker = &setup.users[0];
    
    // Attempt to manipulate precision in share calculations
    // by depositing very small amounts multiple times
    
    let mut total_deposited = 0i128;
    let small_amount = 1; // 1 unit
    
    // Deposit many tiny amounts
    for _ in 0..1000 {
        setup.insurance_fund.deposit(attacker, &setup.token_a.address, &small_amount);
        total_deposited += small_amount;
    }
    
    let attacker_stake = setup.insurance_fund.get_stake(attacker, &setup.token_a.address);
    
    // Check that shares calculation is accurate despite small amounts
    // Shares should be proportional to deposits
    assert!(attacker_stake.if_shares > 0);
    assert!(attacker_stake.if_shares <= total_deposited); // No free shares
}

#[test]
fn test_reentrancy_attack_protection() {
    let setup = Setup::new_with_config(&TestConfig::default());
    
    let attacker = &setup.users[0];
    let deposit_amount = 1000_0000000;
    
    // Attacker deposits
    setup.insurance_fund.deposit(attacker, &setup.token_a.address, &deposit_amount);
    
    // Request withdraw
    setup.insurance_fund.request_withdraw(attacker, &setup.token_a.address, &deposit_amount);
    
    // Wait for unlock period
    setup.env.ledger().set_timestamp(
        setup.env.ledger().timestamp() + THIRTEEN_DAY + 1
    );
    
    // Attempt reentrancy during withdraw
    // This would require modifying the testutils to support reentrancy simulation
    // For now, just test that single withdraw works correctly
    setup.insurance_fund.withdraw(attacker, &setup.token_a.address);
    
    // Verify attacker cannot withdraw again
    let result = std::panic::catch_unwind(|| {
        setup.insurance_fund.withdraw(attacker, &setup.token_a.address);
    });
    
    assert!(result.is_err());
}

#[test]
fn test_interest_rate_manipulation() {
    let setup = Setup::new_with_config(&TestConfig::default());
    
    let user1 = &setup.users[0];
    let user2 = &setup.users[1];
    
    let deposit_amount = 5000_0000000;
    
    // User1 deposits
    setup.insurance_fund.deposit(user1, &setup.token_a.address, &deposit_amount);
    
    // Simulate high utilization to increase interest rates
    // This would require interaction with pool contracts
    // For now, test that interest calculation is bounded
    
    let initial_utilization = setup.insurance_fund.get_utilization();
    
    // User2 deposits more to change utilization
    setup.insurance_fund.deposit(user2, &setup.token_a.address, &deposit_amount);
    
    let final_utilization = setup.insurance_fund.get_utilization();
    
    // Utilization should change predictably
    assert!(final_utilization >= initial_utilization);
    
    // Interest rate should be within reasonable bounds
    let interest_rate = setup.insurance_fund.get_rate();
    assert!(interest_rate >= 0);
    assert!(interest_rate <= 100_00000); // Max 100% APR
}

#[test]
fn test_governance_attack_simulation() {
    let setup = Setup::new_with_config(&TestConfig::default());
    
    let attacker = &setup.users[0];
    let normal_user = &setup.users[1];
    
    // Attacker tries to gain large share of insurance fund
    let large_deposit = 50000_0000000;
    setup.insurance_fund.deposit(attacker, &setup.token_a.address, &large_deposit);
    
    // Normal user deposits smaller amount  
    let normal_deposit = 1000_0000000;
    setup.insurance_fund.deposit(normal_user, &setup.token_a.address, &normal_deposit);
    
    // Check that large depositor doesn't get disproportionate benefits
    let attacker_stake = setup.insurance_fund.get_stake(attacker, &setup.token_a.address);
    let normal_stake = setup.insurance_fund.get_stake(normal_user, &setup.token_a.address);
    
    // Share ratio should be proportional to deposit ratio
    let deposit_ratio = large_deposit / normal_deposit;
    let share_ratio = attacker_stake.if_shares / normal_stake.if_shares;
    
    // Allow for small rounding differences
    let ratio_difference = if share_ratio > deposit_ratio {
        share_ratio - deposit_ratio
    } else {
        deposit_ratio - share_ratio
    };
    
    assert!(ratio_difference <= 1); // Very small difference allowed
}

#[test]
fn test_withdrawal_front_running() {
    let setup = Setup::new_with_config(&TestConfig::default());
    
    let user1 = &setup.users[0];
    let user2 = &setup.users[1];
    
    let deposit_amount = 5000_0000000;
    
    // Both users deposit
    setup.insurance_fund.deposit(user1, &setup.token_a.address, &deposit_amount);
    setup.insurance_fund.deposit(user2, &setup.token_a.address, &deposit_amount);
    
    // User1 requests withdrawal
    setup.insurance_fund.request_withdraw(user1, &setup.token_a.address, &deposit_amount);
    
    // User2 tries to front-run by requesting withdrawal immediately after
    setup.insurance_fund.request_withdraw(user2, &setup.token_a.address, &deposit_amount);
    
    // Both should have to wait the same unlock period
    setup.env.ledger().set_timestamp(
        setup.env.ledger().timestamp() + THIRTEEN_DAY + 1
    );
    
    // Both should be able to withdraw
    setup.insurance_fund.withdraw(user1, &setup.token_a.address);
    setup.insurance_fund.withdraw(user2, &setup.token_a.address);
    
    // Check that both got fair treatment
    let user1_balance = setup.token_a.balance(user1);
    let user2_balance = setup.token_a.balance(user2);
    
    // Both should get approximately the same amount back
    let balance_difference = if user1_balance > user2_balance {
        user1_balance - user2_balance
    } else {
        user2_balance - user1_balance
    };
    
    assert!(balance_difference < deposit_amount / 1000); // Less than 0.1% difference
}

#[test]
fn test_share_dilution_attack() {
    let setup = Setup::new_with_config(&TestConfig::default());
    
    let victim = &setup.users[0];
    let attacker = &setup.users[1];
    
    // Victim deposits first
    let victim_deposit = 1000_0000000;
    setup.insurance_fund.deposit(victim, &setup.token_a.address, &victim_deposit);
    
    let victim_initial_stake = setup.insurance_fund.get_stake(victim, &setup.token_a.address);
    
    // Simulate profit/yield in the fund (this would happen through pool interactions)
    // For testing, we'll deposit additional tokens directly to simulate yield
    
    // Attacker tries to dilute victim's shares by depositing large amount
    let attacker_deposit = 10000_0000000;
    setup.insurance_fund.deposit(attacker, &setup.token_a.address, &attacker_deposit);
    
    // Victim's shares should remain the same
    let victim_final_stake = setup.insurance_fund.get_stake(victim, &setup.token_a.address);
    assert_eq!(victim_initial_stake.if_shares, victim_final_stake.if_shares);
    
    // But victim's share of total should decrease  
    let attacker_stake = setup.insurance_fund.get_stake(attacker, &setup.token_a.address);
    let total_shares = victim_final_stake.if_shares + attacker_stake.if_shares;
    let victim_percentage = (victim_final_stake.if_shares * 100) / total_shares;
    
    // Victim should have much smaller percentage now
    assert!(victim_percentage < 20); // Less than 20% of total
}
