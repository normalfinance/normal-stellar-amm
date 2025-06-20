#![cfg(test)]
extern crate std;

use crate::testutils::{ Setup };
use crate::{ contract::PoolPlane, PoolPlaneClient };
use access_control::constants::ADMIN_ACTIONS_DELAY;
use soroban_sdk::testutils::{ Address as _, Events };
use soroban_sdk::{ symbol_short, vec, Address, Env, IntoVal, Symbol, Vec };
use utils::test_utils::{ install_dummy_wasm, jump };

fn create_plane_contract<'a>(e: &Env) -> PoolPlaneClient<'a> {
    let client = PoolPlaneClient::new(e, &e.register(PoolPlane {}, ()));
    client
}

#[test]
fn test() {
    let e = Env::default();
    e.mock_all_auths();
    e.cost_estimate().budget().reset_unlimited();

    let address1 = Address::generate(&e);

    let plane = create_plane_contract(&e);
    plane.init_admin(&Address::generate(&e));
    plane.update(
        &address1,
        &Vec::from_array(&e, [30_u128]),
        &Vec::from_array(&e, [1000_u128, 1000_u128])
    );

    let data = plane.get(&Vec::from_array(&e, [address1]));

    let data1 = data.get(0).unwrap();
    assert_eq!(data1.0, Vec::from_array(&e, [30_u128]));
    assert_eq!(data1.1, Vec::from_array(&e, [1000_u128, 1000_u128]));
}

#[should_panic(expected = "Error(Contract, #103)")]
#[test]
fn test_init_admin_twice() {
    let e = Env::default();
    e.mock_all_auths();
    e.cost_estimate().budget().reset_unlimited();

    let admin = Address::generate(&e);
    let plane = create_plane_contract(&e);
    plane.init_admin(&admin);
    plane.init_admin(&admin);
}

#[test]
fn test_transfer_ownership_events() {
    let setup = Setup::default();
    let plane = setup.plane;
    let new_admin = Address::generate(&setup.env);

    plane.commit_transfer_ownership(&setup.admin, &symbol_short!("Admin"), &new_admin);
    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![&setup.env, (
            plane.address.clone(),
            (Symbol::new(&setup.env, "commit_transfer_ownership"), symbol_short!("Admin")).into_val(
                &setup.env
            ),
            (new_admin.clone(),).into_val(&setup.env),
        )]
    );

    plane.revert_transfer_ownership(&setup.admin, &symbol_short!("Admin"));
    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![&setup.env, (
            plane.address.clone(),
            (Symbol::new(&setup.env, "revert_transfer_ownership"), symbol_short!("Admin")).into_val(
                &setup.env
            ),
            ().into_val(&setup.env),
        )]
    );

    plane.commit_transfer_ownership(&setup.admin, &symbol_short!("Admin"), &new_admin);
    jump(&setup.env, ADMIN_ACTIONS_DELAY + 1);
    plane.apply_transfer_ownership(&setup.admin, &symbol_short!("Admin"));
    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![&setup.env, (
            plane.address.clone(),
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
    let contract = setup.plane;
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
    let contract = setup.plane;

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
    let contract = setup.plane;
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
    let contract = setup.plane;
    let new_wasm = install_dummy_wasm(&setup.env);

    assert_eq!(contract.get_emergency_mode(), false);
    assert_ne!(contract.version(), 130);

    contract.commit_upgrade(&setup.admin, &new_wasm);
    assert!(contract.try_apply_upgrade(&setup.admin).is_err());
    jump(&setup.env, ADMIN_ACTIONS_DELAY + 1);
    contract.apply_upgrade(&setup.admin);

    assert_eq!(contract.version(), 130)
}
