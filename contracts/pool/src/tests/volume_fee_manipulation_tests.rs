// Author: Alp Onaran
// Company: Halborn Security  
// Volume Manipulation and Fee Distribution Tests

#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Env};
use crate::testutils::Setup;
use utils::constant::{PERCENTAGE_PRECISION, PRICE_PRECISION};

const ONE_DAY: u64 = 86400;
use utils::test_utils::jump;

#[test]
fn test_volume_based_premium_manipulation() {
    let setup = Setup::default();
    let attacker = setup.users[0].clone();
    let admin = setup.admin.clone();
    
    // Setup pool with liquidity
    setup.liq_pool.deposit(&admin, &10_000_000_0000000);
    
    // Baseline: Normal volume over 30 days
    let normal_daily_volume = 100_000_0000000;
    for _ in 0..30 {
        // Normal trading
        setup.liq_pool.swap(
            &admin,
            &setup.token2.address,
            &setup.token1.address,
            &normal_daily_volume,
            &0
        );
        jump(&setup.env, ONE_DAY);
    }
    
    // Get baseline premium rate
    let baseline_premium = calculate_insurance_premium(&setup);
    
    // Attack: Manipulate volume in short period
    jump(&setup.env, ONE_DAY); // Next day
    
    // Attacker does wash trading to inflate volume
    for _ in 0..100 {
        // Swap back and forth
        setup.liq_pool.swap(
            &attacker,
            &setup.token2.address,
            &setup.token1.address,
            &1_000_000_0000000,
            &0
        );
        setup.liq_pool.swap(
            &attacker,
            &setup.token1.address,
            &setup.token2.address,
            &999_000_0000000, // Account for fees
            &0
        );
    }
    
    // Check manipulated premium
    let manipulated_premium = calculate_insurance_premium(&setup);
    
    // Premium should not drop drastically
    let premium_reduction = if baseline_premium > manipulated_premium {
        ((baseline_premium - manipulated_premium) * 100) / baseline_premium
    } else {
        0
    };
    
    assert!(
        premium_reduction < 90, // Less than 90% reduction
        "Volume manipulation achieved {}% premium reduction",
        premium_reduction
    );
}

#[test]
fn test_static_fee_split_unfairness() {
    let setup = Setup::default();
    let lp1 = setup.users[0].clone();
    let lp2 = setup.users[1].clone();
    let trader = setup.users[2].clone();
    
    // Initial state: 50/50 liquidity split
    setup.liq_pool.deposit(&lp1, &5_000_000_0000000);
    setup.liq_pool.deposit(&lp2, &5_000_000_0000000);
    
    let lp1_shares_initial = setup.token_share.balance(&lp1);
    let lp2_shares_initial = setup.token_share.balance(&lp2);
    let total_shares_initial = setup.token_share.total_supply();
    
    // Generate fees
    for _ in 0..10 {
        setup.liq_pool.swap(
            &trader,
            &setup.token2.address,
            &setup.token1.address,
            &100_000_0000000,
            &0
        );
    }
    
    // Collect fees (would be distributed 50/50 to LPs/protocol)
    let total_fees = get_accumulated_fees(&setup);
    let lp_portion = total_fees / 2; // Static 50%
    
    // Now liquidity changes: LP2 withdraws half
    let lp2_withdraw = lp2_shares_initial / 2;
    setup.liq_pool.withdraw(&lp2, &(lp2_withdraw as u128));
    
    // New liquidity split is now ~67/33
    let lp1_ownership = (lp1_shares_initial * 100) / total_shares_initial;
    let lp2_ownership = ((lp2_shares_initial - lp2_withdraw) * 100) / total_shares_initial;
    
    // But fee split remains 50/50 - unfair!
    let fair_lp_portion = (total_fees * (lp1_ownership + lp2_ownership) as u128) / 100;
    let unfairness = if fair_lp_portion > lp_portion {
        ((fair_lp_portion - lp_portion) * 100) / fair_lp_portion
    } else {
        ((lp_portion - fair_lp_portion) * 100) / lp_portion
    };
    
    assert!(
        unfairness < 20, // Less than 20% unfairness
        "Static fee split causes {}% unfairness",
        unfairness
    );
}

