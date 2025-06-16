#![cfg(test)]

use crate::storage_types::{ OracleGuardRails, PriceDivergenceGuardRails, ValidityGuardRails };
use crate::testutils::Setup;
use access_control::constants::ADMIN_ACTIONS_DELAY;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{ symbol_short, Address, Symbol };
use utils::constant::{ ONE_HOUR, PERCENTAGE_PRECISION_U64 };
use utils::storage::MutableOracleInfo;
use utils::test_utils::{ install_dummy_wasm, jump };

// test admin transfer ownership
#[test]
#[should_panic(expected = "Error(Contract, #2908)")]
fn test_admin_transfer_ownership_too_early() {
    let setup = Setup::default();
    let registry = setup.registry;
    let admin_original = setup.users[0].clone();
    let admin_new = Address::generate(&setup.env);

    registry.commit_transfer_ownership(&admin_original, &symbol_short!("Admin"), &admin_new);
    // check admin not changed yet by calling protected method
    assert!(registry.try_revert_transfer_ownership(&admin_new, &symbol_short!("Admin")).is_err());
    jump(&setup.env, ADMIN_ACTIONS_DELAY - 1);
    registry.apply_transfer_ownership(&admin_original, &symbol_short!("Admin"));
}

#[test]
#[should_panic(expected = "Error(Contract, #2906)")]
fn test_admin_transfer_ownership_twice() {
    let setup = Setup::default();
    let registry = setup.registry;
    let admin_original = setup.admin;
    let admin_new = Address::generate(&setup.env);

    registry.commit_transfer_ownership(&admin_original, &symbol_short!("Admin"), &admin_new);
    registry.commit_transfer_ownership(&admin_original, &symbol_short!("Admin"), &admin_new);
}

#[test]
#[should_panic(expected = "Error(Contract, #2907)")]
fn test_admin_transfer_ownership_not_committed() {
    let setup = Setup::default();
    let registry = setup.registry;
    let admin_original = setup.admin;

    jump(&setup.env, ADMIN_ACTIONS_DELAY + 1);
    registry.apply_transfer_ownership(&admin_original, &symbol_short!("Admin"));
}

#[test]
#[should_panic(expected = "Error(Contract, #2907)")]
fn test_admin_transfer_ownership_reverted() {
    let setup = Setup::default();
    let registry = setup.registry;
    let admin_original = setup.admin;
    let admin_new = Address::generate(&setup.env);

    registry.commit_transfer_ownership(&admin_original, &symbol_short!("Admin"), &admin_new);
    // check admin not changed yet by calling protected method
    assert!(registry.try_revert_transfer_ownership(&admin_new, &symbol_short!("Admin")).is_err());
    jump(&setup.env, ADMIN_ACTIONS_DELAY + 1);
    registry.revert_transfer_ownership(&admin_original, &symbol_short!("Admin"));
    registry.apply_transfer_ownership(&admin_original, &symbol_short!("Admin"));
}

#[test]
fn test_admin_transfer_ownership() {
    let setup = Setup::default();
    let registry = setup.registry;
    let admin_original = setup.admin;
    let admin_new = Address::generate(&setup.env);

    registry.commit_transfer_ownership(&admin_original, &symbol_short!("Admin"), &admin_new);
    // check admin not changed yet by calling protected method
    assert!(registry.try_revert_transfer_ownership(&admin_new, &symbol_short!("Admin")).is_err());
    jump(&setup.env, ADMIN_ACTIONS_DELAY + 1);
    registry.apply_transfer_ownership(&admin_original, &symbol_short!("Admin"));

    registry.commit_transfer_ownership(&admin_new, &symbol_short!("Admin"), &admin_new);
}

