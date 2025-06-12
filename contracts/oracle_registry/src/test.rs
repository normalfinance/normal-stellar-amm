#![cfg(test)]
extern crate std;

use crate::storage_types::{ OracleGuardRails, PriceDivergenceGuardRails, ValidityGuardRails };
use crate::testutils::Setup;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{ Address };
use utils::constant::PERCENTAGE_PRECISION_U64;
use utils::storage::OracleInfo;

#[test]
fn test_get_price() {
    let setup = Setup::default();
    let oracle_price_data = setup.oracle_registry.get_price(
        &setup.user,
        &setup.asset_id,
        &false,
        &None
    );
    assert_eq!(oracle_price_data.price, 100);
    assert_eq!(oracle_price_data.delay, 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #501)")]
fn test_get_price_of_unregistered_oracle() {
    let setup = Setup::default();
    setup.oracle_registry.get_price(&setup.user, &setup.unregistered_asset_id, &false, &None);
}

#[test]
fn test_set_oracle_guardrails() {
    let setup = Setup::new_with_config(
        &(TestConfig {
            ..TestConfig::default()
        })
    );

    // let guardrails_before = setup.oracle_registry.get_oracle_guardrails();

    let new_guardrails = OracleGuardRails {
        price_divergence: PriceDivergenceGuardRails {
            oracle_twap_percent_divergence: 10,
        },
        ..setup.oracle_guardrails
    };

    let oracle_price_data = setup.oracle_registry.set_oracle_guardrails(
        &setup.admin,
        &new_guardrails
    );

    assert_eq!(setup.oracle_registry.get_oracle_guardrails(), new_guardrails);
}

#[test]
#[should_panic(expected = "Error(Contract, #102)")]
fn test_set_oracle_guardrails_not_admin() {
    let setup = Setup::default();
    setup.oracle_registry.set_oracle_guardrails(&setup.user, {});
}

#[test]
fn test_set_price_override_limit() {
    let setup = Setup::default();

    let limit_before = setup.oracle_registry.get_price_override_limit();

    let oracle_price_data = setup.oracle_registry.set_price_override_limit(&setup.admin, &100_u128);

    assert_eq!(setup.oracle_registry.get_price_override_limit(), 100);
}

#[test]
#[should_panic(expected = "Error(Contract, #102)")]
fn test_set_price_override_limit_not_admin() {
    let setup = Setup::default();
    setup.oracle_registry.set_price_override_limit(&setup.user, &100_u128);
}

#[test]
fn test_register_oracle() {
    let setup = Setup::default();
    let oracle_info = setup.oracle_registry.register_oracle(
        &setup.admin,
        &setup.asset_id,
        false,
        None
    );
    assert_eq!(setup.token_b.balance(&user), 9870300);
}

#[test]
#[should_panic(expected = "Error(Contract, #102)")]
fn test_register_oracle_not_admin() {
    let setup = Setup::default();
    setup.oracle_registry.register_oracle(
        &setup.user,
        &setup.asset_id,
        &Address::generate(&setup.env),
        None
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #2906)")]
fn test_register_oracle_already_exists() {
    let setup = Setup::default();
    setup.oracle_registry.register_oracle(&setup.admin, &setup.asset_id, false, None);
}

#[test]
#[should_panic(expected = "Error(Contract, #2906)")]
fn test_register_oracle_no_response() {
    let setup = Setup::default();
    setup.oracle_registry.register_oracle(
        &setup.admin,
        &setup.asset_id,
        &Address::generate(&setup.env),
        &Address::generate(&setup.env),
        &7
    );
}

#[test]
fn test_set_oracle_address() {
    let setup = Setup::default();
    let new_address = Address::generate(&setup.env);
    let oracle_info = setup.oracle_registry.set_oracle_address(
        &setup.admin,
        &setup.asset_id,
        &new_address
    );

    assert_eq!(oracle_info, OracleInfo {
        oracle_address: new_address,
        last_updated: setup.env.ledger().timestamp(),
        ..oracle_info
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #102)")]
fn test_set_oracle_address_not_admin() {
    let setup = Setup::default();
    setup.oracle_registry.set_oracle_address(
        &setup.user,
        &setup.asset_id,
        &Address::generate(&setup.env)
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #2906)")]
fn test_set_oracle_address_no_response() {
    let setup = Setup::default();
    setup.oracle_registry.set_oracle_address(
        &setup.admin,
        &setup.asset_id,
        &Address::generate(&setup.env)
    );
}

#[test]
fn test_set_oracle_decimals() {
    let setup = Setup::default();
    let oracle_info = setup.oracle_registry.set_oracle_decimals(&setup.admin, &setup.asset_id, &9);
    assert_eq!(oracle_info.decimals, 9);
}

#[test]
#[should_panic(expected = "Error(Contract, #102)")]
fn test_set_oracle_decimals_not_admin() {
    let setup = Setup::default();
    setup.oracle_registry.set_oracle_decimals(&setup.user, &setup.asset_id, &9);
}

#[test]
#[should_panic(expected = "Error(Contract, #2906)")]
fn test_set_oracle_decimals_invalid() {
    let setup = Setup::default();
    setup.oracle_registry.set_oracle_decimals(&setup.admin, &setup.asset_id, &100);
}

#[test]
fn test_sync_oracle_price() {
    let setup = Setup::default();
    let oracle_info = setup.oracle_registry.sync_oracle_price(&setup.admin, &setup.asset_id, &None);
    assert_eq!(oracle_info.decimals, 9);
}

#[test]
fn test_sync_oracle_price_with_clamp() {
    let setup = Setup::default();
    let oracle_info = setup.oracle_registry.sync_oracle_price(&setup.admin, &setup.asset_id, &None);
    assert_eq!(oracle_info.decimals, 9);
}

#[test]
#[should_panic(expected = "Error(Contract, #2906)")]
fn test_sync_oracle_price_no_responsive() {
    let setup = Setup::default();
    setup.oracle_registry.sync_oracle_price(&setup.admin, &setup.asset_id, &None);
}

#[test]
fn test_set_oracle_price() {
    let setup = Setup::default();
    let oracle_info = setup.oracle_registry.set_oracle_price(
        &setup.admin,
        &setup.asset_id,
        &9,
        &10
    );
    assert_eq!(oracle_info.decimals, 9);
}

#[test]
#[should_panic(expected = "Error(Contract, #102)")]
fn test_set_oracle_price_not_admin() {
    let setup = Setup::default();
    setup.oracle_registry.set_oracle_price(&setup.user, &setup.asset_id, &9, &10);
}

#[test]
#[should_panic(expected = "Error(Contract, #20)")]
fn test_set_oracle_price_outside_limit() {
    let setup = Setup::default();
    setup.oracle_registry.set_oracle_price(&setup.admin, &setup.asset_id, &9, &10);
}

#[test]
fn test_freeze_oracle() {
    let setup = Setup::default();
    let price_before = 0;
    let oracle_info = setup.oracle_registry.freeze_oracle(&setup.admin, &setup.asset_id);
    assert_eq!(oracle_info.frozen, true);

    // Ensure price cannot be updated
    let oracle_price_data = setup.oracle_registry.get_price(
        &setup.user,
        &setup.asset_id,
        &false,
        &None
    );
    assert_eq!(oracle_price_data.price, price_before);
}

#[test]
#[should_panic(expected = "Error(Contract, #102)")]
fn test_freeze_oracle_not_admin() {
    let setup = Setup::default();
    setup.oracle_registry.freeze_oracle(&setup.user, &setup.asset_id);
}

#[test]
fn test_unfreeze_oracle() {
    let setup = Setup::default();
    let price_before = 0;
    let oracle_info = setup.oracle_registry.unfreeze_oracle(&setup.admin, &setup.asset_id);
    assert_eq!(oracle_info.frozen, false);

    // Ensure price can be updated
    let oracle_price_data = setup.oracle_registry.get_price(
        &setup.user,
        &setup.asset_id,
        &false,
        &None
    );
    assert_ne!(oracle_price_data.price, price_before);
}

#[test]
#[should_panic(expected = "Error(Contract, #102)")]
fn test_unfreeze_oracle_not_admin() {
    let setup = Setup::default();
    setup.oracle_registry.unfreeze_oracle(&setup.user, &setup.asset_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #501)")]
fn test_unfreeze_oracle_does_not_exist() {
    let setup = Setup::default();
    setup.oracle_registry.unfreeze_oracle(&setup.user, &setup.unregistered_asset_id);
}