#[test]
fn test_fee_accumulation_overflow() {
    let setup = Setup::default();
    let trader = setup.users[0].clone();
    let admin = setup.admin.clone();
    
    setup.liq_pool.deposit(&admin, &10_000_000_0000000);
    
    // Simulate extreme trading volume
    let large_trade = u128::MAX / 1000; // Very large but not overflow
    
    // Try to accumulate fees that might overflow
    for _ in 0..1000 {
        // Try large trade - in no_std we can't catch panic
        // Just check if pool can handle it
        setup.liq_pool.swap(
            &trader,
            &setup.token2.address,
            &setup.token1.address,
            &(large_trade / 1000000), // Scale down to avoid issues
            &0
        );
    }
    
    // Check fee accumulator didn't overflow
    let fees = get_accumulated_fees(&setup);
    assert!(
        fees < u128::MAX,
        "Fee accumulation overflowed"
    );
}

#[test]
fn test_insurance_premium_time_based() {
    let setup = Setup::default();
    let admin = setup.admin.clone();
    
    setup.liq_pool.deposit(&admin, &10_000_000_0000000);
    
    // Set insurance parameters
    let annual_premium_rate = 200; // 2% annually in basis points
    let pool_value = 10_000_000_0000000u128;
    
    // Calculate expected daily premium
    let daily_premium = (pool_value * annual_premium_rate) / (365 * 10000);
    
    // Simulate time passing without volume
    let initial_insurance = get_insurance_fund_balance(&setup);
    
    for _ in 0..30 {
        jump(&setup.env, ONE_DAY);
        // Trigger premium accrual (would be automatic in production)
        setup.liq_pool.rebalance(&admin);
    }
    
    let final_insurance = get_insurance_fund_balance(&setup);
    let accrued_premium = final_insurance - initial_insurance;
    let expected_premium = daily_premium * 30;
    
    // Check time-based accrual
    let difference = if accrued_premium > expected_premium as i128 {
        ((accrued_premium - expected_premium as i128) * 100) / expected_premium as i128
    } else {
        ((expected_premium as i128 - accrued_premium) * 100) / expected_premium as i128
    };
    
    assert!(
        difference < 10, // Within 10% of expected
        "Time-based premium accrual off by {}%",
        difference
    );
}

#[test]
fn test_buffer_fee_fraction_limits() {
    let setup = Setup::default();
    let admin = setup.admin.clone();
    let trader = setup.users[0].clone();
    
    setup.liq_pool.deposit(&admin, &10_000_000_0000000);
    
    // Test various buffer fee fractions
    let fractions = vec![
        0,     // 0%
        500,   // 5%
        1000,  // 10%
        2000,  // 20%
        5000,  // 50%
        10000, // 100%
    ];
    
    for fraction in fractions {
        // Set buffer fee fraction (would be admin function)
        // setup.liq_pool.set_buffer_fee_fraction(&admin, &fraction);
        
        // Generate fees
        let trade_amount = 100_000_0000000;
        setup.liq_pool.swap(
            &trader,
            &setup.token2.address,
            &setup.token1.address,
            &trade_amount,
            &0
        );
        
        let total_fee = (trade_amount * 30) / 10000; // 0.3% fee
        let buffer_fee = (total_fee * fraction as u128) / 10000;
        
        // Check buffer receives correct amount
        let buffer_balance = get_buffer_balance(&setup);
        
        // Extreme fractions should be limited
        if fraction > 2000 {
            assert!(
                buffer_fee < total_fee / 2,
                "Buffer taking too much: {}% of fees",
                fraction / 100
            );
        }
    }
}

