#![cfg(test)]
extern crate std;

use crate::testutils::{
    create_liqpool_contract,
    create_plane_contract,
    create_reward_boost_feed_contract,
    create_token_contract,
    get_token_admin_client,
    install_token_wasm,
    Setup,
    TestConfig,
};
use access_control::constants::ADMIN_ACTIONS_DELAY;
use soroban_sdk::log;
use core::cmp::min;
use soroban_sdk::testutils::{ AuthorizedFunction, AuthorizedInvocation, Events };
use soroban_sdk::token::{
    StellarAssetClient as SorobanTokenAdminClient,
    TokenClient as SorobanTokenClient,
};
use soroban_sdk::{
    symbol_short,
    testutils::Address as _,
    vec,
    Address,
    Env,
    Error,
    IntoVal,
    Symbol,
    Val,
    Vec,
};
use token_share::Client as ShareTokenClient;
use utils::test_utils::{ assert_approx_eq_abs, install_dummy_wasm, jump };

#[test]
fn test() {
    let setup = Setup::new_with_config(
        &(TestConfig {
            mint_to_user: i128::MAX,
            ..TestConfig::default()
        })
    );
    let e = setup.env;
    let liq_pool = setup.liq_pool;
    let token1 = setup.token1;
    let token2 = setup.token2;
    let token_reward = setup.token_reward;
    let token_share = setup.token_share;
    let user1 = setup.users[0].clone();
    let reward_1_tps = 10_5000000_u128;
    let reward_2_tps = 20_0000000_u128;
    let reward_3_tps = 6_0000000_u128;
    let total_reward_1 = reward_1_tps * 60;
    let amount_to_deposit = 100_0000000;
    let desired_amounts = Vec::from_array(&e, [amount_to_deposit, amount_to_deposit]);
    // assert_eq!(0, desired_amounts);

    liq_pool.deposit(&user1, &desired_amounts, &0);
    assert_eq!(e.auths()[0], (
        user1.clone(),
        AuthorizedInvocation {
            function: AuthorizedFunction::Contract((
                liq_pool.address.clone(),
                Symbol::new(&e, "deposit"),
                Vec::from_array(&e, [
                    user1.to_val(),
                    desired_amounts.to_val(),
                    (0_u128).into_val(&e),
                ]),
            )),
            sub_invocations: std::vec![
                AuthorizedInvocation {
                    function: AuthorizedFunction::Contract((
                        token1.address.clone(),
                        Symbol::new(&e, "transfer"),
                        Vec::from_array(&e, [
                            user1.to_val(),
                            liq_pool.address.to_val(),
                            (desired_amounts.get(0).unwrap() as i128).into_val(&e),
                        ]),
                    )),
                    sub_invocations: std::vec![],
                },
                AuthorizedInvocation {
                    function: AuthorizedFunction::Contract((
                        token2.address.clone(),
                        Symbol::new(&e, "transfer"),
                        Vec::from_array(&e, [
                            user1.to_val(),
                            liq_pool.address.to_val(),
                            (desired_amounts.get(1).unwrap() as i128).into_val(&e),
                        ]),
                    )),
                    sub_invocations: std::vec![],
                }
            ],
        },
    ));
}
