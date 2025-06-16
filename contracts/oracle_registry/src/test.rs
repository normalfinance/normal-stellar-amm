#![cfg(test)]
extern crate std;

use crate::storage_types::{
    HistoricalOracleData,
    OracleGuardRails,
    PriceDivergenceGuardRails,
    ValidityGuardRails,
};
use crate::testutils::{ update_oracle_price, Setup };
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{ Address, Vec };
use utils::constant::{ ONE_HOUR, PERCENTAGE_PRECISION_U64 };
use utils::storage::{ MutableOracleInfo, OracleInfo };
use utils::test_utils::jump;

#[test]
#[should_panic(expected = "Error(Contract, #103)")]
fn test_initialize_twice() {
    let setup = Setup::default();
    setup.registry.initialize(&setup.admin, &setup.emergency_admin);
}

// get price

#[test]
fn test_get_price() {
    let setup = Setup::default();
    let new_oracle_price = 100_0000000_u128;
    let now = setup.env.ledger().timestamp();

    // Fetch oracle
    let oracle_info = setup.registry.get_oracle(&setup.asset_id);

    // Set mock price
    update_oracle_price(&setup, &oracle_info.address, new_oracle_price, &now);

    // Fetch price from registry
    let oracle_price_data = setup.registry.get_price(&setup.asset_id, &false);

    assert_eq!(oracle_price_data.price, new_oracle_price);
    assert_eq!(oracle_price_data.delay, 0);

    // Ensure historical data is updated
    let last_price_info = setup.registry.get_last_price(&setup.asset_id);
    assert_eq!(last_price_info, HistoricalOracleData {
        last_oracle_price: new_oracle_price,
        last_oracle_delay: 0,
        last_oracle_price_twap: new_oracle_price,
        last_oracle_price_twap_ts: now,
    })
}

#[test]
#[should_panic(expected = "Error(Contract, #501)")]
fn test_get_price_with_invalid_asset_id() {
    let setup = Setup::default();
    setup.registry.get_price(&setup.unregistered_asset_id, &false);
}

#[test]
fn test_get_price_cached() {
    let setup = Setup::default();
    let oracle_price_data = setup.registry.get_price(&setup.asset_id, &true);

    // TODO: price should come from historical oracle data
    assert_eq!(oracle_price_data.price, 100);
    assert_eq!(oracle_price_data.delay, 0);
}

