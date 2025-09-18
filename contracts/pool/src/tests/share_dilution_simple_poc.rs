#![cfg(test)]

// Simple proof of concept for share dilution attacks
// This demonstrates the basic mathematical vulnerability

#[test]
fn simple_share_dilution_math() {
    // Initial state: empty pool
    let mut total_shares = 0u128;
    let mut total_assets = 0u128;
    
    // First depositor adds assets
    let first_deposit = 1000u128;
    total_assets += first_deposit;
    total_shares += first_deposit; // 1:1 minting
    
    assert_eq!(total_shares, first_deposit);
    assert_eq!(total_assets, first_deposit);
    
    // Pool generates yield/profit (simulated)
    let yield_amount = 100u128;
    total_assets += yield_amount;
    
    // Now NAV per share = 1100 / 1000 = 1.1
    let nav_per_share = total_assets / total_shares;
    assert_eq!(nav_per_share, 1); // Due to integer division: 1100/1000 = 1
    
    // Second depositor adds same amount but gets shares at current NAV
    let second_deposit = 1000u128;
    total_assets += second_deposit;
    
    // Shares minted should be deposit / nav_per_share
    // But if using 1:1 minting, second depositor gets underpriced shares
    let shares_minted_correct = second_deposit / nav_per_share; // Should be ~909 shares
    let shares_minted_vulnerable = second_deposit; // 1:1 minting = 1000 shares
    
    total_shares += shares_minted_vulnerable;
    
    // Now second depositor has more shares than they should
    assert!(shares_minted_vulnerable > shares_minted_correct);
    
    // When they withdraw, they get more than they put in
    let withdrawal_share = shares_minted_vulnerable;
    let withdrawal_amount = (withdrawal_share * total_assets) / total_shares;
    
    assert!(withdrawal_amount > second_deposit);
}

#[test] 
fn share_dilution_with_multiple_deposits() {
    let mut total_shares = 0u128;
    let mut total_assets = 0u128;
    
    // First depositor
    let first_deposit = 1000u128;
    total_assets += first_deposit;
    total_shares += first_deposit;
    
    // Simulate yield
    total_assets += 500u128; // 50% yield
    
    // Multiple small deposits that exploit 1:1 minting
    for _ in 0..10 {
        let small_deposit = 100u128;
        total_assets += small_deposit;
        total_shares += small_deposit; // 1:1 minting vulnerability
    }
    
    // Check that total shares exceed what they should be
    let correct_total_assets = first_deposit + 500u128 + (10 * 100u128);
    assert_eq!(total_assets, correct_total_assets);
    
    // But shares are inflated due to 1:1 minting
    let inflated_shares = first_deposit + (10 * 100u128);
    assert_eq!(total_shares, inflated_shares);
    
    // NAV per share is artificially low
    let nav_per_share = total_assets / total_shares;
    let expected_nav_per_share = correct_total_assets / first_deposit; // If only first depositor had shares
    
    assert!(nav_per_share < expected_nav_per_share);
}

#[test]
fn demonstrate_dilution_impact() {
    // Show how early depositors get diluted
    
    let mut total_shares = 0u128;
    let mut total_assets = 0u128;
    
    // Alice deposits first
    let alice_deposit = 1000u128;
    total_assets += alice_deposit;
    total_shares += alice_deposit;
    let alice_shares = alice_deposit;
    
    // Pool generates significant yield
    total_assets += 1000u128; // 100% yield
    
    // Alice's value before dilution
    let alice_value_before = (alice_shares * total_assets) / total_shares;
    assert_eq!(alice_value_before, 2000u128); // Should have 2000 due to 100% yield
    
    // Bob exploits 1:1 minting
    let bob_deposit = 1000u128;
    total_assets += bob_deposit;
    total_shares += bob_deposit; // 1:1 minting
    
    // Alice's value after Bob's deposit
    let alice_value_after = (alice_shares * total_assets) / total_shares;
    
    // Alice gets diluted
    assert!(alice_value_after < alice_value_before);
    
    // Bob's shares are underpriced
    let bob_shares = bob_deposit;
    let bob_value = (bob_shares * total_assets) / total_shares;
    assert!(bob_value > bob_deposit);
}
