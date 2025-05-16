#![cfg(test)]
extern crate std;

use crate::testutils::{create_token_contract, get_token_admin_client, Setup};
use soroban_sdk::testutils::Address as _;
use soroban_sdk::token::TokenClient;
use soroban_sdk::{vec, Address, Vec};

#[test]
fn test_integration() {
    let setup = Setup::default();

    // create tokens
    let mut tokens = std::vec![
        create_token_contract(&setup.env, &setup.admin).address,
        create_token_contract(&setup.env, &setup.admin).address,
        create_token_contract(&setup.env, &setup.admin).address
    ];
    tokens.sort();
    let xlm = TokenClient::new(&setup.env, &tokens[0]);
    let usdc = TokenClient::new(&setup.env, &tokens[1]);

    let xlm_admin = get_token_admin_client(&setup.env, &xlm.address);
    let usdc_admin = get_token_admin_client(&setup.env, &usdc.address);

    // deploy pools
    let (standard_pool, standard_pool_hash) =
        setup.deploy_standard_pool(&xlm.address, &usdc.address, 30);
    xlm_admin.mint(&setup.admin, &344_000_0000000);
    usdc_admin.mint(&setup.admin, &100_000_0000000);
    standard_pool.deposit(&setup.admin, &100_000_0000000, &0);

    // swap through many pools at once
    let user = Address::generate(&setup.env);
    xlm_admin.mint(&user, &10_0000000);

    let tokens = Vec::from_array(&setup.env, [usdc.address.clone(), xlm.address.clone()]);
    let (pool_index, _pool_address) = setup.router.get_pools(&tokens).iter().last().unwrap();

    assert_eq!(
        setup.router.swap(
            &user,
            &tokens,
            &usdc.address,
            &xlm.address,
            &pool_index,
            &10_0000000,
            &2_8952731
        ),
        2_8952731
    );

    // deploy provider swap fee contract
    let swap_fee = setup.deploy_swap_fee_contract(&setup.operator, &setup.admin, 1000);

    // now swap with additional provider fee
    xlm_admin.mint(&user, &10_0000000);
    assert_eq!(
        swap_fee.swap(
            &user,
            &(
                vec![&setup.env, xlm.address.clone(), usdc.address.clone()],
                standard_pool_hash.clone(),
                usdc.address.clone(),
            ),
            &xlm.address,
            &10_0000000,
            &2_8864196,
            &30
        ),
        2_8864196
    );
}
