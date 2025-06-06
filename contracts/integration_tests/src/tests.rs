#![cfg(test)]
extern crate std;

use crate::testutils::{ create_token_contract, deploy_provider_swap_fee_contract, get_token_admin_client, Setup };
use soroban_sdk::testutils::Address as _;
use soroban_sdk::token::TokenClient;
use soroban_sdk::{ vec, Address, Vec };

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
    let nbtc = TokenClient::new(&setup.env, &tokens[0]);
    let xlm = TokenClient::new(&setup.env, &tokens[1]);

    let nbtc_admin = get_token_admin_client(&setup.env, &nbtc.address);
    let xlm_admin = get_token_admin_client(&setup.env, &xlm.address);

    // deploy pools
    let (pool, pool_hash) = setup.deploy_pool(
        &nbtc.address,
        &xlm.address,
        30
    );
    xlm_admin.mint(&setup.admin, &344_000_0000000);
    pool.deposit(&setup.admin, &100_000_0000000);

    // swap through many pools at once
    let user = Address::generate(&setup.env);
    xlm_admin.mint(&user, &10_0000000);

    let tokens = Vec::from_array(&setup.env, [nbtc.address.clone(), xlm.address.clone()]);
    let (pool_index, _pool_address) = setup.router.get_pools(&tokens).iter().last().unwrap();

    assert_eq!(
        setup.router.swap(
            &user,
            &tokens,
            &nbtc.address,
            &xlm.address,
            &pool_index,
            &10_0000000,
            &2_8952731
        ),
        2_8952731
    );

    // deploy provider swap fee contract
    let swap_fee = deploy_provider_swap_fee_contract(&setup.env, &setup.operator, &setup.admin, &setup.admin);

    // now swap with additional provider fee
    xlm_admin.mint(&user, &10_0000000);
    assert_eq!(
        swap_fee.swap(
            &user,
            &(
                vec![&setup.env, xlm.address.clone(), nbtc.address.clone()],
                pool_hash.clone(),
                nbtc.address.clone(),
            ),
            &xlm.address,
            &10_0000000,
            &2_8864196,
            &30
        ),
        2_8864196
    );
}
