#![cfg(test)]

// use crate::testutils::{ install_dummy_wasm, jump, Setup };
use access_control::constants::ADMIN_ACTIONS_DELAY;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{ symbol_short, Address, Symbol };
use utils::test_utils::{ install_dummy_wasm, jump };

use crate::testutils::Setup;

// test admin transfer ownership
#[test]
#[should_panic(expected = "Error(Contract, #2908)")]
fn test_admin_transfer_ownership_too_early() {
    let setup = Setup::default();
    let buffer = setup.buffer;
    let admin_original = setup.admin;
    let admin_new = Address::generate(&setup.env);

    buffer.commit_transfer_ownership(&admin_original, &symbol_short!("Admin"), &admin_new);
    // check admin not changed yet by calling protected method
    assert!(buffer.try_revert_transfer_ownership(&admin_new, &symbol_short!("Admin")).is_err());
    jump(&setup.env, ADMIN_ACTIONS_DELAY - 1);
    buffer.apply_transfer_ownership(&admin_original, &symbol_short!("Admin"));
}

#[test]
#[should_panic(expected = "Error(Contract, #2906)")]
fn test_admin_transfer_ownership_twice() {
    let setup = Setup::default();
    let buffer = setup.buffer;
    let admin_original = setup.admin;
    let admin_new = Address::generate(&setup.env);

    buffer.commit_transfer_ownership(&admin_original, &symbol_short!("Admin"), &admin_new);
    buffer.commit_transfer_ownership(&admin_original, &symbol_short!("Admin"), &admin_new);
}

#[test]
#[should_panic(expected = "Error(Contract, #2907)")]
fn test_admin_transfer_ownership_not_committed() {
    let setup = Setup::default();
    let buffer = setup.buffer;
    let admin_original = setup.admin;

    jump(&setup.env, ADMIN_ACTIONS_DELAY + 1);
    buffer.apply_transfer_ownership(&admin_original, &symbol_short!("Admin"));
}

#[test]
#[should_panic(expected = "Error(Contract, #2907)")]
fn test_admin_transfer_ownership_reverted() {
    let setup = Setup::default();
    let buffer = setup.buffer;
    let admin_original = setup.admin;
    let admin_new = Address::generate(&setup.env);

    buffer.commit_transfer_ownership(&admin_original, &symbol_short!("Admin"), &admin_new);
    // check admin not changed yet by calling protected method
    assert!(buffer.try_revert_transfer_ownership(&admin_new, &symbol_short!("Admin")).is_err());
    jump(&setup.env, ADMIN_ACTIONS_DELAY + 1);
    buffer.revert_transfer_ownership(&admin_original, &symbol_short!("Admin"));
    buffer.apply_transfer_ownership(&admin_original, &symbol_short!("Admin"));
}

#[test]
fn test_admin_transfer_ownership() {
    let setup = Setup::default();
    let buffer = setup.buffer;
    let admin_original = setup.admin;
    let admin_new = Address::generate(&setup.env);

    buffer.commit_transfer_ownership(&admin_original, &symbol_short!("Admin"), &admin_new);
    // check admin not changed yet by calling protected method
    assert!(buffer.try_revert_transfer_ownership(&admin_new, &symbol_short!("Admin")).is_err());
    jump(&setup.env, ADMIN_ACTIONS_DELAY + 1);
    buffer.apply_transfer_ownership(&admin_original, &symbol_short!("Admin"));

    buffer.commit_transfer_ownership(&admin_new, &symbol_short!("Admin"), &admin_new);
}

// test emergency admin transfer ownership
#[test]
#[should_panic(expected = "Error(Contract, #2908)")]
fn test_emergency_admin_transfer_ownership_too_early() {
    let setup = Setup::default();
    let buffer = setup.buffer;
    let emergency_admin_new = Address::generate(&setup.env);

    buffer.commit_transfer_ownership(
        &setup.admin,
        &Symbol::new(&setup.env, "EmergencyAdmin"),
        &emergency_admin_new
    );

    // check emergency admin not changed yet by calling protected method
    assert!(buffer.try_set_emergency_mode(&emergency_admin_new, &false).is_err());
    assert!(buffer.try_set_emergency_mode(&setup.emergency_admin, &false).is_ok());

    jump(&setup.env, ADMIN_ACTIONS_DELAY - 1);
    buffer.apply_transfer_ownership(&setup.admin, &Symbol::new(&setup.env, "EmergencyAdmin"));
}

