#![cfg(test)]
extern crate std;

use crate::constants::CONSTANT_PRODUCT_FEE_AVAILABLE;
use crate::testutils;
use crate::testutils::{ test_token, Setup };
use access_control::constants::ADMIN_ACTIONS_DELAY;
use soroban_sdk::testutils::{
    AuthorizedFunction,
    AuthorizedInvocation,
    Events,
    MockAuth,
    MockAuthInvoke,
};
use soroban_sdk::String;
use soroban_sdk::{
    symbol_short,
    testutils::Address as _,
    vec,
    Address,
    FromVal,
    IntoVal,
    Map,
    Symbol,
    Val,
    Vec,
    U256,
};
use utils::test_utils::{
    assert_approx_eq_abs,
    assert_approx_eq_abs_u256,
    install_dummy_wasm,
    jump,
};

#[test]
#[should_panic(expected = "Error(Contract, #103)")]
fn test_init_admin_twice() {
    let setup = Setup::default();
    setup.registry.init_admin(&setup.admin);
}

#[test]
fn test_total_liquidity() {
    let setup = Setup::default();
    let e = setup.env;
    let user1 = Address::generate(&e);
    
}


#[test]
fn test_privileged_users() {
    let setup = Setup::default();
    let e = setup.env;
    let router = setup.router;

    let [token1, token2, _, _] = setup.tokens;
    let reward_token = setup.reward_token;

    let tokens = Vec::from_array(&e, [token1.address.clone(), token2.address.clone()]);

    let user1 = Address::generate(&e);
    reward_token.mint(&user1, &10_0000000);

    let (_, standard_address) = router.init_standard_pool(
        &user1,
        &setup.oracles,
        &setup.target_asset,
        &tokens,
        &String::from_str(&e, "Pool Share Token"),
        &String::from_str(&e, "Pool Share Token"),
        &30
    );
    let privileged_addrs: Map<Symbol, Vec<Address>> = Map::from_array(&e, [
        (Symbol::new(&e, "Admin"), Vec::from_array(&e, [setup.admin])),
        (Symbol::new(&e, "EmergencyAdmin"), Vec::from_array(&e, [setup.emergency_admin])),
        (Symbol::new(&e, "RewardsAdmin"), Vec::from_array(&e, [setup.rewards_admin])),
        (Symbol::new(&e, "OperationsAdmin"), Vec::from_array(&e, [setup.operations_admin])),
        (Symbol::new(&e, "PauseAdmin"), Vec::from_array(&e, [setup.pause_admin])),
        (
            Symbol::new(&e, "EmergencyPauseAdmin"),
            Vec::from_array(&e, [setup.emergency_pause_admin]),
        ),
    ]);
    assert_eq!(privileged_addrs, router.get_privileged_addrs());
    // test addresses inheritance
    assert_eq!(
        privileged_addrs,
        testutils::standard_pool::Client::new(&e, &standard_address).get_privileged_addrs()
    );
}

#[test]
fn test_set_privileged_addresses_event() {
    let setup = Setup::default();
    let router = setup.router;

    router.set_privileged_addrs(
        &setup.admin.clone(),
        &setup.rewards_admin.clone(),
        &setup.operations_admin.clone(),
        &setup.pause_admin.clone(),
        &Vec::from_array(&setup.env, [setup.emergency_pause_admin.clone()])
    );

    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![&setup.env, (
            router.address.clone(),
            (Symbol::new(&setup.env, "set_privileged_addrs"),).into_val(&setup.env),
            (
                setup.rewards_admin,
                setup.operations_admin,
                setup.pause_admin,
                Vec::from_array(&setup.env, [setup.emergency_pause_admin]),
            ).into_val(&setup.env),
        )]
    );
}