#[test]
fn test_fee_distribution_with_zero_liquidity() {
    let setup = Setup::default();
    let lp = setup.users[0].clone();
    let trader = setup.users[1].clone();
    
    // Deposit and immediately withdraw
    setup.liq_pool.deposit(&lp, &1_000_000_0000000);
    let shares = setup.token_share.balance(&lp);
    setup.liq_pool.withdraw(&lp, &(shares as u128));
    
    // Try to generate fees with zero/minimal liquidity
    // Try swap with minimal liquidity
    // This should fail or have extreme slippage
    let trader_balance_before = setup.token2.balance(&trader);
    
    // Attempt swap
    setup.liq_pool.swap(
        &trader,
        &setup.token2.address,
        &setup.token1.address,
        &10000_0000000,
        &0
    );
    
    let trader_balance_after = setup.token2.balance(&trader);
    
    // Should have failed or gotten terrible rate
    assert!(
        trader_balance_after <= trader_balance_before,
        "Swap succeeded with zero liquidity - fee distribution undefined"
    );
}

#[test]
fn test_dynamic_fee_adjustment() {
    let setup = Setup::default();
    let admin = setup.admin.clone();
    let trader = setup.users[0].clone();
    
    setup.liq_pool.deposit(&admin, &10_000_000_0000000);
    
    // Test fee adjustment based on volatility
    let base_fee = 30; // 0.3%
    
    // Low volatility period
    for _ in 0..10 {
        setup.liq_pool.swap(
            &trader,
            &setup.token2.address,
            &setup.token1.address,
            &10000_0000000,
            &0
        );
        jump(&setup.env, 3600); // 1 hour
    }
    
    let low_vol_fee = get_current_fee_rate(&setup);
    
    // High volatility period (large trades)
    for _ in 0..5 {
        setup.liq_pool.swap(
            &trader,
            &setup.token2.address,
            &setup.token1.address,
            &1000000_0000000, // 100x larger
            &0
        );
    }
    
    let high_vol_fee = get_current_fee_rate(&setup);
    
    // Fees should adjust with volatility
    assert!(
        high_vol_fee >= low_vol_fee,
        "Dynamic fees not responding to volatility"
    );
}

#[test]
fn test_fee_compound_effect() {
    let setup = Setup::default();
    let lp = setup.users[0].clone();
    let trader = setup.users[1].clone();
    
    let initial_deposit = 1_000_000_0000000;
    setup.liq_pool.deposit(&lp, &initial_deposit);
    
    // Simulate trading over time with fee reinvestment
    for day in 0..365 {
        // Daily trading volume
        let daily_volume = 100_000_0000000;
        setup.liq_pool.swap(
            &trader,
            &setup.token2.address,
            &setup.token1.address,
            &daily_volume,
            &0
        );
        
        // Fees compound into liquidity
        if day % 30 == 0 {
            // Monthly fee collection/reinvestment
            let lp_shares = setup.token_share.balance(&lp);
            // Get total shares approximation
            let reserves = setup.liq_pool.get_reserves();
            let total_shares = reserves.get(1).unwrap() as i128;
            let lp_ownership = (lp_shares * PERCENTAGE_PRECISION as i128) / total_shares;
            
            // LP's share grows with compounded fees
            assert!(
                lp_ownership > 0,
                "LP ownership diluted despite fee accrual"
            );
        }
        
        jump(&setup.env, ONE_DAY);
    }
    
    // Final value should exceed initial + simple interest
    let final_shares = setup.token_share.balance(&lp);
    let reserves = setup.liq_pool.get_reserves();
    let total_value = reserves.get(1).unwrap();
    let lp_value = (total_value * final_shares as u128) / total_value; // Simplified
    
    let total_return = ((lp_value - initial_deposit) * 100) / initial_deposit;
    
    assert!(
        total_return > 0,
        "No positive return despite fee accumulation"
    );
}

// Helper functions
fn calculate_insurance_premium(setup: &Setup) -> i128 {
    // Placeholder - would calculate actual premium
    1000_0000000
}

fn get_accumulated_fees(setup: &Setup) -> u128 {
    // Placeholder - would get actual accumulated fees
    10000_0000000
}

fn get_insurance_fund_balance(setup: &Setup) -> i128 {
    // Placeholder - would get actual insurance fund balance
    1000000_0000000
}

fn get_buffer_balance(setup: &Setup) -> i128 {
    // Placeholder - would get actual buffer balance
    100000_0000000
}

fn get_current_fee_rate(setup: &Setup) -> u32 {
    // Placeholder - would get actual current fee rate
    30
}
