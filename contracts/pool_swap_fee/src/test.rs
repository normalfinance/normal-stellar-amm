#![cfg(test)]
extern crate std;

use crate::testutils::Setup;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{ Address, Vec };

#[test]
fn test_strict_send() {
    let setup = Setup::default();

    let tokens = Vec::from_array(&setup.env, [
        setup.token_a.address.clone(),
        setup.token_b.address.clone(),
    ]);
    let (pool_index, _pool_address) = setup.router.get_pools(&tokens).iter().last().unwrap();

    // Record init Buffer balance
    let buffer_balance_before = setup.token_b.balance(&setup.buffer.address);

    // Mint user token_b to swap
    let user = Address::generate(&setup.env);
    setup.token_b_admin_client.mint(&user, &1_0000000);

    // Swap
    let result = setup.fee_collector.swap(
        &user,
        &tokens,
        &setup.token_b.address,
        &setup.token_a.address,
        &pool_index,
        &1_0000000,
        &9870300
    );
    assert_eq!(result, 9870300); // (10000000 - .3%) - 1%
    assert_eq!(setup.token_a.balance(&user), 9870300);
}

#[test]
fn test_strict_send_bad_slippage() {
    let setup = Setup::default();

    let tokens = Vec::from_array(&setup.env, [
        setup.token_a.address.clone(),
        setup.token_b.address.clone(),
    ]);
    let (pool_index, _pool_address) = setup.router.get_pools(&tokens).iter().last().unwrap();

    let user = Address::generate(&setup.env);
    setup.token_b_admin_client.mint(&user, &1_0000000);

    assert!(
        setup.fee_collector
            .try_swap(
                &user,
                &tokens,
                &setup.token_b.address,
                &setup.token_a.address,
                &pool_index,
                &1_0000000,
                &9870301 // value is not enough to cover provider fee
            )
            .is_err()
    );
    assert!(
        setup.fee_collector
            .try_swap(&user, &swap_path, &setup.token_a.address, &1_0000000, &9870300)
            .is_ok()
    );
}

#[test]
fn test_claim_fee() {
    let setup = Setup::default();

    let tokens = Vec::from_array(&setup.env, [
        setup.token_a.address.clone(),
        setup.token_b.address.clone(),
    ]);
    let (pool_index, _pool_address) = setup.router.get_pools(&tokens).iter().last().unwrap();

    let user = Address::generate(&setup.env);
    setup.token_b_admin_client.mint(&user, &1_0000000);

    setup.fee_collector.swap(
        &user,
        &tokens,
        &setup.token_b.address,
        &setup.token_a.address,
        &pool_index,
        &1_0000000,
        &0
    );
    assert_eq!(setup.fee_collector.claim_fees(&setup.admin, &setup.token_b.address), 99699); // ~ (10000000 - .3%) * 1%
    assert_eq!(setup.fee_collector.claim_fees(&setup.admin, &setup.token_a.address), 0);
    assert_eq!(setup.token_a.balance(&setup.fee_destination), 0);
    assert_eq!(setup.token_b.balance(&setup.fee_destination), 99699);
}