#[test]
#[should_panic(expected = "Error(Contract, #2906)")]
fn test_emergency_admin_transfer_ownership_twice() {
    let setup = Setup::default();
    let buffer = setup.buffer;
    let emergency_admin_new = Address::generate(&setup.env);

    buffer.commit_transfer_ownership(
        &setup.admin,
        &Symbol::new(&setup.env, "EmergencyAdmin"),
        &emergency_admin_new
    );
    buffer.commit_transfer_ownership(
        &setup.admin,
        &Symbol::new(&setup.env, "EmergencyAdmin"),
        &emergency_admin_new
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #2907)")]
fn test_emergency_admin_transfer_ownership_not_committed() {
    let setup = Setup::default();
    let buffer = setup.buffer;

    jump(&setup.env, ADMIN_ACTIONS_DELAY + 1);
    buffer.apply_transfer_ownership(&setup.admin, &Symbol::new(&setup.env, "EmergencyAdmin"));
}

#[test]
#[should_panic(expected = "Error(Contract, #2907)")]
fn test_emergency_admin_transfer_ownership_reverted() {
    let setup = Setup::default();
    let buffer = setup.buffer;
    let emergency_admin_new = Address::generate(&setup.env);

    buffer.commit_transfer_ownership(
        &setup.admin,
        &Symbol::new(&setup.env, "EmergencyAdmin"),
        &emergency_admin_new
    );

    // check emergency admin not changed yet by calling protected method
    assert!(buffer.try_set_emergency_mode(&emergency_admin_new, &false).is_err());
    assert!(buffer.try_set_emergency_mode(&setup.emergency_admin, &false).is_ok());

    jump(&setup.env, ADMIN_ACTIONS_DELAY + 1);
    buffer.revert_transfer_ownership(&setup.admin, &Symbol::new(&setup.env, "EmergencyAdmin"));
    buffer.apply_transfer_ownership(&setup.admin, &Symbol::new(&setup.env, "EmergencyAdmin"));
}

#[test]
fn test_emergency_admin_transfer_ownership() {
    let setup = Setup::default();
    let buffer = setup.buffer;
    let emergency_admin_new = Address::generate(&setup.env);

    buffer.commit_transfer_ownership(
        &setup.admin,
        &Symbol::new(&setup.env, "EmergencyAdmin"),
        &emergency_admin_new
    );

    // check emergency admin not changed yet by calling protected method
    assert!(buffer.try_set_emergency_mode(&emergency_admin_new, &false).is_err());
    assert!(buffer.try_set_emergency_mode(&setup.emergency_admin, &false).is_ok());

    jump(&setup.env, ADMIN_ACTIONS_DELAY + 1);
    buffer.apply_transfer_ownership(&setup.admin, &Symbol::new(&setup.env, "EmergencyAdmin"));

    // check emergency admin has changed
    assert!(buffer.try_set_emergency_mode(&emergency_admin_new, &false).is_ok());
    assert!(buffer.try_set_emergency_mode(&setup.emergency_admin, &false).is_err());
}

#[test]
fn test_transfer_ownership_separate_deadlines() {
    let setup = Setup::default();
    let buffer = setup.buffer;
    let admin_new = Address::generate(&setup.env);
    let emergency_admin_new = Address::generate(&setup.env);

    assert_eq!(
        buffer.get_future_address(&Symbol::new(&setup.env, "EmergencyAdmin")),
        setup.emergency_admin
    );
    assert_eq!(buffer.get_future_address(&symbol_short!("Admin")), setup.admin);

    assert!(buffer.try_set_emergency_mode(&emergency_admin_new, &false).is_err());
    assert!(buffer.try_set_emergency_mode(&setup.emergency_admin, &false).is_ok());

    buffer.commit_transfer_ownership(
        &setup.admin,
        &Symbol::new(&setup.env, "EmergencyAdmin"),
        &emergency_admin_new
    );
    jump(&setup.env, 10);
    buffer.commit_transfer_ownership(&setup.admin, &symbol_short!("Admin"), &admin_new);

    assert_eq!(
        buffer.get_future_address(&Symbol::new(&setup.env, "EmergencyAdmin")),
        emergency_admin_new
    );
    assert_eq!(buffer.get_future_address(&symbol_short!("Admin")), admin_new);

    jump(&setup.env, ADMIN_ACTIONS_DELAY + 1 - 10);
    buffer.apply_transfer_ownership(&setup.admin, &Symbol::new(&setup.env, "EmergencyAdmin"));
    assert!(buffer.try_apply_transfer_ownership(&setup.admin, &symbol_short!("Admin")).is_err());

    assert_eq!(
        buffer.get_future_address(&Symbol::new(&setup.env, "EmergencyAdmin")),
        emergency_admin_new
    );

    jump(&setup.env, 10);
    buffer.apply_transfer_ownership(&setup.admin, &symbol_short!("Admin"));

    assert_eq!(buffer.get_future_address(&symbol_short!("Admin")), admin_new);

    // check ownership transfer is complete. new admin is capable to call protected methods
    //      and new emergency admin can change toggle emergency mode
    buffer.commit_transfer_ownership(&admin_new, &Symbol::new(&setup.env, "Admin"), &setup.admin);
    assert!(buffer.try_set_emergency_mode(&emergency_admin_new, &false).is_ok());
    assert!(buffer.try_set_emergency_mode(&setup.emergency_admin, &false).is_err());
}

// upgrade
#[test]
fn test_commit_upgrade_third_party_user() {
    let setup = Setup::default();
    let buffer = setup.buffer;
    let user = Address::generate(&setup.env);
    assert!(buffer.try_commit_upgrade(&user, &install_dummy_wasm(&setup.env)).is_err());
}

#[test]
fn test_commit_upgrade_emergency_admin() {
    let setup = Setup::default();
    let buffer = setup.buffer;
    assert!(
        buffer.try_commit_upgrade(&setup.emergency_admin, &install_dummy_wasm(&setup.env)).is_err()
    );
}

#[test]
fn test_commit_upgrade_admin() {
    let setup = Setup::default();
    let buffer = setup.buffer;
    assert!(buffer.try_commit_upgrade(&setup.admin, &install_dummy_wasm(&setup.env)).is_ok());
}

#[test]
fn test_apply_upgrade_third_party_user() {
    let setup = Setup::default();
    let buffer = setup.buffer;
    let user = Address::generate(&setup.env);
    buffer.commit_upgrade(&setup.admin, &install_dummy_wasm(&setup.env));
    jump(&setup.env, ADMIN_ACTIONS_DELAY + 1);
    assert!(buffer.try_apply_upgrade(&user).is_err());
}

#[test]
fn test_apply_upgrade_emergency_admin() {
    let setup = Setup::default();
    let buffer = setup.buffer;
    buffer.commit_upgrade(&setup.admin, &install_dummy_wasm(&setup.env));
    jump(&setup.env, ADMIN_ACTIONS_DELAY + 1);
    assert!(buffer.try_apply_upgrade(&setup.emergency_admin).is_err());
}

#[test]
fn test_apply_upgrade_admin() {
    let setup = Setup::default();
    let buffer = setup.buffer;
    buffer.commit_upgrade(&setup.admin, &install_dummy_wasm(&setup.env));
    jump(&setup.env, ADMIN_ACTIONS_DELAY + 1);
    assert_ne!(buffer.version(), 130);
    assert!(buffer.try_apply_upgrade(&setup.admin).is_ok());
    assert_eq!(buffer.version(), 130);
}

// emergency mode
#[test]
fn test_set_emergency_mode_third_party_user() {
    let setup = Setup::default();
    let buffer = setup.buffer;
    let user = Address::generate(&setup.env);
    assert!(buffer.try_set_emergency_mode(&user, &false).is_err());
}

#[test]
fn test_set_emergency_mode_emergency_admin() {
    let setup = Setup::default();
    let buffer = setup.buffer;
    assert!(buffer.try_set_emergency_mode(&setup.admin, &false).is_err());
}

#[test]
fn test_set_emergency_mode_admin() {
    let setup = Setup::default();
    let buffer = setup.buffer;
    assert!(buffer.try_set_emergency_mode(&setup.emergency_admin, &false).is_ok());
}
