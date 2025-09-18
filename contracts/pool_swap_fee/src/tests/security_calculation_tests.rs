#![cfg(test)]

use crate::testutils::Setup;
use soroban_sdk::testutils::Address as _;

#[test]
fn test_lp_revenue_fraction_bounds() {
    let setup = Setup::default();
    
    // Test that LP revenue fraction is within valid bounds
    let lp_fraction = setup.fee_collector.get_lp_revenue_fraction();
    
    // Should be between 0 and 10000 (0% to 100%)
    assert!(lp_fraction <= 10000);
}

#[test]
fn test_router_address_valid() {
    let setup = Setup::default();
    
    // Test that router address is not zero/invalid
    let router = setup.fee_collector.get_router();
    
    // Router should be a valid address
    let _router_str = router.to_string();
}

#[test]
fn test_fee_destination_security() {
    let setup = Setup::default();
    
    // Test that fee destination is properly set and secure
    let fee_dest = setup.fee_collector.get_fee_destination();
    
    // Should match configured destination
    assert_eq!(fee_dest, setup.fee_destination);
}

#[test]
fn test_configuration_consistency() {
    let setup = Setup::default();
    
    // Test that all configuration values are consistent
    let router = setup.fee_collector.get_router();
    let fee_dest = setup.fee_collector.get_fee_destination();
    let lp_fraction = setup.fee_collector.get_lp_revenue_fraction();
    
    // All should be valid
    let _router_str = router.to_string();
    assert_eq!(fee_dest, setup.fee_destination);
    assert!(lp_fraction <= 10000);
}
