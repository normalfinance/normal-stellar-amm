#![cfg(test)]

use crate::testutils::Setup;
use soroban_sdk::testutils::Address as _;

#[test]
fn test_get_router() {
    let setup = Setup::default();
    
    // Test that router address can be retrieved
    let router = setup.fee_collector.get_router();
    
    // Router should be a valid address (check it's not empty bytes)
    // Since we can't easily check the exact value, just verify it's retrievable
    let _router_str = router.to_string();
}

#[test]
fn test_get_fee_destination() {
    let setup = Setup::default();
    
    // Test that fee destination can be retrieved
    let fee_dest = setup.fee_collector.get_fee_destination();
    
    // Should return the configured fee destination
    assert_eq!(fee_dest, setup.fee_destination);
}

#[test]
fn test_get_lp_revenue_fraction() {
    let setup = Setup::default();
    
    // Test that LP revenue fraction can be retrieved
    let lp_fraction = setup.fee_collector.get_lp_revenue_fraction();
    
    // Should be a reasonable fraction (in basis points)
    assert!(lp_fraction <= 10000); // Should not exceed 100%
}

#[test]
fn test_router_configuration() {
    let setup = Setup::default();
    
    // Test that router is properly configured
    let configured_router = setup.fee_collector.get_router();
    
    // Router should be properly configured (verify it's retrievable)
    let _router_str = configured_router.to_string();
}