#[test]
fn test_get_price_oracle_frozen() {
    let setup = Setup::default();
    let oracle_price_data = setup.registry.get_price(&setup.asset_id, &false);
    // TODO: price should come from historical oracle data
    assert_eq!(oracle_price_data.price, 100);
    assert_eq!(oracle_price_data.delay, 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #18)")]
fn test_get_price_invalid_oracle_price_nonpositive() {
    let setup = Setup::default();
    let now = setup.env.ledger().timestamp();

    // Fetch oracle
    let oracle_info = setup.registry.get_oracle(&setup.asset_id);

    // Set mock price
    update_oracle_price(&setup, &oracle_info.address, 0, &now);

    setup.registry.get_price(&setup.asset_id, &false);
}

#[test]
#[should_panic(expected = "Error(Contract, #18)")]
fn test_get_price_invalid_oracle_delay() {
    let setup = Setup::default();
    let now = setup.env.ledger().timestamp();

    // Fetch oracle
    let oracle_info = setup.registry.get_oracle(&setup.asset_id);

    // Set mock price
    update_oracle_price(&setup, &oracle_info.address, 0, &now);

    // Make the price stale
    let guardrails = setup.registry.get_oracle_guardrails();
    jump(&setup.env, guardrails.validity.slots_before_stale_for_pool + ONE_HOUR);

    setup.registry.get_price(&setup.asset_id, &false);
}

#[test]
#[should_panic(expected = "Error(Contract, #18)")]
fn test_get_price_invalid_oracle_price_volatility() {
    let setup = Setup::default();
    let new_oracle_price = 100_0000000_u128;
    let now = setup.env.ledger().timestamp();

    // Fetch oracle
    let oracle_info = setup.registry.get_oracle(&setup.asset_id);

    // Set mock price
    update_oracle_price(&setup, &oracle_info.address, new_oracle_price, &now);

    jump(&setup.env, 5);

    // Set the price with volatility
    let too_volatile_price = new_oracle_price.checked_mul(20).unwrap().checked_div(100).unwrap();
    update_oracle_price(&setup, &oracle_info.address, too_volatile_price, &now);

    setup.registry.get_price(&setup.asset_id, &false);
}

// register oracle

#[test]
fn test_register_oracle() {
    let setup = Setup::default();
    let oracle = Address::generate(&setup.env);
    let asset = Address::generate(&setup.env);

    let oracle_info = setup.registry.register_oracle(
        &setup.admin,
        &setup.unregistered_asset_id,
        &oracle,
        &asset,
        &7
    );

    // assert_eq!(setup.token_b.balance(&user), 9870300);
}

#[test]
#[should_panic(expected = "Error(Contract, #2906)")]
fn test_register_oracle_already_exists() {
    let setup = Setup::default();
    setup.registry.register_oracle(&setup.admin, &setup.asset_id, false, None);
}

#[test]
#[should_panic(expected = "Error(Contract, #2906)")]
fn test_register_oracle_no_response() {
    let setup = Setup::default();
    setup.registry.register_oracle(
        &setup.admin,
        &setup.asset_id,
        &Address::generate(&setup.env),
        &Address::generate(&setup.env),
        &7
    );
}

// update oracle (address, decimals, sanitize_clamp, freeze)

#[test]
#[should_panic(expected = "Error(Contract, #19)")]
fn test_update_oracle_does_not_exist() {
    let setup = Setup::default();
    setup.registry.update_oracle(&setup.user, &setup.unregistered_asset_id);
}

#[test]
fn test_set_address() {
    let setup = Setup::default();

    let update = MutableOracleInfo {
        address: Some(Address::generate(&setup.env)),
        ..MutableOracleInfo::new()
    };

    let oracle_info = setup.registry.update_oracle(&setup.admin, &setup.asset_id, &update);

    assert_eq!(oracle_info, OracleInfo {
        address: new_address,
        last_updated: setup.env.ledger().timestamp(),
        ..oracle_info
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #2906)")]
fn test_update_oracle_no_response() {
    let setup = Setup::default();
    setup.registry.set_address(
        &setup.admin,
        &setup.asset_id,
        &Address::generate(&setup.env)
    );
}

#[test]
fn test_set_oracle_decimals() {
    let setup = Setup::default();
    let oracle_info = setup.registry.update_oracle(&setup.admin, &setup.asset_id, &9);
    assert_eq!(oracle_info.decimals, 9);
}

#[test]
#[should_panic(expected = "Error(Contract, #2906)")]
fn test_set_oracle_decimals_invalid() {
    let setup = Setup::default();

    let update = MutableOracleInfo {
        decimals: Some(100),
        ..MutableOracleInfo::new()
    };

    setup.registry.update_oracle(&setup.admin, &setup.asset_id, &update);
}

#[test]
fn test_set_oracle_price() {
    let setup = Setup::default();
    let oracle_info = setup.registry.set_oracle_price(&setup.admin, &setup.asset_id, &9, &10);
    assert_eq!(oracle_info.decimals, 9);
}

#[test]
#[should_panic(expected = "Error(Contract, #20)")]
fn test_set_oracle_price_outside_limit() {
    let setup = Setup::default();
    setup.registry.set_oracle_price(&setup.admin, &setup.asset_id, &9, &10);
}

#[test]
fn test_freeze_oracle() {
    let setup = Setup::default();

    let update = MutableOracleInfo {
        frozen: Some(true),
        ..MutableOracleInfo::new()
    };

    let price_before = 0;
    let oracle_info = setup.registry.update_oracle(&setup.admin, &setup.asset_id, &update);
    assert_eq!(oracle_info.frozen, true);

    // Ensure price cannot be updated
    let oracle_price_data = setup.registry.get_price(&setup.user, &setup.asset_id, &false);
    assert_eq!(oracle_price_data.price, price_before);
}

#[test]
fn test_unfreeze_oracle() {
    let setup = Setup::default();

    let freeze_update = MutableOracleInfo {
        frozen: Some(true),
        ..MutableOracleInfo::new()
    };
    setup.registry.update_oracle(&setup.admin, &setup.asset_id, &freeze_update);

    let last_price_before = setup.registry.get_last_price(&setup.asset_id);

    jump(setup.env, 10);
    update_oracle_price(&setup, oracle, new_price, now);

    let unfreeze_update = MutableOracleInfo {
        frozen: Some(false),
        ..MutableOracleInfo::new()
    };
    let oracle_info = setup.registry.update_oracle(&setup.admin, &setup.asset_id, &unfreeze_update);

    // Ensure frozen is set to false
    assert_eq!(oracle_info.frozen, false);

    // Ensure price was not updated while frozen
    let last_price = setup.registry.get_last_price(&setup.asset_id);
    assert_eq!(last_price, last_price_before);

    // Ensure price can now be updated (calling get_price() will update it)
    let oracle_price_data = setup.registry.get_price(&setup.asset_id, &false);
    assert_ne!(oracle_price_data.price, price_before);
}

//  admin

#[test]
fn test_set_oracle_guardrails() {
    let setup = Setup::new_with_config(
        &(TestConfig {
            ..TestConfig::default()
        })
    );

    // let guardrails_before = setup.registry.get_oracle_guardrails();

    let new_guardrails = OracleGuardRails {
        price_divergence: PriceDivergenceGuardRails {
            oracle_twap_percent_divergence: 10,
        },
        ..setup.oracle_guardrails
    };

    let oracle_price_data = setup.registry.set_oracle_guardrails(&setup.admin, &new_guardrails);

    assert_eq!(setup.registry.get_oracle_guardrails(), new_guardrails);
}

#[test]
fn test_set_price_override_limit() {
    let setup = Setup::default();

    let limit_before = setup.registry.get_price_override_limit();

    setup.registry.set_price_override_limit(&setup.admin, &100_u32);

    let updated_limit = setup.registry.get_price_override_limit();
    assert_eq!(updated_limit, 100);
    assert_ne!(limit_before, updated_limit);
}

#[test]
fn test_set_price_override_threshold() {
    let setup = Setup::default();

    let threshold_before = setup.registry.get_price_override_threshold();

    setup.registry.set_price_override_threshold(&setup.admin, &ONE_HOUR);

    let updated_threshold = setup.registry.get_price_override_threshold();
    assert_eq!(updated_threshold, ONE_HOUR);
    assert_ne!(threshold_before, updated_threshold);
}