#[test]
fn test_transfer_ownership_events() {
    let setup = Setup::default();
    let router = setup.router;
    let new_admin = Address::generate(&setup.env);

    router.commit_transfer_ownership(&setup.admin, &symbol_short!("Admin"), &new_admin);
    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![&setup.env, (
            router.address.clone(),
            (Symbol::new(&setup.env, "commit_transfer_ownership"), symbol_short!("Admin")).into_val(
                &setup.env
            ),
            (new_admin.clone(),).into_val(&setup.env),
        )]
    );

    router.revert_transfer_ownership(&setup.admin, &symbol_short!("Admin"));
    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![&setup.env, (
            router.address.clone(),
            (Symbol::new(&setup.env, "revert_transfer_ownership"), symbol_short!("Admin")).into_val(
                &setup.env
            ),
            ().into_val(&setup.env),
        )]
    );

    router.commit_transfer_ownership(&setup.admin, &symbol_short!("Admin"), &new_admin);
    jump(&setup.env, ADMIN_ACTIONS_DELAY + 1);
    router.apply_transfer_ownership(&setup.admin, &symbol_short!("Admin"));
    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![&setup.env, (
            router.address.clone(),
            (Symbol::new(&setup.env, "apply_transfer_ownership"), symbol_short!("Admin")).into_val(
                &setup.env
            ),
            (new_admin.clone(),).into_val(&setup.env),
        )]
    );
}

#[test]
fn test_upgrade_events() {
    let setup = Setup::default();
    let contract = setup.router;
    let new_wasm_hash = install_dummy_wasm(&setup.env);

    contract.commit_upgrade(&setup.admin, &new_wasm_hash);
    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![&setup.env, (
            contract.address.clone(),
            (Symbol::new(&setup.env, "commit_upgrade"),).into_val(&setup.env),
            (new_wasm_hash.clone(),).into_val(&setup.env),
        )]
    );

    contract.revert_upgrade(&setup.admin);
    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![&setup.env, (
            contract.address.clone(),
            (Symbol::new(&setup.env, "revert_upgrade"),).into_val(&setup.env),
            ().into_val(&setup.env),
        )]
    );

    contract.commit_upgrade(&setup.admin, &new_wasm_hash);
    jump(&setup.env, ADMIN_ACTIONS_DELAY + 1);
    contract.apply_upgrade(&setup.admin);
    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![&setup.env, (
            contract.address.clone(),
            (Symbol::new(&setup.env, "apply_upgrade"),).into_val(&setup.env),
            (new_wasm_hash.clone(),).into_val(&setup.env),
        )]
    );
}

#[test]
fn test_emergency_mode_events() {
    let setup = Setup::default();
    let contract = setup.router;

    contract.set_emergency_mode(&setup.emergency_admin, &true);
    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![&setup.env, (
            contract.address.clone(),
            (Symbol::new(&setup.env, "enable_emergency_mode"),).into_val(&setup.env),
            ().into_val(&setup.env),
        )]
    );
    contract.set_emergency_mode(&setup.emergency_admin, &false);
    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![&setup.env, (
            contract.address.clone(),
            (Symbol::new(&setup.env, "disable_emergency_mode"),).into_val(&setup.env),
            ().into_val(&setup.env),
        )]
    );
}

#[test]
fn test_emergency_upgrade() {
    let setup = Setup::default();
    let contract = setup.router;
    let new_wasm = install_dummy_wasm(&setup.env);

    assert_eq!(contract.get_emergency_mode(), false);
    assert_ne!(contract.version(), 130);
    contract.set_emergency_mode(&setup.emergency_admin, &true);

    contract.commit_upgrade(&setup.admin, &new_wasm);
    contract.apply_upgrade(&setup.admin);

    assert_eq!(contract.version(), 130)
}

#[test]
fn test_regular_upgrade() {
    let setup = Setup::default();
    let contract = setup.router;
    let new_wasm = install_dummy_wasm(&setup.env);

    assert_eq!(contract.get_emergency_mode(), false);
    assert_ne!(contract.version(), 130);

    contract.commit_upgrade(&setup.admin, &new_wasm);
    assert!(contract.try_apply_upgrade(&setup.admin).is_err());
    jump(&setup.env, ADMIN_ACTIONS_DELAY + 1);
    contract.apply_upgrade(&setup.admin);

    assert_eq!(contract.version(), 130)
}