// test emergency admin transfer ownership
#[test]
#[should_panic(expected = "Error(Contract, #2908)")]
fn test_emergency_admin_transfer_ownership_too_early() {
    let setup = Setup::default();
    let registry = setup.registry;
    let emergency_admin_new = Address::generate(&setup.env);

    registry.commit_transfer_ownership(
        &setup.admin,
        &Symbol::new(&setup.env, "EmergencyAdmin"),
        &emergency_admin_new
    );

    // check emergency admin not changed yet by calling protected method
    assert!(registry.try_set_emergency_mode(&emergency_admin_new, &false).is_err());
    assert!(registry.try_set_emergency_mode(&setup.emergency_admin, &false).is_ok());

    jump(&setup.env, ADMIN_ACTIONS_DELAY - 1);
    registry.apply_transfer_ownership(&setup.admin, &Symbol::new(&setup.env, "EmergencyAdmin"));
}

#[test]
#[should_panic(expected = "Error(Contract, #2906)")]
fn test_emergency_admin_transfer_ownership_twice() {
    let setup = Setup::default();
    let registry = setup.registry;
    let emergency_admin_new = Address::generate(&setup.env);

    registry.commit_transfer_ownership(
        &setup.admin,
        &Symbol::new(&setup.env, "EmergencyAdmin"),
        &emergency_admin_new
    );
    registry.commit_transfer_ownership(
        &setup.admin,
        &Symbol::new(&setup.env, "EmergencyAdmin"),
        &emergency_admin_new
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #2907)")]
fn test_emergency_admin_transfer_ownership_not_committed() {
    let setup = Setup::default();
    let registry = setup.registry;

    jump(&setup.env, ADMIN_ACTIONS_DELAY + 1);
    registry.apply_transfer_ownership(&setup.admin, &Symbol::new(&setup.env, "EmergencyAdmin"));
}

#[test]
#[should_panic(expected = "Error(Contract, #2907)")]
fn test_emergency_admin_transfer_ownership_reverted() {
    let setup = Setup::default();
    let registry = setup.registry;
    let emergency_admin_new = Address::generate(&setup.env);

    registry.commit_transfer_ownership(
        &setup.admin,
        &Symbol::new(&setup.env, "EmergencyAdmin"),
        &emergency_admin_new
    );

    // check emergency admin not changed yet by calling protected method
    assert!(registry.try_set_emergency_mode(&emergency_admin_new, &false).is_err());
    assert!(registry.try_set_emergency_mode(&setup.emergency_admin, &false).is_ok());

    jump(&setup.env, ADMIN_ACTIONS_DELAY + 1);
    registry.revert_transfer_ownership(&setup.admin, &Symbol::new(&setup.env, "EmergencyAdmin"));
    registry.apply_transfer_ownership(&setup.admin, &Symbol::new(&setup.env, "EmergencyAdmin"));
}

#[test]
fn test_emergency_admin_transfer_ownership() {
    let setup = Setup::default();
    let registry = setup.registry;
    let emergency_admin_new = Address::generate(&setup.env);

    registry.commit_transfer_ownership(
        &setup.admin,
        &Symbol::new(&setup.env, "EmergencyAdmin"),
        &emergency_admin_new
    );

    // check emergency admin not changed yet by calling protected method
    assert!(registry.try_set_emergency_mode(&emergency_admin_new, &false).is_err());
    assert!(registry.try_set_emergency_mode(&setup.emergency_admin, &false).is_ok());

    jump(&setup.env, ADMIN_ACTIONS_DELAY + 1);
    registry.apply_transfer_ownership(&setup.admin, &Symbol::new(&setup.env, "EmergencyAdmin"));

    // check emergency admin has changed
    assert!(registry.try_set_emergency_mode(&emergency_admin_new, &false).is_ok());
    assert!(registry.try_set_emergency_mode(&setup.emergency_admin, &false).is_err());
}

#[test]
fn test_transfer_ownership_separate_deadlines() {
    let setup = Setup::default();
    let registry = setup.registry;
    let admin_new = Address::generate(&setup.env);
    let emergency_admin_new = Address::generate(&setup.env);

    assert_eq!(
        registry.get_future_address(&Symbol::new(&setup.env, "EmergencyAdmin")),
        setup.emergency_admin
    );
    assert_eq!(registry.get_future_address(&symbol_short!("Admin")), setup.admin);

    assert!(registry.try_set_emergency_mode(&emergency_admin_new, &false).is_err());
    assert!(registry.try_set_emergency_mode(&setup.emergency_admin, &false).is_ok());

    registry.commit_transfer_ownership(
        &setup.admin,
        &Symbol::new(&setup.env, "EmergencyAdmin"),
        &emergency_admin_new
    );
    jump(&setup.env, 10);
    registry.commit_transfer_ownership(&setup.admin, &symbol_short!("Admin"), &admin_new);

    assert_eq!(
        registry.get_future_address(&Symbol::new(&setup.env, "EmergencyAdmin")),
        emergency_admin_new
    );
    assert_eq!(registry.get_future_address(&symbol_short!("Admin")), admin_new);

    jump(&setup.env, ADMIN_ACTIONS_DELAY + 1 - 10);
    registry.apply_transfer_ownership(&setup.admin, &Symbol::new(&setup.env, "EmergencyAdmin"));
    assert!(registry.try_apply_transfer_ownership(&setup.admin, &symbol_short!("Admin")).is_err());

    assert_eq!(
        registry.get_future_address(&Symbol::new(&setup.env, "EmergencyAdmin")),
        emergency_admin_new
    );

    jump(&setup.env, 10);
    registry.apply_transfer_ownership(&setup.admin, &symbol_short!("Admin"));

    assert_eq!(registry.get_future_address(&symbol_short!("Admin")), admin_new);

    // check ownership transfer is complete. new admin is capable to call protected methods
    //      and new emergency admin can change toggle emergency mode
    registry.commit_transfer_ownership(&admin_new, &Symbol::new(&setup.env, "Admin"), &setup.admin);
    assert!(registry.try_set_emergency_mode(&emergency_admin_new, &false).is_ok());
    assert!(registry.try_set_emergency_mode(&setup.emergency_admin, &false).is_err());
}

// upgrade pool & token
#[test]
fn test_commit_upgrade() {
    let setup = Setup::default();
    let registry = setup.registry;
    let new_wasm = install_dummy_wasm(&setup.env);
    let user = Address::generate(&setup.env);

    for (addr, is_ok) in [
        (user, false),
        (setup.admin, true),
        (setup.emergency_admin, false),
    ] {
        assert_eq!(registry.try_commit_upgrade(&addr, &new_wasm).is_ok(), is_ok);
    }
}

#[test]
fn test_apply_upgrade_third_party_user() {
    let setup = Setup::default();
    let registry = setup.registry;
    let user = Address::generate(&setup.env);
    registry.commit_upgrade(&setup.admin, &install_dummy_wasm(&setup.env));
    jump(&setup.env, ADMIN_ACTIONS_DELAY + 1);
    assert!(registry.try_apply_upgrade(&user).is_err());
}

#[test]
fn test_apply_upgrade_emergency_admin() {
    let setup = Setup::default();
    let registry = setup.registry;
    registry.commit_upgrade(&setup.admin, &install_dummy_wasm(&setup.env));
    jump(&setup.env, ADMIN_ACTIONS_DELAY + 1);
    assert!(registry.try_apply_upgrade(&setup.emergency_admin).is_err());
}

#[test]
fn test_apply_upgrade_admin() {
    let setup = Setup::default();
    let registry = setup.registry;
    // let token = ShareTokenClient::new(&setup.env, &registry.share_id());
    let new_wasm = install_dummy_wasm(&setup.env);

    assert_ne!(registry.version(), 130);
    // assert_ne!(token.version(), 130);

    registry.commit_upgrade(&setup.admin, &new_wasm);
    jump(&setup.env, ADMIN_ACTIONS_DELAY + 1);
    assert_eq!(registry.apply_upgrade(&setup.admin), new_wasm);

    // check contracts updated, dummy contract version is 130
    assert_eq!(registry.version(), 130);
    // assert_eq!(token.version(), 130);
}

// emergency mode
#[test]
fn test_set_emergency_mode_third_party_user() {
    let setup = Setup::default();
    let registry = setup.registry;
    let user = Address::generate(&setup.env);
    assert!(registry.try_set_emergency_mode(&user, &false).is_err());
}

#[test]
fn test_set_emergency_mode_emergency_admin() {
    let setup = Setup::default();
    let registry = setup.registry;
    assert!(registry.try_set_emergency_mode(&setup.admin, &false).is_err());
}

#[test]
fn test_set_emergency_mode_admin() {
    let setup = Setup::default();
    let registry = setup.registry;
    assert!(registry.try_set_emergency_mode(&setup.emergency_admin, &false).is_ok());
}

// admin setters

#[test]
fn test_set_oracle_guard_rails() {
    let setup = Setup::default();
    let registry = setup.registry;
    let user = Address::generate(&setup.env);

    let new_oracle_guard_rails = OracleGuardRails {
        price_divergence: PriceDivergenceGuardRails {
            oracle_twap_percent_divergence: PERCENTAGE_PRECISION_U64 / 3,
        },
        validity: ValidityGuardRails {
            slots_before_stale_for_pool: 12, // ~5 seconds
            confidence_interval_max_size: 20_000, // 2% of price
            too_volatile_ratio: 5, // 5x or 80% down
        },
    };

    for (addr, is_ok) in [
        (user, false),
        (setup.admin, true),
    ] {
        assert_eq!(
            registry.try_set_oracle_guardrails(&addr, &new_oracle_guard_rails).is_ok(),
            is_ok
        );
    }
}

#[test]
fn test_set_price_override_limit() {
    let setup = Setup::default();
    let registry = setup.registry;
    let user = Address::generate(&setup.env);

    for (addr, is_ok) in [
        (user, false),
        (setup.admin, true),
    ] {
        assert_eq!(registry.try_set_price_override_limit(&addr, &110_u32).is_ok(), is_ok);
    }
}

#[test]
fn test_set_price_override_threshold() {
    let setup = Setup::default();
    let registry = setup.registry;
    let user = Address::generate(&setup.env);

    for (addr, is_ok) in [
        (user, false),
        (setup.admin, true),
    ] {
        assert_eq!(registry.try_set_price_override_threshold(&addr, &ONE_HOUR).is_ok(), is_ok);
    }
}

#[test]
fn test_register_oracle() {
    let setup = Setup::default();
    let registry = setup.registry;
    let user = Address::generate(&setup.env);

    let new_asset_id = Symbol::new(&setup.env, "ETH");
    let new_oracle = Address::generate(&setup.env);

    for (addr, is_ok) in [
        (user, false),
        (setup.admin, true),
    ] {
        assert_eq!(
            registry
                .try_register_oracle(&addr, &new_asset_id, &new_oracle, &new_oracle, &7, &0)
                .is_ok(),
            is_ok
        );
    }
}

#[test]
fn test_update_oracle() {
    let setup = Setup::default();
    let registry = setup.registry;
    let user = Address::generate(&setup.env);
    let update = MutableOracleInfo {
        decimals: Some(9),
        ..MutableOracleInfo::new()
    };

    for (addr, is_ok) in [
        (user, false),
        (setup.admin, true),
    ] {
        assert_eq!(registry.try_update_oracle(&addr, &setup.asset_id, &update).is_ok(), is_ok);
    }
}

#[test]
fn test_set_oracle_price() {
    let setup = Setup::default();
    let registry = setup.registry;
    let user = Address::generate(&setup.env);

    for (addr, is_ok) in [
        (user, false),
        (setup.admin, true),
    ] {
        assert_eq!(registry.try_set_oracle_price(&addr, &setup.asset_id, &12).is_ok(), is_ok);
    }
}
