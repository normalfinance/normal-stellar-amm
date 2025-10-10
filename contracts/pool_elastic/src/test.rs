#![cfg(test)]
extern crate std;

use crate::rewards::get_rewards_manager;
use crate::testutils::{
    create_liqpool_contract, create_plane_contract, create_token_contract, deploy_rewards_gauge,
    get_token_admin_client, install_token_wasm, setup_price_feed_oracle, Setup, TestConfig,
};
use access_control::constants::ADMIN_ACTIONS_DELAY;
use core::cmp::min;
use pool_config_storage::testutils::deploy_config_storage;
use rewards::storage::{PoolRewardsStorageTrait, UserRewardsStorageTrait};
use sep_40_oracle::testutils::{Asset as MockAsset, MockPriceOracleClient, MockPriceOracleWASM};
use soroban_sdk::testutils::{AuthorizedFunction, AuthorizedInvocation, Events};
use soroban_sdk::token::{
    StellarAssetClient as SorobanTokenAdminClient, TokenClient as SorobanTokenClient,
};
use soroban_sdk::BytesN;
use soroban_sdk::{
    symbol_short, testutils::Address as _, vec, Address, Env, Error, IntoVal, Map, Symbol, Val, Vec,
};
use token_share::Client as ShareTokenClient;
use utils::constant::{THIRTEEN_DAY, TWENTY_FOUR_HOUR};
use utils::test_utils::{assert_approx_eq_abs, install_dummy_wasm, jump};

#[test]
fn test() {
    let setup = Setup::new_with_config(&TestConfig {
        mint_to_user: i128::MAX,
        ..TestConfig::default()
    });
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

    liq_pool.deposit(&user1, &desired_amounts, &0);
    assert_eq!(
        e.auths()[0],
        (
            user1.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    liq_pool.address.clone(),
                    Symbol::new(&e, "deposit"),
                    Vec::from_array(
                        &e,
                        [
                            user1.to_val(),
                            desired_amounts.to_val(),
                            0_u128.into_val(&e)
                        ]
                    ),
                )),
                sub_invocations: std::vec![
                    AuthorizedInvocation {
                        function: AuthorizedFunction::Contract((
                            token1.address.clone(),
                            Symbol::new(&e, "transfer"),
                            Vec::from_array(
                                &e,
                                [
                                    user1.to_val(),
                                    liq_pool.address.to_val(),
                                    (desired_amounts.get(0).unwrap() as i128).into_val(&e),
                                ]
                            ),
                        )),
                        sub_invocations: std::vec![],
                    },
                    AuthorizedInvocation {
                        function: AuthorizedFunction::Contract((
                            token2.address.clone(),
                            Symbol::new(&e, "transfer"),
                            Vec::from_array(
                                &e,
                                [
                                    user1.to_val(),
                                    liq_pool.address.to_val(),
                                    (desired_amounts.get(1).unwrap() as i128).into_val(&e),
                                ]
                            ),
                        )),
                        sub_invocations: std::vec![],
                    }
                ],
            }
        )
    );

    assert_eq!(token_reward.balance(&user1), 0);
    // 30 seconds passed, half of the reward is available for the user
    jump(&e, 30);
    assert_eq!(liq_pool.claim(&user1), total_reward_1 / 2);
    assert_eq!(token_reward.balance(&user1) as u128, total_reward_1 / 2);
    // 60 seconds more passed. full reward was available though half already claimed
    jump(&e, 60);
    assert_eq!(liq_pool.claim(&user1), total_reward_1 / 2);
    assert_eq!(token_reward.balance(&user1) as u128, total_reward_1);

    // more rewards added with different configs
    let total_reward_2 = reward_2_tps * 100;
    liq_pool.set_rewards_config(
        &user1,
        &e.ledger().timestamp().saturating_add(100),
        &reward_2_tps,
    );
    jump(&e, 105);
    let total_reward_3 = reward_3_tps * 50;
    liq_pool.set_rewards_config(
        &user1,
        &e.ledger().timestamp().saturating_add(50),
        &reward_3_tps,
    );
    jump(&e, 500);
    // two rewards available for the user
    assert_eq!(liq_pool.claim(&user1), total_reward_2 + total_reward_3);
    assert_eq!(
        token_reward.balance(&user1) as u128,
        total_reward_1 + total_reward_2 + total_reward_3
    );

    // when we deposit equal amounts, we gotta have deposited amount of share tokens
    let expected_share_amount = amount_to_deposit as i128;
    assert_eq!(token_share.balance(&user1), expected_share_amount);
    assert_eq!(token_share.balance(&liq_pool.address), 0);
    assert_eq!(
        token1.balance(&user1),
        i128::MAX - amount_to_deposit as i128
    );
    assert_eq!(token1.balance(&liq_pool.address), amount_to_deposit as i128);
    assert_eq!(
        token2.balance(&user1),
        i128::MAX - amount_to_deposit as i128
    );
    assert_eq!(token2.balance(&liq_pool.address), amount_to_deposit as i128);

    let swap_in_amount = 1_0000000_u128;
    let expected_swap_result = 9871580_u128;

    assert_eq!(
        liq_pool.estimate_swap(&0, &1, &swap_in_amount, &false),
        expected_swap_result
    );
    assert_eq!(
        liq_pool.swap(&user1, &0, &1, &swap_in_amount, &expected_swap_result),
        expected_swap_result
    );
    assert_eq!(
        e.auths()[0],
        (
            user1.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    liq_pool.address.clone(),
                    Symbol::new(&e, "swap"),
                    (&user1, 0_u32, 1_u32, swap_in_amount, expected_swap_result).into_val(&e)
                )),
                sub_invocations: std::vec![AuthorizedInvocation {
                    function: AuthorizedFunction::Contract((
                        token1.address.clone(),
                        Symbol::new(&e, "transfer"),
                        Vec::from_array(
                            &e,
                            [
                                user1.to_val(),
                                liq_pool.address.to_val(),
                                (swap_in_amount as i128).into_val(&e),
                            ]
                        ),
                    )),
                    sub_invocations: std::vec![],
                },],
            }
        )
    );

    // claim fees
    assert_eq!(
        token1.balance(&liq_pool.address),
        amount_to_deposit as i128 + swap_in_amount as i128
    );
    let fee_destination = Address::generate(&e);
    let protocol_fee = liq_pool
        .claim_protocol_fees(&setup.system_fee_admin, &fee_destination)
        .get(0)
        .unwrap();
    assert!(protocol_fee > 0);
    assert_eq!(token1.balance(&fee_destination), protocol_fee as i128);
    assert_eq!(token2.balance(&fee_destination), 0);

    let swap_in_amount_minus_protocol_fee = swap_in_amount - protocol_fee;

    assert_eq!(
        token1.balance(&user1),
        i128::MAX - amount_to_deposit as i128 - swap_in_amount as i128
    );
    assert_eq!(
        token1.balance(&liq_pool.address),
        amount_to_deposit as i128 + swap_in_amount_minus_protocol_fee as i128
    );
    assert_eq!(
        token2.balance(&user1),
        i128::MAX - amount_to_deposit as i128 + expected_swap_result as i128
    );
    assert_eq!(
        token2.balance(&liq_pool.address),
        amount_to_deposit as i128 - expected_swap_result as i128
    );

    let withdraw_amounts = [
        amount_to_deposit + swap_in_amount_minus_protocol_fee,
        amount_to_deposit - expected_swap_result,
    ];
    liq_pool.withdraw(
        &user1,
        &(expected_share_amount as u128),
        &Vec::from_array(&e, withdraw_amounts),
    );
    assert_eq!(
        e.auths()[0],
        (
            user1.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    liq_pool.address.clone(),
                    Symbol::new(&e, "withdraw"),
                    Vec::from_array(
                        &e,
                        [
                            user1.clone().into_val(&e),
                            (expected_share_amount as u128).into_val(&e),
                            Vec::from_array(&e, withdraw_amounts).into_val(&e)
                        ],
                    )
                )),
                sub_invocations: std::vec![AuthorizedInvocation {
                    function: AuthorizedFunction::Contract((
                        token_share.address.clone(),
                        Symbol::new(&e, "burn"),
                        Vec::from_array(
                            &e,
                            [user1.to_val(), (amount_to_deposit as i128).into_val(&e),]
                        ),
                    )),
                    sub_invocations: std::vec![],
                }],
            }
        )
    );

    jump(&e, 600);
    assert_eq!(liq_pool.claim(&user1), 0);
    assert_eq!(
        token_reward.balance(&user1) as u128,
        total_reward_1 + total_reward_2 + total_reward_3
    );

    assert_eq!(token1.balance(&user1), i128::MAX - protocol_fee as i128);
    assert_eq!(token2.balance(&user1), i128::MAX);
    assert_eq!(token_share.balance(&user1), 0);
    assert_eq!(token1.balance(&liq_pool.address), 0);
    assert_eq!(token2.balance(&liq_pool.address), 0);
    assert_eq!(token_share.balance(&liq_pool.address), 0);
}

#[test]
fn test_strict_receive() {
    let setup = Setup::new_with_config(&TestConfig {
        mint_to_user: i128::MAX,
        ..TestConfig::default()
    });
    let user1 = setup.users[0].clone();
    let desired_amounts = Vec::from_array(&setup.env, [100_0000000, 100_0000000]);
    setup.liq_pool.deposit(&user1, &desired_amounts, &0);
    let swap_in_amount = 1_0000000_u128;
    let expected_swap_result = 9871580_u128;

    assert_eq!(
        setup
            .liq_pool
            .estimate_swap(&0, &1, &swap_in_amount, &false),
        expected_swap_result
    );
    assert_eq!(
        setup
            .liq_pool
            .estimate_swap_strict_receive(&0, &1, &expected_swap_result, &false),
        swap_in_amount
    );
    assert_eq!(
        setup
            .liq_pool
            .swap_strict_receive(&user1, &0, &1, &expected_swap_result, &swap_in_amount),
        swap_in_amount
    );
}

#[test]
fn test_strict_receive_over_max() {
    let setup = Setup::new_with_config(&TestConfig {
        mint_to_user: i128::MAX,
        ..TestConfig::default()
    });
    let user1 = setup.users[0].clone();
    let desired_amounts = Vec::from_array(&setup.env, [100_0000000, 100_0000000]);
    setup.liq_pool.deposit(&user1, &desired_amounts, &0);

    assert!(setup
        .liq_pool
        .try_estimate_swap_strict_receive(&0, &1, &100_0000000, &false)
        .is_err());
    assert!(setup
        .liq_pool
        .try_swap_strict_receive(&user1, &0, &1, &100_0000000, &100_0000000)
        .is_err());
    // technically we can buy 100_0000000 - 1, but we can't pay for it
    assert!(setup
        .liq_pool
        .try_estimate_swap_strict_receive(&0, &1, &99_9999999, &false)
        .is_ok());
    assert!(setup
        .liq_pool
        .try_swap_strict_receive(&user1, &0, &1, &99_9999999, &100_0000000)
        .is_err());
    // maximum we're able to buy is `reserve - 1`
    assert_eq!(
        setup
            .liq_pool
            .estimate_swap_strict_receive(&0, &1, &99_9999999, &false),
        100300902607_8234705,
    );
    assert_eq!(
        setup
            .liq_pool
            .swap_strict_receive(&user1, &0, &1, &99_9999999, &100300902607_8234705),
        100300902607_8234705
    );
}

#[test]
fn test_events() {
    let setup = Setup::new_with_config(&TestConfig {
        mint_to_user: i128::MAX,
        ..TestConfig::default()
    });
    let e = setup.env;
    let liq_pool = setup.liq_pool;
    let token1 = setup.token1;
    let token2 = setup.token2;
    let user1 = setup.users[0].clone();
    let amount_to_deposit = 100_0000000;
    let desired_amounts = Vec::from_array(&e, [amount_to_deposit, amount_to_deposit]);

    liq_pool.deposit(&user1, &desired_amounts, &0);
    let events = e.events().all();
    assert_eq!(
        events.slice(events.len() - 2..events.len()),
        vec![
            &e,
            (
                liq_pool.address.clone(),
                (
                    Symbol::new(&e, "deposit_liquidity"),
                    token1.address.clone(),
                    token2.address.clone()
                )
                    .into_val(&e),
                (
                    amount_to_deposit as i128,
                    amount_to_deposit as i128,
                    amount_to_deposit as i128
                )
                    .into_val(&e),
            ),
            (
                liq_pool.address.clone(),
                (Symbol::new(&e, "update_reserves"),).into_val(&e),
                vec![&e, amount_to_deposit as i128, amount_to_deposit as i128].to_val()
            ),
        ]
    );

    assert_eq!(liq_pool.swap(&user1, &0, &1, &100, &97), 98);
    let events = e.events().all();
    assert_eq!(
        events.slice(events.len() - 2..events.len()),
        vec![
            &e,
            (
                liq_pool.address.clone(),
                (
                    Symbol::new(&e, "trade"),
                    token1.address.clone(),
                    token2.address.clone(),
                    user1.clone()
                )
                    .into_val(&e),
                (100_i128, 98_i128, 1_i128).into_val(&e),
            ),
            (
                liq_pool.address.clone(),
                (Symbol::new(&e, "update_reserves"),).into_val(&e),
                vec![&e, 100_0000100_i128, 999999902_i128,].to_val()
            ),
        ]
    );

    let amounts_out = liq_pool.withdraw(&user1, &amount_to_deposit, &Vec::from_array(&e, [0, 0]));
    assert_eq!(amounts_out.get(0).unwrap(), 1000000100);
    assert_eq!(amounts_out.get(1).unwrap(), 999999902);
    let events = e.events().all();
    assert_eq!(
        events.slice(events.len() - 2..events.len()),
        vec![
            &e,
            (
                liq_pool.address.clone(),
                (
                    Symbol::new(&e, "withdraw_liquidity"),
                    token1.address.clone(),
                    token2.address.clone()
                )
                    .into_val(&e),
                (
                    amount_to_deposit as i128,
                    amounts_out.get(0).unwrap() as i128,
                    amounts_out.get(1).unwrap() as i128
                )
                    .into_val(&e),
            ),
            (
                liq_pool.address.clone(),
                (Symbol::new(&e, "update_reserves"),).into_val(&e),
                vec![&e, 0_i128, 0_i128].to_val()
            ),
        ]
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #2006)")]
fn test_deposit_min_mint() {
    let setup = Setup::new_with_config(&TestConfig {
        mint_to_user: i128::MAX,
        ..TestConfig::default()
    });
    let liq_pool = setup.liq_pool;
    let user1 = setup.users[0].clone();

    liq_pool.deposit(
        &user1,
        &Vec::from_array(
            &setup.env,
            [1_000_000_000_000_0000000, 1_000_000_000_000_0000000],
        ),
        &0,
    );
    liq_pool.deposit(&user1, &Vec::from_array(&setup.env, [1, 1]), &10);
}

#[test]
#[should_panic(expected = "Error(Contract, #2004)")]
fn test_zero_initial_deposit() {
    let setup = Setup::default();
    let user1 = setup.users[0].clone();
    setup
        .liq_pool
        .deposit(&user1, &Vec::from_array(&setup.env, [100, 0]), &0);
}

#[test]
fn test_zero_deposit_ok() {
    let setup = Setup::default();
    let liq_pool = setup.liq_pool;
    let user1 = setup.users[0].clone();
    liq_pool.deposit(&user1, &Vec::from_array(&setup.env, [100, 100]), &0);
    liq_pool.deposit(&user1, &Vec::from_array(&setup.env, [100, 0]), &0);
}

#[test]
#[should_panic(expected = "Error(Contract, #201)")]
fn initialize_already_initialized() {
    let setup = Setup::default();

    let users = Setup::generate_random_users(&setup.env, 3);
    let token1 = create_token_contract(&setup.env, &users[1]);
    let token2 = create_token_contract(&setup.env, &users[2]);

    setup.liq_pool.initialize(
        &users[0],
        &(
            users[0].clone(),
            users[0].clone(),
            users[0].clone(),
            users[0].clone(),
            Vec::from_array(&setup.env, [users[0].clone()]),
            users[0].clone(),
        ),
        &users[0],
        &setup.oracle_addr,
        &install_token_wasm(&setup.env),
        &Vec::from_array(&setup.env, [token1.address.clone(), token2.address.clone()]),
        &(10_u32, 5000_u32),
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #202)")]
fn initialize_already_initialized_plane() {
    let setup = Setup::default();
    setup.liq_pool.init_pools_plane(&setup.plane.address);
}

#[test]
#[should_panic(expected = "Error(Contract, #201)")]
fn initialize_already_initialized_config_storage() {
    let setup = Setup::default();
    setup
        .liq_pool
        .init_config_storage(&setup.admin, &setup.plane.address);
}

#[test]
fn test_custom_fee() {
    let config = TestConfig {
        mint_to_user: 1000000_0000000,
        ..TestConfig::default()
    };
    let setup = Setup::new_with_config(&config);

    // we're checking fraction against output for 1 token
    for fee_config in [
        (0, 9900990_u128),    // 0%
        (10, 9891187_u128),   // 0.1%
        (30, 9871580_u128),   // 0.3%
        (100, 9802950_u128),  // 1%
        (1000, 8919722_u128), // 10%
        (3000, 6951340_u128), // 30%
        (5000, 4975124_u128), // 50%
    ] {
        let liqpool = create_liqpool_contract(
            &setup.env,
            &Address::generate(&setup.env),
            &setup.users[0],
            &setup.oracle_addr,
            &install_token_wasm(&setup.env),
            &Vec::from_array(
                &setup.env,
                [setup.token1.address.clone(), setup.token2.address.clone()],
            ),
            &setup.token_reward.address,
            fee_config.0, // ten percent
            &setup.plane.address,
            &setup.config_storage.address,
        );
        liqpool.deposit(
            &setup.users[0],
            &Vec::from_array(&setup.env, [100_0000000, 100_0000000]),
            &0,
        );
        assert_eq!(
            liqpool.estimate_swap(&1, &0, &1_0000000, &false),
            fee_config.1
        );
        assert_eq!(
            liqpool.swap(&setup.users[0], &1, &0, &1_0000000, &0),
            fee_config.1
        );

        // full withdraw & deposit to reset pool reserves
        liqpool.withdraw(
            &setup.users[0],
            &(SorobanTokenClient::new(&setup.env, &liqpool.share_id()).balance(&setup.users[0])
                as u128),
            &Vec::from_array(&setup.env, [0, 0]),
        );
        liqpool.deposit(
            &setup.users[0],
            &Vec::from_array(&setup.env, [100_0000000, 100_0000000]),
            &0,
        );
        assert_eq!(
            liqpool.estimate_swap(&0, &1, &1_0000000, &false),
            fee_config.1
        ); // re-check swap result didn't change
        assert_eq!(
            liqpool.estimate_swap_strict_receive(&0, &1, &fee_config.1, &false),
            1_0000000
        );
        assert_eq!(
            liqpool.swap_strict_receive(&setup.users[0], &0, &1, &fee_config.1, &1_0000000),
            1_0000000
        );
    }
}

#[test]
fn test_simple_ongoing_reward() {
    let setup = Setup::default();
    let env = setup.env;
    let liq_pool = setup.liq_pool;
    let token_reward = setup.token_reward;
    let users = setup.users;
    let total_reward_1 = TestConfig::default().reward_tps * 60;
    assert_eq!(liq_pool.get_total_configured_reward(), total_reward_1);
    assert_eq!(liq_pool.get_total_accumulated_reward(), 0);
    assert_eq!(liq_pool.get_total_claimed_reward(), 0);

    // 10 seconds passed since config, user depositing
    jump(&env, 10);

    assert_eq!(
        liq_pool.get_total_configured_reward(),
        total_reward_1 - TestConfig::default().reward_tps * 10
    );
    assert_eq!(liq_pool.get_total_accumulated_reward(), 0);
    assert_eq!(liq_pool.get_total_claimed_reward(), 0);

    liq_pool.deposit(&users[0], &Vec::from_array(&env, [100, 100]), &0);

    assert_eq!(token_reward.balance(&users[0]), 0);
    // 30 seconds passed, half of the reward is available for the user
    jump(&env, 30);

    assert_eq!(
        liq_pool.get_total_configured_reward(),
        total_reward_1 - TestConfig::default().reward_tps * 10
    );
    assert_eq!(
        liq_pool.get_total_accumulated_reward(),
        TestConfig::default().reward_tps * 30
    );
    assert_eq!(liq_pool.get_total_claimed_reward(), 0);

    assert_eq!(liq_pool.claim(&users[0]), total_reward_1 / 2);
    assert_eq!(token_reward.balance(&users[0]) as u128, total_reward_1 / 2);

    assert_eq!(
        liq_pool.get_total_configured_reward(),
        total_reward_1 - TestConfig::default().reward_tps * 10
    );
    assert_eq!(
        liq_pool.get_total_accumulated_reward(),
        TestConfig::default().reward_tps * 30
    );
    assert_eq!(
        liq_pool.get_total_claimed_reward(),
        TestConfig::default().reward_tps * 30
    );

    // 40 seconds passed, reward config ended
    //  5/6 of the reward is available for the user since he has missed first 10 seconds
    jump(&env, 40);

    assert_eq!(
        liq_pool.get_total_configured_reward(),
        total_reward_1 - TestConfig::default().reward_tps * 10
    );
    assert_eq!(
        liq_pool.get_total_accumulated_reward(),
        total_reward_1 - TestConfig::default().reward_tps * 10
    );
    assert_eq!(
        liq_pool.get_total_claimed_reward(),
        TestConfig::default().reward_tps * 30
    );

    assert_eq!(liq_pool.claim(&users[0]), total_reward_1 * 2 / 6);
    assert_eq!(
        token_reward.balance(&users[0]) as u128,
        total_reward_1 * 5 / 6
    );

    assert_eq!(
        liq_pool.get_total_configured_reward(),
        total_reward_1 - TestConfig::default().reward_tps * 10
    );
    assert_eq!(
        liq_pool.get_total_accumulated_reward(),
        total_reward_1 - TestConfig::default().reward_tps * 10
    );
    assert_eq!(
        liq_pool.get_total_claimed_reward(),
        TestConfig::default().reward_tps * 50
    );
}

#[test]
fn test_estimate_ongoing_reward() {
    let setup = Setup::default();
    let env = setup.env;
    let liq_pool = setup.liq_pool;
    let token_reward = setup.token_reward;
    let users = setup.users;

    // 10 seconds passed since config, user depositing
    jump(&env, 10);
    liq_pool.deposit(&users[0], &Vec::from_array(&env, [100, 100]), &0);

    assert_eq!(token_reward.balance(&users[0]), 0);
    // 30 seconds passed, half of the reward is available for the user
    jump(&env, 30);
    let total_reward_1 = TestConfig::default().reward_tps * 60;
    assert_eq!(liq_pool.get_user_reward(&users[0]), total_reward_1 / 2);
    assert_eq!(token_reward.balance(&users[0]) as u128, 0);
}

#[test]
fn test_simple_reward() {
    let setup = Setup::setup(&TestConfig::default());
    setup.mint_tokens_for_users(TestConfig::default().mint_to_user);
    let env = setup.env;
    let liq_pool = setup.liq_pool;
    let token_reward = setup.token_reward;
    let users = setup.users;

    // 10 seconds. user depositing
    jump(&env, 10);
    liq_pool.deposit(&users[0], &Vec::from_array(&env, [100, 100]), &0);

    // 20 seconds. rewards set up for 60 seconds
    jump(&env, 10);
    let reward_1_tps = 10_5000000_u128;
    let total_reward_1 = reward_1_tps * 60;
    liq_pool.set_rewards_config(
        &users[0],
        &env.ledger().timestamp().saturating_add(60),
        &reward_1_tps,
    );

    // 90 seconds. rewards ended.
    jump(&env, 70);
    // calling set rewards config to checkpoint. should be removed
    liq_pool.set_rewards_config(
        &users[0],
        &env.ledger().timestamp().saturating_add(60),
        &0_u128,
    );

    // 100 seconds. user claim reward
    jump(&env, 10);
    assert_eq!(token_reward.balance(&users[0]), 0);
    // full reward should be available to the user
    assert_eq!(liq_pool.claim(&users[0]), total_reward_1);
    assert_eq!(token_reward.balance(&users[0]) as u128, total_reward_1);
}

#[test]
fn test_two_users_rewards() {
    let setup = Setup::default();
    let env = setup.env;
    let liq_pool = setup.liq_pool;
    let token_reward = setup.token_reward;
    let users = setup.users;

    let total_reward_1 = &TestConfig::default().reward_tps * 60;

    // two users make deposit for equal value. second after 30 seconds after rewards start,
    //  so it gets only 1/4 of total reward
    liq_pool.deposit(&users[0], &Vec::from_array(&env, [100, 100]), &0);
    jump(&env, 30);
    assert_eq!(liq_pool.claim(&users[0]), total_reward_1 / 2);
    liq_pool.deposit(&users[1], &Vec::from_array(&env, [100, 100]), &0);
    jump(&env, 100);
    assert_eq!(liq_pool.claim(&users[0]), total_reward_1 / 4);
    assert_eq!(liq_pool.claim(&users[1]), total_reward_1 / 4);
    assert_eq!(
        token_reward.balance(&users[0]) as u128,
        total_reward_1 / 4 * 3
    );
    assert_eq!(token_reward.balance(&users[1]) as u128, total_reward_1 / 4);
}

#[test]
fn test_lazy_user_rewards() {
    let setup = Setup::default();
    let env = setup.env;
    let liq_pool = setup.liq_pool;
    let token_reward = setup.token_reward;
    let users = setup.users;

    let total_reward_1 = &TestConfig::default().reward_tps * 60;

    liq_pool.deposit(&users[0], &Vec::from_array(&env, [100, 100]), &0);
    jump(&env, 59);
    liq_pool.deposit(&users[1], &Vec::from_array(&env, [1000, 1000]), &0);
    jump(&env, 100);
    let user1_claim = liq_pool.claim(&users[0]);
    let user2_claim = liq_pool.claim(&users[1]);
    assert_approx_eq_abs(
        user1_claim,
        total_reward_1 * 59 / 60 + total_reward_1 / 1100 * 100 / 60,
        1000,
    );
    assert_approx_eq_abs(user2_claim, total_reward_1 / 1100 * 1000 / 60, 1000);
    assert_approx_eq_abs(token_reward.balance(&users[0]) as u128, user1_claim, 1000);
    assert_approx_eq_abs(token_reward.balance(&users[1]) as u128, user2_claim, 1000);
    assert_approx_eq_abs(user1_claim + user2_claim, total_reward_1, 1000);
}

#[test]
fn test_rewards_disable_before_expiration() {
    let setup = Setup::new_with_config(&TestConfig {
        users_count: 3,
        reward_tps: 0,
        ..TestConfig::default()
    });
    let env = setup.env;
    let liq_pool = setup.liq_pool;
    let users = setup.users;

    // user 1 has 10% of total reward
    liq_pool.deposit(&users[0], &Vec::from_array(&env, [900, 900]), &0);
    liq_pool.deposit(&users[1], &Vec::from_array(&env, [100, 100]), &0);

    jump(&env, 10);
    let admin = users[0].clone();
    let tps = 1_0000000;
    // admin sets rewards distribution a bit in the future from the expected point
    liq_pool.set_rewards_config(&admin, &env.ledger().timestamp().saturating_add(100), &tps);

    // user 2 enters. now user 1 gets 5% of total reward, user 2 receives 50%
    jump(&env, 20);
    liq_pool.deposit(&users[2], &Vec::from_array(&env, [1000, 1000]), &0);

    jump(&env, 10);
    liq_pool.withdraw(&users[2], &1000, &Vec::from_array(&env, [1000, 1000]));

    // before config expiration, admin decides to stop as it's time to reward other pools
    jump(&env, 50);
    liq_pool.set_rewards_config(&admin, &env.ledger().timestamp().saturating_add(10), &0);

    // user decides to claim in far future
    jump(&env, 1000);
    assert_eq!(
        liq_pool.claim(&users[1]),
        tps * 20 / 10 + tps * 10 / 20 + tps * 50 / 10
    );
    assert_eq!(liq_pool.claim(&users[2]), tps * 10 / 2);
}

#[test]
fn test_rewards_disable_after_expiration() {
    let setup = Setup::new_with_config(&TestConfig {
        reward_tps: 0,
        ..TestConfig::default()
    });
    let env = setup.env;
    let liq_pool = setup.liq_pool;
    let users = setup.users;

    // user 1 has 10% of total reward
    liq_pool.deposit(&users[0], &Vec::from_array(&env, [900, 900]), &0);
    liq_pool.deposit(&users[1], &Vec::from_array(&env, [100, 100]), &0);

    jump(&env, 10);
    let admin = users[0].clone();
    let tps = 1_0000000;
    // admin sets rewards distribution, then decides to stop rewards after expiration
    liq_pool.set_rewards_config(&admin, &env.ledger().timestamp().saturating_add(100), &tps);
    jump(&env, 150);
    liq_pool.set_rewards_config(&admin, &env.ledger().timestamp().saturating_add(100), &0);

    // user decides to claim in far future
    jump(&env, 1000);
    assert_eq!(liq_pool.claim(&users[1]), tps * 100 / 10);
}

#[test]
fn test_rewards_set_new_after_expiration() {
    let setup = Setup::new_with_config(&TestConfig {
        reward_tps: 0,
        ..TestConfig::default()
    });
    let env = setup.env;
    let liq_pool = setup.liq_pool;
    let users = setup.users;

    // user 1 has 10% of total reward
    liq_pool.deposit(&users[0], &Vec::from_array(&env, [900, 900]), &0);
    liq_pool.deposit(&users[1], &Vec::from_array(&env, [100, 100]), &0);

    jump(&env, 10);
    let admin = users[0].clone();
    let tps_1 = 1_0000000;
    let tps_2 = 10000;
    // admin configures first rewards distribution, then it ends and admin sets new one which also expires
    liq_pool.set_rewards_config(
        &admin,
        &env.ledger().timestamp().saturating_add(100),
        &tps_1,
    );
    jump(&env, 150);
    liq_pool.set_rewards_config(
        &admin,
        &env.ledger().timestamp().saturating_add(100),
        &tps_2,
    );

    // user decides to claim in far future
    jump(&env, 1000);
    assert_eq!(
        liq_pool.claim(&users[1]),
        tps_1 * 100 / 10 + tps_2 * 100 / 10
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #702)")]
fn test_rewards_same_expiration_time() {
    let setup = Setup::default();
    let env = setup.env;
    let liq_pool = setup.liq_pool;
    let users = setup.users;

    jump(&env, 10);
    liq_pool.set_rewards_config(&users[0], &env.ledger().timestamp().saturating_add(100), &1);
    jump(&env, 10);
    liq_pool.set_rewards_config(&users[0], &env.ledger().timestamp().saturating_add(90), &2);
}

#[test]
#[should_panic(expected = "Error(Contract, #701)")]
fn test_rewards_past() {
    let setup = Setup::default();
    let env = setup.env;
    let liq_pool = setup.liq_pool;
    let users = setup.users;

    jump(&env, 10);
    let original_expiration_time = env.ledger().timestamp().saturating_add(100);
    liq_pool.set_rewards_config(&users[0], &original_expiration_time, &1);
    jump(&env, 1000);
    liq_pool.set_rewards_config(&users[0], &original_expiration_time.saturating_add(90), &2);
}

fn test_rewards_many_users(iterations_to_simulate: u32) {
    // first user comes as initial liquidity provider
    //  many users come
    //  user does withdraw

    let setup = Setup::new_with_config(&TestConfig {
        users_count: 100,
        ..TestConfig::default()
    });
    let env = setup.env;
    let liq_pool = setup.liq_pool;
    let users = setup.users;
    let token1_admin_client = setup.token1_admin_client;
    let token2_admin_client = setup.token2_admin_client;
    let token_reward_admin_client = setup.token_reward_admin_client;

    let admin = users[0].clone();
    let first_user = Address::generate(&env);

    for i in 0..101 {
        let user = match i {
            0 => &first_user,
            val => &users[val - 1],
        };
        token1_admin_client.mint(user, &1_000_000_000_000_000_000_000);
        token2_admin_client.mint(user, &1_000_000_000_000_000_000_000);
    }

    token_reward_admin_client.mint(&liq_pool.address, &1_000_000_000_000_0000000);

    let reward_1_tps = 10_5000000_u128;
    liq_pool.set_rewards_config(
        &admin,
        &env.ledger()
            .timestamp()
            .saturating_add((iterations_to_simulate * 2 + 110).into()),
        &reward_1_tps,
    );
    jump(&env, 10);

    // we have this because of last jump(100)
    let mut expected_reward = 100 * reward_1_tps / iterations_to_simulate as u128;
    for i in 0..iterations_to_simulate as u128 {
        expected_reward += reward_1_tps / (i + 1);
    }

    liq_pool.deposit(
        &first_user,
        &Vec::from_array(&env, [1_000_000_000_000_0000000, 1_000_000_000_000_0000000]),
        &0,
    );
    jump(&env, 1);

    for i in 1..iterations_to_simulate as usize {
        let user = &users[i % 10];
        liq_pool.deposit(
            user,
            &Vec::from_array(&env, [1_000_000_000_000_0000000, 1_000_000_000_000_0000000]),
            &0,
        );
        jump(&env, 1);
    }

    jump(&env, 100);
    env.cost_estimate().budget().reset_default();
    let user1_claim = liq_pool.claim(&first_user);
    env.cost_estimate().budget().print();
    assert_approx_eq_abs(user1_claim, expected_reward, 10000); // small loss because of rounding is fine
}

#[test]
fn test_deposit_inequal_return_change() {
    let setup = Setup::default();
    let e = setup.env;
    let liq_pool = setup.liq_pool;
    let token1 = setup.token1;
    let token2 = setup.token2;
    let users = setup.users;
    let user1 = users[0].clone();
    liq_pool.deposit(&user1, &Vec::from_array(&e, [100, 100]), &0);
    assert_eq!(token1.balance(&liq_pool.address), 100);
    assert_eq!(token2.balance(&liq_pool.address), 100);
    liq_pool.deposit(&user1, &Vec::from_array(&e, [200, 100]), &0);
    assert_eq!(token1.balance(&liq_pool.address), 200);
    assert_eq!(token2.balance(&liq_pool.address), 200);
}

#[test]
fn test_rewards_1k() {
    test_rewards_many_users(1_000);
}

#[cfg(feature = "slow_tests")]
#[test]
fn test_rewards_50k() {
    test_rewards_many_users(50_000);
}

#[test]
#[should_panic(expected = "Error(Contract, #102)")]
fn test_config_rewards_not_admin() {
    let setup = Setup::setup(&TestConfig::default());
    setup.mint_tokens_for_users(TestConfig::default().mint_to_user);
    let env = setup.env;
    let liq_pool = setup.liq_pool;
    let users = setup.users;

    liq_pool.set_rewards_config(
        &users[1],
        &env.ledger().timestamp().saturating_add(60),
        &10_5000000_u128,
    );
}

#[test]
fn test_config_rewards_router() {
    let setup = Setup::setup(&TestConfig::default());
    setup.mint_tokens_for_users(TestConfig::default().mint_to_user);
    let env = setup.env;
    let liq_pool = setup.liq_pool;
    let router = setup.router;

    liq_pool.set_rewards_config(
        &router,
        &env.ledger().timestamp().saturating_add(60),
        &10_5000000_u128,
    );
}

#[test]
fn test_config_rewards_override() {
    let setup = Setup::setup(&TestConfig::default());
    setup.mint_tokens_for_users(TestConfig::default().mint_to_user);
    let env = setup.env;
    let liq_pool = setup.liq_pool;
    let users = setup.users;
    let router = setup.router;

    liq_pool.deposit(&users[1], &Vec::from_array(&env, [100, 100]), &0);

    assert_eq!(liq_pool.get_total_accumulated_reward(), 0);
    assert_eq!(liq_pool.get_total_configured_reward(), 0);
    let tps = 10_5000000_u128;
    liq_pool.set_rewards_config(&router, &env.ledger().timestamp().saturating_add(60), &tps);

    jump(&env, 30);
    // assert_eq!(liq_pool.get_total_accumulated_reward(), tps * 30);
    // assert_eq!(liq_pool.get_total_configured_reward(), tps * 60);
    liq_pool.set_rewards_config(&router, &env.ledger().timestamp().saturating_add(0), &0);

    // assert_eq!(liq_pool.get_total_accumulated_reward(), tps * 30);
    // assert_eq!(liq_pool.get_total_configured_reward(), tps * 30);

    jump(&env, 5);

    assert_eq!(liq_pool.get_total_accumulated_reward(), tps * 30);
    assert_eq!(liq_pool.get_total_configured_reward(), tps * 30);
}

#[should_panic(expected = "Error(Contract, #2018)")]
#[test]
fn test_zero_swap() {
    let setup = Setup::new_with_config(&TestConfig {
        mint_to_user: i128::MAX,
        ..TestConfig::default()
    });
    let e = setup.env;
    let liq_pool = setup.liq_pool;
    let users = setup.users;
    let user1 = users[0].clone();
    let amount_to_deposit = 1_0000000;
    let desired_amounts = Vec::from_array(&e, [amount_to_deposit, amount_to_deposit]);

    liq_pool.deposit(&user1, &desired_amounts, &0);
    liq_pool.swap(&user1, &0, &1, &0, &0);
}

#[test]
fn test_large_numbers() {
    let setup = Setup::new_with_config(&TestConfig {
        mint_to_user: i128::MAX,
        ..TestConfig::default()
    });
    let e = setup.env;
    let liq_pool = setup.liq_pool;
    let token1 = setup.token1;
    let token2 = setup.token2;
    let token_share = setup.token_share;
    let users = setup.users;
    let user1 = users[0].clone();
    let amount_to_deposit = u128::MAX / 1_000_000;
    let desired_amounts = Vec::from_array(&e, [amount_to_deposit, amount_to_deposit]);

    liq_pool.deposit(&user1, &desired_amounts, &0);

    // when we deposit equal amounts, we gotta have deposited amount of share tokens
    let expected_share_amount = amount_to_deposit as i128;
    assert_eq!(token_share.balance(&user1), expected_share_amount);
    assert_eq!(token_share.balance(&liq_pool.address), 0);
    assert_eq!(
        token1.balance(&user1),
        i128::MAX - amount_to_deposit as i128
    );
    assert_eq!(token1.balance(&liq_pool.address), amount_to_deposit as i128);
    assert_eq!(
        token2.balance(&user1),
        i128::MAX - amount_to_deposit as i128
    );
    assert_eq!(token2.balance(&liq_pool.address), amount_to_deposit as i128);

    let swap_in = amount_to_deposit / 1_000;
    // swap out shouldn't differ for more than 0.4% since fee is 0.3%
    let expected_swap_result_delta = swap_in / 250;
    let estimate_swap_result = liq_pool.estimate_swap(&0, &1, &swap_in, &false);
    assert_approx_eq_abs(estimate_swap_result, swap_in, expected_swap_result_delta);
    assert_eq!(
        liq_pool.swap(&user1, &0, &1, &swap_in, &estimate_swap_result),
        estimate_swap_result
    );

    // claim fees
    assert_eq!(
        token1.balance(&liq_pool.address),
        amount_to_deposit as i128 + swap_in as i128
    );
    let protocol_fee = liq_pool
        .claim_protocol_fees(&setup.system_fee_admin, &setup.system_fee_admin)
        .get(0)
        .unwrap();
    assert!(protocol_fee > 0);
    let swap_in_amount_minus_protocol_fee = swap_in - protocol_fee;

    assert_eq!(
        token1.balance(&user1),
        i128::MAX - amount_to_deposit as i128 - swap_in as i128
    );
    assert_eq!(
        token1.balance(&liq_pool.address),
        amount_to_deposit as i128 + swap_in_amount_minus_protocol_fee as i128
    );
    assert_eq!(
        token2.balance(&user1),
        i128::MAX - amount_to_deposit as i128 + estimate_swap_result as i128
    );
    assert_eq!(
        token2.balance(&liq_pool.address),
        amount_to_deposit as i128 - estimate_swap_result as i128
    );

    let withdraw_amounts = [
        amount_to_deposit + swap_in_amount_minus_protocol_fee,
        amount_to_deposit - estimate_swap_result,
    ];
    liq_pool.withdraw(
        &user1,
        &(expected_share_amount as u128),
        &Vec::from_array(&e, withdraw_amounts),
    );

    assert_eq!(token1.balance(&user1), i128::MAX - protocol_fee as i128);
    assert_eq!(token2.balance(&user1), i128::MAX);
    assert_eq!(token_share.balance(&user1), 0);
    assert_eq!(token1.balance(&liq_pool.address), 0);
    assert_eq!(token2.balance(&liq_pool.address), 0);
    assert_eq!(token_share.balance(&liq_pool.address), 0);
}

#[test]
fn test_swap_killed() {
    let setup = Setup::new_with_config(&TestConfig {
        mint_to_user: i128::MAX,
        ..TestConfig::default()
    });
    let e = setup.env;
    let liq_pool = setup.liq_pool;
    let users = setup.users;

    assert_eq!(liq_pool.get_is_killed_deposit(), false);
    assert_eq!(liq_pool.get_is_killed_swap(), false);
    assert_eq!(liq_pool.get_is_killed_claim(), false);

    let admin = users[0].clone();

    liq_pool.kill_swap(&admin);
    assert_eq!(
        vec![&e, e.events().all().last().unwrap()],
        vec![
            &e,
            (
                liq_pool.address.clone(),
                (Symbol::new(&e, "kill_swap"),).into_val(&e),
                Val::VOID.into_val(&e),
            )
        ]
    );
    assert_eq!(liq_pool.get_is_killed_deposit(), false);
    assert_eq!(liq_pool.get_is_killed_swap(), true);
    assert_eq!(liq_pool.get_is_killed_claim(), false);

    let user1 = users[1].clone();
    let amount_to_deposit = 1_0000000;
    let desired_amounts = Vec::from_array(&e, [amount_to_deposit, amount_to_deposit]);

    liq_pool.deposit(&user1, &desired_amounts, &0);

    assert_eq!(
        liq_pool.try_swap(&user1, &0, &1, &100, &0).unwrap_err(),
        Ok(Error::from_contract_error(206))
    );

    liq_pool.unkill_swap(&admin);
    assert_eq!(
        vec![&e, e.events().all().last().unwrap()],
        vec![
            &e,
            (
                liq_pool.address.clone(),
                (Symbol::new(&e, "unkill_swap"),).into_val(&e),
                Val::VOID.into_val(&e),
            )
        ]
    );
    assert_eq!(liq_pool.get_is_killed_deposit(), false);
    assert_eq!(liq_pool.get_is_killed_swap(), false);
    assert_eq!(liq_pool.get_is_killed_claim(), false);

    liq_pool.swap(&user1, &0, &1, &100, &0);
}

#[test]
fn test_deposit_killed() {
    let setup = Setup::new_with_config(&TestConfig {
        mint_to_user: i128::MAX,
        ..TestConfig::default()
    });
    let e = setup.env;
    let liq_pool = setup.liq_pool;
    let users = setup.users;

    assert_eq!(liq_pool.get_is_killed_deposit(), false);
    assert_eq!(liq_pool.get_is_killed_swap(), false);
    assert_eq!(liq_pool.get_is_killed_claim(), false);

    let admin = users[0].clone();

    liq_pool.kill_deposit(&admin);
    assert_eq!(
        vec![&e, e.events().all().last().unwrap()],
        vec![
            &e,
            (
                liq_pool.address.clone(),
                (Symbol::new(&e, "kill_deposit"),).into_val(&e),
                Val::VOID.into_val(&e),
            )
        ]
    );
    assert_eq!(liq_pool.get_is_killed_deposit(), true);
    assert_eq!(liq_pool.get_is_killed_swap(), false);
    assert_eq!(liq_pool.get_is_killed_claim(), false);

    let user1 = users[1].clone();
    let amount_to_deposit = 1_0000000;
    let desired_amounts = Vec::from_array(&e, [amount_to_deposit, amount_to_deposit]);

    assert_eq!(
        liq_pool
            .try_deposit(&user1, &desired_amounts, &0)
            .unwrap_err(),
        Ok(Error::from_contract_error(205))
    );

    liq_pool.unkill_deposit(&admin);
    assert_eq!(
        vec![&e, e.events().all().last().unwrap()],
        vec![
            &e,
            (
                liq_pool.address.clone(),
                (Symbol::new(&e, "unkill_deposit"),).into_val(&e),
                Val::VOID.into_val(&e),
            )
        ]
    );
    assert_eq!(liq_pool.get_is_killed_deposit(), false);
    assert_eq!(liq_pool.get_is_killed_swap(), false);
    assert_eq!(liq_pool.get_is_killed_claim(), false);

    liq_pool.deposit(&user1, &desired_amounts, &0);
}

#[test]
fn test_claim_killed() {
    let setup = Setup::setup(&TestConfig::default());
    setup.mint_tokens_for_users(TestConfig::default().mint_to_user);
    let env = setup.env;
    let liq_pool = setup.liq_pool;
    let users = setup.users;
    assert_eq!(liq_pool.get_is_killed_deposit(), false);
    assert_eq!(liq_pool.get_is_killed_swap(), false);
    assert_eq!(liq_pool.get_is_killed_claim(), false);

    liq_pool.kill_claim(&users[0]);
    assert_eq!(
        vec![&env, env.events().all().last().unwrap()],
        vec![
            &env,
            (
                liq_pool.address.clone(),
                (Symbol::new(&env, "kill_claim"),).into_val(&env),
                Val::VOID.into_val(&env),
            )
        ]
    );
    assert_eq!(liq_pool.get_is_killed_deposit(), false);
    assert_eq!(liq_pool.get_is_killed_swap(), false);
    assert_eq!(liq_pool.get_is_killed_claim(), true);

    // 10 seconds. user depositing
    jump(&env, 10);
    liq_pool.deposit(&users[1], &Vec::from_array(&env, [100, 100]), &0);

    // 20 seconds. rewards set up for 60 seconds
    jump(&env, 10);
    let reward_1_tps = 10_5000000_u128;
    let total_reward_1 = reward_1_tps * 60;
    liq_pool.set_rewards_config(
        &users[0],
        &env.ledger().timestamp().saturating_add(60),
        &reward_1_tps,
    );

    // 90 seconds. rewards ended.
    jump(&env, 70);

    // 100 seconds. user claim reward
    jump(&env, 10);

    assert_eq!(
        liq_pool.try_claim(&users[1]).unwrap_err(),
        Ok(Error::from_contract_error(207))
    );
    liq_pool.unkill_claim(&users[0]);
    assert_eq!(
        vec![&env, env.events().all().last().unwrap()],
        vec![
            &env,
            (
                liq_pool.address.clone(),
                (Symbol::new(&env, "unkill_claim"),).into_val(&env),
                Val::VOID.into_val(&env),
            )
        ]
    );
    assert_eq!(liq_pool.get_is_killed_deposit(), false);
    assert_eq!(liq_pool.get_is_killed_swap(), false);
    assert_eq!(liq_pool.get_is_killed_claim(), false);
    assert_eq!(liq_pool.claim(&users[1]), total_reward_1);
}

#[test]
fn test_withdraw_rewards() {
    // test user cannot withdraw reward tokens from the pool
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);

    let mut token1 = create_token_contract(&e, &admin);
    let mut token2 = create_token_contract(&e, &admin);

    let plane = create_plane_contract(&e);

    if &token2.address < &token1.address {
        std::mem::swap(&mut token1, &mut token2);
    }
    let token1_admin_client = get_token_admin_client(&e, &token1.address);
    let token2_admin_client = get_token_admin_client(&e, &token2.address);
    let token_reward_admin_client = SorobanTokenAdminClient::new(&e, &token1.address.clone());

    let router = Address::generate(&e);

    // oracle
    let sol_asset = MockAsset::Other(Symbol::new(&e, "SOL"));
    let usdc_asset = MockAsset::Other(Symbol::new(&e, "USDC"));

    let (oracle_addr, oracle_client) = setup_price_feed_oracle(
        &e,
        &admin,
        &MockAsset::Other(Symbol::new(&e, "USD")),
        &Vec::from_array(&e, [sol_asset.clone(), usdc_asset.clone()]),
        14,
        300,
    );

    let prices_1: Vec<i128> = Vec::from_array(&e, [230_00000000000000, 1_00000000000000]);
    oracle_client.set_price(&prices_1, &1755271154);

    let liq_pool = create_liqpool_contract(
        &e,
        &admin,
        &router,
        &oracle_addr,
        &install_token_wasm(&e),
        &Vec::from_array(&e, [token1.address.clone(), token2.address.clone()]),
        &token_reward_admin_client.address,
        30,
        &plane.address,
        &deploy_config_storage(&e, &admin, &admin).address,
    );
    let token_share = ShareTokenClient::new(&e, &liq_pool.share_id());

    token1_admin_client.mint(&user1, &100_0000000);
    token2_admin_client.mint(&user1, &100_0000000);
    liq_pool.deposit(&user1, &Vec::from_array(&e, [100_0000000, 100_0000000]), &0);
    assert_eq!(
        liq_pool.get_reserves(),
        Vec::from_array(&e, [100_0000000, 100_0000000])
    );

    liq_pool.set_rewards_config(
        &admin,
        &e.ledger().timestamp().saturating_add(100),
        &1_000_0000000,
    );
    token_reward_admin_client.mint(&liq_pool.address, &(1_000_0000000 * 100));
    jump(&e, 100);

    token1_admin_client.mint(&user2, &1_000_0000000);
    token2_admin_client.mint(&user2, &1_000_0000000);
    liq_pool.deposit(
        &user2,
        &Vec::from_array(&e, [1_000_0000000, 1_000_0000000]),
        &0,
    );
    assert_eq!(
        liq_pool.get_reserves(),
        Vec::from_array(&e, [1_100_0000000, 1_100_0000000])
    );

    assert_eq!(
        liq_pool.get_reserves(),
        Vec::from_array(&e, [1_100_0000000, 1_100_0000000])
    );
    assert_eq!(
        token1.balance(&liq_pool.address),
        1_100_0000000 + 1_000_0000000 * 100
    );
    assert_eq!(token2.balance(&liq_pool.address), 1_100_0000000);

    assert_eq!(
        liq_pool.withdraw(
            &user2,
            &(token_share.balance(&user2) as u128),
            &Vec::from_array(&e, [0, 0]),
        ),
        Vec::from_array(&e, [1_000_0000000, 1_000_0000000])
    );
    assert_eq!(
        liq_pool.get_reserves(),
        Vec::from_array(&e, [100_0000000, 100_0000000])
    );
    assert_eq!(
        token1.balance(&liq_pool.address),
        100_0000000 + 1_000_0000000 * 100
    );
    assert_eq!(token2.balance(&liq_pool.address), 100_0000000);
    assert_eq!(token1.balance(&user2), 1_000_0000000);
    assert_eq!(token2.balance(&user2), 1_000_0000000);

    assert_eq!(liq_pool.claim(&user1), 1_000_0000000 * 100);
    assert_eq!(liq_pool.claim(&user2), 0);
}

#[test]
fn test_deposit_rewards() {
    // test pool reserves are not affected by rewards if reward token is one of pool tokens and presented in pool balance
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let user1 = Address::generate(&e);

    let mut token1 = create_token_contract(&e, &admin);
    let mut token2 = create_token_contract(&e, &admin);

    let plane = create_plane_contract(&e);

    if &token2.address < &token1.address {
        std::mem::swap(&mut token1, &mut token2);
    }
    let token1_admin_client = get_token_admin_client(&e, &token1.address);
    let token2_admin_client = get_token_admin_client(&e, &token2.address);
    let token_reward_admin_client = SorobanTokenAdminClient::new(&e, &token1.address.clone());

    let router = Address::generate(&e);

    // oracle
    let sol_asset = MockAsset::Other(Symbol::new(&e, "SOL"));
    let usdc_asset = MockAsset::Other(Symbol::new(&e, "USDC"));

    let (oracle_addr, oracle_client) = setup_price_feed_oracle(
        &e,
        &admin,
        &MockAsset::Other(Symbol::new(&e, "USD")),
        &Vec::from_array(&e, [sol_asset.clone(), usdc_asset.clone()]),
        14,
        300,
    );

    let prices_1: Vec<i128> = Vec::from_array(&e, [230_00000000000000, 1_00000000000000]);
    oracle_client.set_price(&prices_1, &1755271154);

    let liq_pool = create_liqpool_contract(
        &e,
        &admin,
        &router,
        &oracle_addr,
        &install_token_wasm(&e),
        &Vec::from_array(&e, [token1.address.clone(), token2.address.clone()]),
        &token_reward_admin_client.address,
        30,
        &plane.address,
        &deploy_config_storage(&e, &admin, &admin).address,
    );

    liq_pool.set_rewards_config(
        &admin,
        &e.ledger().timestamp().saturating_add(100),
        &1_000_0000000,
    );
    token_reward_admin_client.mint(&liq_pool.address, &(1_000_0000000 * 100));
    assert_eq!(liq_pool.get_reserves(), Vec::from_array(&e, [0, 0]));

    token1_admin_client.mint(&user1, &1000_0000000);
    token2_admin_client.mint(&user1, &1000_0000000);
    liq_pool.deposit(&user1, &Vec::from_array(&e, [1_0000000, 100_0000000]), &0);
    assert_eq!(
        liq_pool.get_reserves(),
        Vec::from_array(&e, [1_0000000, 100_0000000])
    );
    liq_pool.deposit(&user1, &Vec::from_array(&e, [1_0000000, 100_0000000]), &0);
    assert_eq!(
        liq_pool.get_reserves(),
        Vec::from_array(&e, [2_0000000, 200_0000000])
    );
}

#[test]
fn test_swap_rewards() {
    // check that swap rewards are calculated correctly if reward token is one of pool tokens
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);

    let mut token1 = create_token_contract(&e, &admin);
    let mut token2 = create_token_contract(&e, &admin);

    let plane = create_plane_contract(&e);

    if &token2.address < &token1.address {
        std::mem::swap(&mut token1, &mut token2);
    }
    let token1_admin_client = get_token_admin_client(&e, &token1.address);
    let token2_admin_client = get_token_admin_client(&e, &token2.address);
    let token_reward_admin_client = SorobanTokenAdminClient::new(&e, &token1.address.clone());

    let router = Address::generate(&e);

    // oracle
    let sol_asset = MockAsset::Other(Symbol::new(&e, "SOL"));
    let usdc_asset = MockAsset::Other(Symbol::new(&e, "USDC"));

    let (oracle_addr, oracle_client) = setup_price_feed_oracle(
        &e,
        &admin,
        &MockAsset::Other(Symbol::new(&e, "USD")),
        &Vec::from_array(&e, [sol_asset.clone(), usdc_asset.clone()]),
        14,
        300,
    );

    let prices_1: Vec<i128> = Vec::from_array(&e, [230_00000000000000, 1_00000000000000]);
    oracle_client.set_price(&prices_1, &1755271154);

    // we compare two pools to check swap in both directions
    let liq_pool1 = create_liqpool_contract(
        &e,
        &admin,
        &router,
        &oracle_addr,
        &install_token_wasm(&e),
        &Vec::from_array(&e, [token1.address.clone(), token2.address.clone()]),
        &token_reward_admin_client.address,
        30,
        &plane.address,
        &deploy_config_storage(&e, &admin, &admin).address,
    );
    let liq_pool2 = create_liqpool_contract(
        &e,
        &admin,
        &router,
        &oracle_addr,
        &install_token_wasm(&e),
        &Vec::from_array(&e, [token1.address.clone(), token2.address.clone()]),
        &token_reward_admin_client.address,
        30,
        &plane.address,
        &deploy_config_storage(&e, &admin, &admin).address,
    );
    token1_admin_client.mint(&user1, &200_0000000);
    token2_admin_client.mint(&user1, &200_0000000);
    liq_pool1.deposit(&user1, &Vec::from_array(&e, [100_0000000, 100_0000000]), &0);
    liq_pool2.deposit(&user1, &Vec::from_array(&e, [100_0000000, 100_0000000]), &0);
    assert_eq!(
        liq_pool1.get_reserves(),
        Vec::from_array(&e, [100_0000000, 100_0000000])
    );
    assert_eq!(
        liq_pool2.get_reserves(),
        Vec::from_array(&e, [100_0000000, 100_0000000])
    );

    let estimate1_before_rewards = liq_pool1.estimate_swap(&0, &1, &10_0000000, &false);
    let estimate2_before_rewards = liq_pool1.estimate_swap(&1, &0, &10_0000000, &false);
    // swap is balanced, so values should be the same
    assert_eq!(estimate1_before_rewards, estimate2_before_rewards);

    liq_pool1.set_rewards_config(
        &admin,
        &e.ledger().timestamp().saturating_add(100),
        &1_000_0000000,
    );
    liq_pool2.set_rewards_config(
        &admin,
        &e.ledger().timestamp().saturating_add(100),
        &1_000_0000000,
    );
    token_reward_admin_client.mint(&liq_pool1.address, &(1_000_0000000 * 100));
    token_reward_admin_client.mint(&liq_pool2.address, &(1_000_0000000 * 100));
    jump(&e, 100);

    let estimate1_after_rewards = liq_pool1.estimate_swap(&0, &1, &10_0000000, &false);
    let estimate2_after_rewards = liq_pool1.estimate_swap(&1, &0, &10_0000000, &false);
    // balances are out of balance, but reserves are balanced.
    assert_eq!(estimate1_after_rewards, estimate2_after_rewards);
    assert_eq!(estimate1_before_rewards, estimate1_after_rewards);

    token1_admin_client.mint(&user2, &10_0000000);
    token2_admin_client.mint(&user2, &10_0000000);
    // in case of disbalance, user may receive much more tokens than he sent as reward is included
    let swap_result1 = liq_pool1.swap(&user2, &0, &1, &10_0000000, &estimate1_after_rewards);
    let swap_result2 = liq_pool2.swap(&user2, &1, &0, &10_0000000, &estimate1_after_rewards);
    assert_eq!(swap_result1, estimate1_after_rewards);
    assert_eq!(swap_result2, estimate1_after_rewards);

    // claim admin fee to avoid issues with reserves
    assert_eq!(
        liq_pool1.claim_protocol_fees(&admin, &admin),
        Vec::from_array(&e, [150000, 0])
    );

    let reserves1 = liq_pool1.get_reserves();

    // check that balance minus rewards is equal to reserves as they should also have fee and it's same for both pools but in different order
    assert_eq!(
        liq_pool1.get_reserves(),
        Vec::from_array(
            &e,
            [
                token1.balance(&liq_pool1.address) as u128 - 1_000_0000000 * 100,
                token2.balance(&liq_pool1.address) as u128
            ]
        )
    );
    // reverse pool1 reserves to check swap in other direction gave same results
    assert_eq!(
        liq_pool2.get_reserves(),
        Vec::from_array(&e, [reserves1.get(1).unwrap(), reserves1.get(0).unwrap()])
    );
}

#[test]
fn test_claim_rewards() {
    // test user cannot claim from pool if rewards configured but not distributed
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let user1 = Address::generate(&e);

    let mut token1 = create_token_contract(&e, &admin);
    let mut token2 = create_token_contract(&e, &admin);

    let plane = create_plane_contract(&e);

    if &token2.address < &token1.address {
        std::mem::swap(&mut token1, &mut token2);
    }
    let token1_admin_client = get_token_admin_client(&e, &token1.address);
    let token2_admin_client = get_token_admin_client(&e, &token2.address);
    let token_reward_admin_client = SorobanTokenAdminClient::new(&e, &token1.address.clone());

    let router = Address::generate(&e);

    // oracle
    let sol_asset = MockAsset::Other(Symbol::new(&e, "SOL"));
    let usdc_asset = MockAsset::Other(Symbol::new(&e, "USDC"));

    let (oracle_addr, oracle_client) = setup_price_feed_oracle(
        &e,
        &admin,
        &MockAsset::Other(Symbol::new(&e, "USD")),
        &Vec::from_array(&e, [sol_asset.clone(), usdc_asset.clone()]),
        14,
        300,
    );

    let prices_1: Vec<i128> = Vec::from_array(&e, [230_00000000000000, 1_00000000000000]);
    oracle_client.set_price(&prices_1, &1755271154);

    let liq_pool = create_liqpool_contract(
        &e,
        &admin,
        &router,
        &oracle_addr,
        &install_token_wasm(&e),
        &Vec::from_array(&e, [token1.address.clone(), token2.address.clone()]),
        &token_reward_admin_client.address,
        30,
        &plane.address,
        &deploy_config_storage(&e, &admin, &admin).address,
    );

    token1_admin_client.mint(&user1, &100_0000000);
    token2_admin_client.mint(&user1, &100_0000000);
    liq_pool.deposit(&user1, &Vec::from_array(&e, [100_0000000, 100_0000000]), &0);
    assert_eq!(
        liq_pool.get_reserves(),
        Vec::from_array(&e, [100_0000000, 100_0000000])
    );

    liq_pool.set_rewards_config(&admin, &e.ledger().timestamp().saturating_add(100), &1000);
    jump(&e, 100);

    assert!(liq_pool.try_claim(&user1).is_err());
    token_reward_admin_client.mint(&liq_pool.address, &(1000 * 100));
    assert_eq!(liq_pool.claim(&user1), 1000 * 100);
}

#[test]
fn test_drain_reward() {
    let setup = Setup::new_with_config(&TestConfig {
        users_count: 5,
        reward_tps: 10_5000000,
        rewards_count: 10_5000000 * 60,
        mint_to_user: 1000_0000000,
        ..TestConfig::default()
    });
    let env = setup.env;
    let liq_pool = setup.liq_pool;
    let token_share = setup.token_share;
    let users = setup.users;

    // 10 seconds passed since config, user depositing
    jump(&env, 10);

    liq_pool.deposit(
        &users[0],
        &Vec::from_array(&env, [1000_0000000, 1000_0000000]),
        &0,
    );
    let (_, lp_amount) = liq_pool.deposit(
        &users[1],
        &Vec::from_array(&env, [100_0000000, 100_0000000]),
        &0,
    );

    jump(&env, 10);

    for i in 2..5 {
        token_share.transfer(&users[i - 1], &users[i], &(lp_amount as i128));
        // liq_pool.get_user_reward(&users[i]);
        // liq_pool.claim(&users[i]);
        liq_pool.deposit(&users[i], &Vec::from_array(&env, [1, 1]), &0);
    }

    jump(&env, 50);
    assert_eq!(liq_pool.claim(&users[4]), 381818182);
    token_share.transfer(&users[4], &users[3], &(lp_amount as i128));
    assert_eq!(liq_pool.claim(&users[3]), 0);
    token_share.transfer(&users[3], &users[2], &(lp_amount as i128));
    assert_eq!(liq_pool.claim(&users[2]), 0);
    token_share.transfer(&users[2], &users[1], &(lp_amount as i128));
    assert_eq!(liq_pool.claim(&users[1]), 95454545);
    assert_eq!(liq_pool.claim(&users[0]), 4772727271);
}

#[test]
fn test_drain_reserves() {
    // test pool reserves are not affected by rewards if reward token is one of pool tokens and presented in pool balance
    let e = Env::default();
    e.mock_all_auths();
    e.cost_estimate().budget().reset_unlimited();

    let admin = Address::generate(&e);
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);
    let user3 = Address::generate(&e);
    let user4 = Address::generate(&e);

    let mut token1 = create_token_contract(&e, &admin);
    let mut token2 = create_token_contract(&e, &admin);

    let plane = create_plane_contract(&e);

    if &token2.address < &token1.address {
        std::mem::swap(&mut token1, &mut token2);
    }
    let token1_admin_client = get_token_admin_client(&e, &token1.address);
    let token2_admin_client = get_token_admin_client(&e, &token2.address);
    let token_reward_admin_client = SorobanTokenAdminClient::new(&e, &token1.address.clone());

    let router = Address::generate(&e);

    // oracle
    let sol_asset = MockAsset::Other(Symbol::new(&e, "SOL"));
    let usdc_asset = MockAsset::Other(Symbol::new(&e, "USDC"));

    let (oracle_addr, oracle_client) = setup_price_feed_oracle(
        &e,
        &admin,
        &MockAsset::Other(Symbol::new(&e, "USD")),
        &Vec::from_array(&e, [sol_asset.clone(), usdc_asset.clone()]),
        14,
        300,
    );

    let prices_1: Vec<i128> = Vec::from_array(&e, [230_00000000000000, 1_00000000000000]);
    oracle_client.set_price(&prices_1, &1755271154);

    let liq_pool = create_liqpool_contract(
        &e,
        &admin,
        &router,
        &oracle_addr,
        &install_token_wasm(&e),
        &Vec::from_array(&e, [token1.address.clone(), token2.address.clone()]),
        &token_reward_admin_client.address,
        30,
        &plane.address,
        &deploy_config_storage(&e, &admin, &admin).address,
    );

    liq_pool.set_rewards_config(
        &admin,
        &e.ledger().timestamp().saturating_add(100),
        &1_000_0000000,
    );
    token_reward_admin_client.mint(&liq_pool.address, &(1_000_0000000 * 100));
    assert_eq!(liq_pool.get_reserves(), Vec::from_array(&e, [0, 0]));

    // first user deposits
    token1_admin_client.mint(&user1, &1_000_000_0000000);
    token2_admin_client.mint(&user1, &1_000_000_0000000);
    liq_pool.deposit(
        &user1,
        &Vec::from_array(&e, [1_000_000_0000000, 1_000_000_0000000]),
        &0,
    );

    // first exploiter deposits
    token1_admin_client.mint(&user2, &1_000_000_0000000);
    token2_admin_client.mint(&user2, &1_000_000_0000000);
    let (_, lp_amount) = liq_pool.deposit(
        &user2,
        &Vec::from_array(&e, [300_000_0000000, 300_000_0000000]),
        &0,
    );

    let token_share = SorobanTokenClient::new(&e, &liq_pool.share_id());

    token_share.transfer(&user2, &user3, &(lp_amount as i128));
    liq_pool.claim(&user3);
    token_share.transfer(&user3, &user4, &(lp_amount as i128));
    liq_pool.claim(&user4);

    jump(&e, 100);

    // exploit starts
    assert_eq!(liq_pool.claim(&user4), 230769230769);
    token_share.transfer(&user4, &user3, &(lp_amount as i128));
    assert_eq!(liq_pool.claim(&user3), 0);
    token_share.transfer(&user3, &user2, &(lp_amount as i128));
    assert_eq!(liq_pool.claim(&user2), 0);

    // first user claims
    assert_eq!(liq_pool.claim(&user1), 769230769230);

    // check reserves
    assert_eq!(
        liq_pool.get_reserves(),
        Vec::from_array(&e, [1_300_000_0000000, 1_300_000_0000000])
    );
    assert_eq!(token1.balance(&liq_pool.address), 1_300_000_0000001); // 1 token left on balance because of rounding
    assert_eq!(token2.balance(&liq_pool.address), 1_300_000_0000000);
}

#[test]
fn test_return_unused_reward() {
    let setup = Setup::new_with_config(&TestConfig {
        reward_tps: 0,
        reward_token_in_pool: false,
        mint_to_user: 0,
        rewards_count: 0,
        ..TestConfig::default()
    });
    assert_ne!(setup.token1.address, setup.token_reward.address);
    let e = setup.env;
    let admin = setup.admin;
    let liq_pool = setup.liq_pool;
    let router = setup.router;
    let token_1_admin_client = SorobanTokenAdminClient::new(&e, &setup.token1.address.clone());
    let token_2_admin_client = SorobanTokenAdminClient::new(&e, &setup.token2.address.clone());
    let token_reward_admin_client =
        SorobanTokenAdminClient::new(&e, &setup.token_reward.address.clone());
    let user = Address::generate(&e);

    token_1_admin_client.mint(&user, &1000_0000000);
    token_2_admin_client.mint(&user, &1000_0000000);
    liq_pool.deposit(
        &user,
        &Vec::from_array(&e, [1000_0000000, 1000_0000000]),
        &0,
    );

    liq_pool.set_rewards_config(
        &admin,
        &e.ledger().timestamp().saturating_add(60),
        &1_0000000,
    );
    // pool has configured rewards, but not minted
    assert_eq!(liq_pool.get_unused_reward(), 0);

    token_reward_admin_client.mint(&liq_pool.address, &(1_0000000 * 100));

    // we've configured rewards for 60 seconds, but minted for 100. 40 surplus
    assert_eq!(liq_pool.get_unused_reward(), 1_0000000 * 40);

    // 10 seconds passed
    jump(&e, 10);
    liq_pool.claim(&user);

    assert_eq!(liq_pool.get_unused_reward(), 1_0000000 * 40);
    assert_eq!(setup.token_reward.balance(&router), 0);
    jump(&e, 10);

    // pool stops rewards on new iteration
    liq_pool.set_rewards_config(&admin, &e.ledger().timestamp().saturating_add(0), &0);
    assert_eq!(liq_pool.get_unused_reward(), 1_0000000 * 80);

    jump(&e, 10);
    // new config iteration. pool got 50 seconds of rewards. 100 - 20 - 50 = 30 unused
    liq_pool.set_rewards_config(
        &admin,
        &e.ledger().timestamp().saturating_add(50),
        &1_0000000,
    );

    // neither time nor claim should affect unused rewards
    assert_eq!(liq_pool.get_unused_reward(), 1_0000000 * 30);
    jump(&e, 10);
    assert_eq!(liq_pool.get_unused_reward(), 1_0000000 * 30);
    liq_pool.claim(&user);
    assert_eq!(liq_pool.get_unused_reward(), 1_0000000 * 30);
    jump(&e, 10);
    assert_eq!(liq_pool.get_unused_reward(), 1_0000000 * 30);
    assert_eq!(setup.token_reward.balance(&router), 0);
    assert_eq!(liq_pool.return_unused_reward(&admin), 1_0000000 * 30);
    assert_eq!(setup.token_reward.balance(&router), 1_0000000 * 30);
}

#[test]
fn test_return_unused_reward_reward_token_in_pool() {
    let setup = Setup::new_with_config(&TestConfig {
        reward_tps: 0,
        reward_token_in_pool: true,
        mint_to_user: 0,
        rewards_count: 0,
        ..TestConfig::default()
    });
    assert_eq!(setup.token1.address, setup.token_reward.address);
    let e = setup.env;
    let admin = setup.admin;
    let liq_pool = setup.liq_pool;
    let router = setup.router;
    let token_1_admin_client = SorobanTokenAdminClient::new(&e, &setup.token1.address.clone());
    let token_2_admin_client = SorobanTokenAdminClient::new(&e, &setup.token2.address.clone());
    let token_reward_admin_client =
        SorobanTokenAdminClient::new(&e, &setup.token_reward.address.clone());
    let user = Address::generate(&e);

    token_1_admin_client.mint(&user, &1000_0000000);
    token_2_admin_client.mint(&user, &1000_0000000);
    liq_pool.deposit(
        &user,
        &Vec::from_array(&e, [1000_0000000, 1000_0000000]),
        &0,
    );

    liq_pool.set_rewards_config(
        &admin,
        &e.ledger().timestamp().saturating_add(60),
        &1_0000000,
    );
    // pool has configured rewards, but not minted
    assert_eq!(liq_pool.get_unused_reward(), 0);

    token_reward_admin_client.mint(&liq_pool.address, &(1_0000000 * 100));

    // we've configured rewards for 60 seconds, but minted for 100. 40 surplus
    assert_eq!(liq_pool.get_unused_reward(), 1_0000000 * 40);

    // 10 seconds passed
    jump(&e, 10);
    liq_pool.claim(&user);

    assert_eq!(liq_pool.get_unused_reward(), 1_0000000 * 40);
    assert_eq!(setup.token_reward.balance(&router), 0);
    jump(&e, 10);

    // pool stops rewards on new iteration
    liq_pool.set_rewards_config(&admin, &e.ledger().timestamp().saturating_add(0), &0);
    assert_eq!(liq_pool.get_unused_reward(), 1_0000000 * 80);

    jump(&e, 10);
    // new config iteration. pool got 50 seconds of rewards. 100 - 20 - 50 = 30 unused
    liq_pool.set_rewards_config(
        &admin,
        &e.ledger().timestamp().saturating_add(50),
        &1_0000000,
    );

    // neither time nor claim should affect unused rewards
    assert_eq!(liq_pool.get_unused_reward(), 1_0000000 * 30);
    jump(&e, 10);
    assert_eq!(liq_pool.get_unused_reward(), 1_0000000 * 30);
    liq_pool.claim(&user);
    assert_eq!(liq_pool.get_unused_reward(), 1_0000000 * 30);
    jump(&e, 10);
    assert_eq!(liq_pool.get_unused_reward(), 1_0000000 * 30);
    assert_eq!(setup.token_reward.balance(&router), 0);
    assert_eq!(liq_pool.return_unused_reward(&admin), 1_0000000 * 30);
    assert_eq!(setup.token_reward.balance(&router), 1_0000000 * 30);
}

#[test]
fn test_kill_deposit_event() {
    let setup = Setup::default();
    let pool = setup.liq_pool;

    pool.kill_deposit(&setup.admin);
    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![
            &setup.env,
            (
                pool.address.clone(),
                (Symbol::new(&setup.env, "kill_deposit"),).into_val(&setup.env),
                ().into_val(&setup.env),
            ),
        ]
    );
}

#[test]
fn test_kill_swap_event() {
    let setup = Setup::default();
    let pool = setup.liq_pool;

    pool.kill_swap(&setup.admin);
    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![
            &setup.env,
            (
                pool.address.clone(),
                (Symbol::new(&setup.env, "kill_swap"),).into_val(&setup.env),
                ().into_val(&setup.env),
            ),
        ]
    );
}

#[test]
fn test_kill_claim_event() {
    let setup = Setup::default();
    let pool = setup.liq_pool;

    pool.kill_claim(&setup.admin);
    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![
            &setup.env,
            (
                pool.address.clone(),
                (Symbol::new(&setup.env, "kill_claim"),).into_val(&setup.env),
                ().into_val(&setup.env),
            ),
        ]
    );
}

#[test]
fn test_unkill_deposit_event() {
    let setup = Setup::default();
    let pool = setup.liq_pool;

    pool.unkill_deposit(&setup.admin);
    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![
            &setup.env,
            (
                pool.address.clone(),
                (Symbol::new(&setup.env, "unkill_deposit"),).into_val(&setup.env),
                ().into_val(&setup.env),
            ),
        ]
    );
}

#[test]
fn test_unkill_swap_event() {
    let setup = Setup::default();
    let pool = setup.liq_pool;

    pool.unkill_swap(&setup.admin);
    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![
            &setup.env,
            (
                pool.address.clone(),
                (Symbol::new(&setup.env, "unkill_swap"),).into_val(&setup.env),
                ().into_val(&setup.env),
            ),
        ]
    );
}

#[test]
fn test_unkill_claim_event() {
    let setup = Setup::default();
    let pool = setup.liq_pool;

    pool.unkill_claim(&setup.admin);
    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![
            &setup.env,
            (
                pool.address.clone(),
                (Symbol::new(&setup.env, "unkill_claim"),).into_val(&setup.env),
                ().into_val(&setup.env),
            ),
        ]
    );
}

#[test]
fn test_set_privileged_addresses_event() {
    let setup = Setup::default();
    let pool = setup.liq_pool;

    pool.set_privileged_addrs(
        &setup.admin,
        &setup.rewards_admin,
        &setup.operations_admin,
        &setup.pause_admin,
        &Vec::from_array(&setup.env, [setup.emergency_pause_admin.clone()]),
        &setup.system_fee_admin,
    );

    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![
            &setup.env,
            (
                pool.address.clone(),
                (Symbol::new(&setup.env, "set_privileged_addrs"),).into_val(&setup.env),
                (
                    setup.rewards_admin,
                    setup.operations_admin,
                    setup.pause_admin,
                    Vec::from_array(&setup.env, [setup.emergency_pause_admin]),
                    setup.system_fee_admin,
                )
                    .into_val(&setup.env),
            ),
        ]
    );
}

#[test]
fn test_set_rewards_config() {
    let setup = Setup::default();
    let pool = setup.liq_pool;

    pool.set_rewards_config(
        &setup.admin.clone(),
        &setup.env.ledger().timestamp().saturating_add(100),
        &1_0000000,
    );

    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![
            &setup.env,
            (
                pool.address.clone(),
                (Symbol::new(&setup.env, "set_rewards_config"),).into_val(&setup.env),
                (
                    setup.env.ledger().timestamp().saturating_add(100),
                    1_0000000_u128,
                )
                    .into_val(&setup.env),
            ),
        ]
    );
}

#[test]
fn test_transfer_ownership_events() {
    let setup = Setup::default();
    let pool = setup.liq_pool;
    let new_admin = Address::generate(&setup.env);

    pool.commit_transfer_ownership(&setup.admin, &symbol_short!("Admin"), &new_admin);
    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![
            &setup.env,
            (
                pool.address.clone(),
                (
                    Symbol::new(&setup.env, "commit_transfer_ownership"),
                    symbol_short!("Admin")
                )
                    .into_val(&setup.env),
                (new_admin.clone(),).into_val(&setup.env),
            ),
        ]
    );

    pool.revert_transfer_ownership(&setup.admin, &symbol_short!("Admin"));
    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![
            &setup.env,
            (
                pool.address.clone(),
                (
                    Symbol::new(&setup.env, "revert_transfer_ownership"),
                    symbol_short!("Admin")
                )
                    .into_val(&setup.env),
                ().into_val(&setup.env),
            ),
        ]
    );

    pool.commit_transfer_ownership(&setup.admin, &symbol_short!("Admin"), &new_admin);
    jump(&setup.env, ADMIN_ACTIONS_DELAY + 1);
    pool.apply_transfer_ownership(&setup.admin, &symbol_short!("Admin"));
    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![
            &setup.env,
            (
                pool.address.clone(),
                (
                    Symbol::new(&setup.env, "apply_transfer_ownership"),
                    symbol_short!("Admin")
                )
                    .into_val(&setup.env),
                (new_admin.clone(),).into_val(&setup.env),
            ),
        ]
    );
}

#[test]
fn test_upgrade_events() {
    let setup = Setup::default();
    let contract = setup.liq_pool;
    let new_wasm_hash = install_dummy_wasm(&setup.env);
    let token_new_wasm_hash = install_dummy_wasm(&setup.env);
    let gauge_new_wasm_hash = install_dummy_wasm(&setup.env);

    contract.commit_upgrade(
        &setup.admin,
        &new_wasm_hash,
        &token_new_wasm_hash,
        &gauge_new_wasm_hash,
    );
    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![
            &setup.env,
            (
                contract.address.clone(),
                (Symbol::new(&setup.env, "commit_upgrade"),).into_val(&setup.env),
                (
                    new_wasm_hash.clone(),
                    token_new_wasm_hash.clone(),
                    gauge_new_wasm_hash.clone()
                )
                    .into_val(&setup.env),
            ),
        ]
    );

    contract.revert_upgrade(&setup.admin);
    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![
            &setup.env,
            (
                contract.address.clone(),
                (Symbol::new(&setup.env, "revert_upgrade"),).into_val(&setup.env),
                ().into_val(&setup.env),
            ),
        ]
    );

    contract.commit_upgrade(
        &setup.admin,
        &new_wasm_hash,
        &token_new_wasm_hash,
        &gauge_new_wasm_hash,
    );
    jump(&setup.env, ADMIN_ACTIONS_DELAY + 1);
    contract.apply_upgrade(&setup.admin);
    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![
            &setup.env,
            (
                contract.address.clone(),
                (Symbol::new(&setup.env, "apply_upgrade"),).into_val(&setup.env),
                (new_wasm_hash.clone(), token_new_wasm_hash.clone()).into_val(&setup.env),
            ),
        ]
    );
}

#[test]
fn test_emergency_mode_events() {
    let setup = Setup::default();
    let contract = setup.liq_pool;

    contract.set_emergency_mode(&setup.emergency_admin, &true);
    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![
            &setup.env,
            (
                contract.address.clone(),
                (Symbol::new(&setup.env, "enable_emergency_mode"),).into_val(&setup.env),
                ().into_val(&setup.env),
            ),
        ]
    );
    contract.set_emergency_mode(&setup.emergency_admin, &false);
    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![
            &setup.env,
            (
                contract.address.clone(),
                (Symbol::new(&setup.env, "disable_emergency_mode"),).into_val(&setup.env),
                ().into_val(&setup.env),
            ),
        ]
    );
}

#[test]
fn test_emergency_upgrade() {
    let setup = Setup::default();
    let contract = setup.liq_pool;
    let token = ShareTokenClient::new(&setup.env, &contract.share_id());

    let new_wasm = install_dummy_wasm(&setup.env);
    let new_token_wasm = install_dummy_wasm(&setup.env);
    let new_gauge_wasm = install_dummy_wasm(&setup.env);

    assert_eq!(contract.get_emergency_mode(), false);
    assert_ne!(contract.version(), 130);
    assert_ne!(token.version(), 130);
    contract.set_emergency_mode(&setup.emergency_admin, &true);

    contract.commit_upgrade(&setup.admin, &new_wasm, &new_token_wasm, &new_gauge_wasm);
    contract.apply_upgrade(&setup.admin);

    assert_eq!(contract.version(), 130);
    assert_eq!(token.version(), 130);
}

#[test]
#[should_panic(expected = "Error(Contract, #2907)")]
fn test_apply_emergency_upgrade_not_commited() {
    let setup = Setup::default();
    let contract = setup.liq_pool;

    let new_wasm = install_dummy_wasm(&setup.env);
    let new_token_wasm = install_dummy_wasm(&setup.env);
    let new_gauge_wasm = install_dummy_wasm(&setup.env);
    contract.commit_upgrade(&setup.admin, &new_wasm, &new_token_wasm, &new_gauge_wasm);
    contract.revert_upgrade(&setup.admin);

    contract.set_emergency_mode(&setup.emergency_admin, &true);
    contract.apply_upgrade(&setup.admin);
}

#[test]
fn test_regular_upgrade_token() {
    let setup = Setup::default();
    let contract = setup.liq_pool;
    let token = ShareTokenClient::new(&setup.env, &contract.share_id());

    let token_wasm = setup
        .env
        .deployer()
        .upload_contract_wasm(token_share::token::WASM);
    let new_wasm = install_dummy_wasm(&setup.env);
    let new_gauge_wasm = install_dummy_wasm(&setup.env);

    // dummy wasm has version 130, everything else has greater version
    assert_eq!(contract.get_emergency_mode(), false);
    assert_ne!(contract.version(), 130);
    assert_ne!(token.version(), 130);

    contract.commit_upgrade(&setup.admin, &new_wasm, &token_wasm, &new_gauge_wasm);
    assert!(contract.try_apply_upgrade(&setup.admin).is_err());
    jump(&setup.env, ADMIN_ACTIONS_DELAY + 1);
    assert_eq!(
        contract.apply_upgrade(&setup.admin),
        (new_wasm.clone(), token_wasm.clone())
    );

    assert_eq!(contract.version(), 130);
    assert_ne!(token.version(), 130);
}

#[test]
fn test_regular_upgrade_pool() {
    let setup = Setup::default();
    let contract = setup.liq_pool;
    let token = ShareTokenClient::new(&setup.env, &contract.share_id());
    let gauge = deploy_rewards_gauge(&setup.env, &contract.address, &setup.token_reward.address);

    let new_wasm = install_dummy_wasm(&setup.env);
    let new_token_wasm = install_dummy_wasm(&setup.env);
    let new_gauge_wasm = install_dummy_wasm(&setup.env);

    // dummy wasm has version 130, everything else has greater version
    assert_eq!(contract.get_emergency_mode(), false);
    assert_ne!(contract.version(), 130);
    assert_ne!(token.version(), 130);
    assert_ne!(gauge.version(), 130);

    // add gauge to pool
    contract.gauge_add(&setup.admin, &gauge.address);
    contract.commit_upgrade(&setup.admin, &new_wasm, &new_token_wasm, &new_gauge_wasm);
    assert!(contract.try_apply_upgrade(&setup.admin).is_err());
    jump(&setup.env, ADMIN_ACTIONS_DELAY + 1);
    assert_eq!(
        contract.apply_upgrade(&setup.admin),
        (new_wasm.clone(), new_token_wasm.clone())
    );

    assert_eq!(contract.version(), 130);
    assert_eq!(token.version(), 130);
    assert_eq!(gauge.version(), 130);
}

#[test]
fn test_claim_event() {
    let setup = Setup::default();
    let liq_pool = setup.liq_pool;
    let token_1_admin_client =
        SorobanTokenAdminClient::new(&setup.env, &setup.token1.address.clone());
    let token_2_admin_client =
        SorobanTokenAdminClient::new(&setup.env, &setup.token2.address.clone());
    let token_reward_admin_client =
        SorobanTokenAdminClient::new(&setup.env, &setup.token_reward.address.clone());

    let user = Address::generate(&setup.env);

    token_1_admin_client.mint(&user, &1000);
    token_2_admin_client.mint(&user, &1000);
    liq_pool.deposit(&user, &Vec::from_array(&setup.env, [1000, 1000]), &0);
    token_reward_admin_client.mint(&liq_pool.address, &1_000_000_0000000);
    let reward_1_tps = 10_5000000_u128;
    let total_reward_1 = reward_1_tps * 70;
    liq_pool.set_rewards_config(
        &setup.admin,
        &setup.env.ledger().timestamp().saturating_add(70),
        &reward_1_tps,
    );
    jump(&setup.env, 70);
    liq_pool.claim(&user);

    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![
            &setup.env,
            (
                liq_pool.address.clone(),
                (
                    Symbol::new(&setup.env, "claim_reward"),
                    setup.token_reward.address.clone(),
                    user.clone(),
                )
                    .into_val(&setup.env),
                (total_reward_1 as i128,).into_val(&setup.env),
            ),
        ]
    );
}

#[test]
fn test_deposit_and_withdraw_fee() {
    let setup = Setup::default();
    let token_1_admin_client =
        SorobanTokenAdminClient::new(&setup.env, &setup.token1.address.clone());
    let token_2_admin_client =
        SorobanTokenAdminClient::new(&setup.env, &setup.token2.address.clone());

    let user1 = Address::generate(&setup.env);
    token_1_admin_client.mint(&user1, &1000_0000000);
    token_2_admin_client.mint(&user1, &1000_0000000);

    let user2 = Address::generate(&setup.env);
    token_1_admin_client.mint(&user2, &1000_0000000);
    token_2_admin_client.mint(&user2, &1000_0000000);

    // initialize pool reserves to avoid edge cases with first deposit/withdraw
    setup.liq_pool.deposit(
        &user1,
        &Vec::from_array(&setup.env, [500_0000000, 1000_0000000]),
        &0,
    );
    assert_eq!(setup.token1.balance(&setup.liq_pool.address), 500_0000000);
    assert_eq!(setup.token2.balance(&setup.liq_pool.address), 1000_0000000);
    assert_eq!(
        setup.liq_pool.get_reserves(),
        Vec::from_array(&setup.env, [500_0000000, 1000_0000000])
    );

    setup.liq_pool.deposit(
        &user2,
        &Vec::from_array(&setup.env, [1000_0000000, 1000_0000000]),
        &0,
    );
    assert_eq!(setup.token1.balance(&setup.liq_pool.address), 1000_0000000);
    assert_eq!(setup.token2.balance(&setup.liq_pool.address), 2000_0000000);
    assert_eq!(
        setup.liq_pool.get_reserves(),
        Vec::from_array(&setup.env, [1000_0000000, 2000_0000000])
    );

    setup.liq_pool.withdraw(
        &user2,
        &(setup.token_share.balance(&user2) as u128),
        &Vec::from_array(&setup.env, [0, 0]),
    );
    assert_eq!(setup.token1.balance(&user2), 1000_0000000);
    assert_eq!(setup.token2.balance(&user2), 1000_0000000);
    assert_eq!(setup.token1.balance(&setup.liq_pool.address), 500_0000000);
    assert_eq!(setup.token2.balance(&setup.liq_pool.address), 1000_0000000);
    assert_eq!(
        setup.liq_pool.get_reserves(),
        Vec::from_array(&setup.env, [500_0000000, 1000_0000000])
    );
}

#[test]
fn test_custom_protocol_fee() {
    let config = TestConfig {
        mint_to_user: 1000000_0000000,
        ..TestConfig::default()
    };
    let setup = Setup::new_with_config(&config);

    // we're checking fraction against output for 1 token
    for (protocol_fee_fraction, protocol_fee_amount) in [
        (1000, 3000_u128),  // 0.3% * 10%
        (3000, 9000_u128),  // 0.3% * 30%
        (5000, 15000_u128), // 0.3% * 50%
    ] {
        let liqpool = create_liqpool_contract(
            &setup.env,
            &setup.admin,
            &Address::generate(&setup.env),
            &setup.oracle_addr,
            &install_token_wasm(&setup.env),
            &Vec::from_array(
                &setup.env,
                [setup.token1.address.clone(), setup.token2.address.clone()],
            ),
            &setup.token_reward.address,
            30,
            &setup.plane.address,
            &setup.config_storage.address,
        );
        liqpool.set_protocol_fee_fraction(&setup.admin, &protocol_fee_fraction);
        liqpool.deposit(
            &setup.users[0],
            &Vec::from_array(&setup.env, [100_0000000, 100_0000000]),
            &0,
        );
        assert_eq!(liqpool.estimate_swap(&1, &0, &1_0000000, &false), 9871580);
        assert_eq!(
            liqpool.swap(&setup.users[0], &1, &0, &1_0000000, &0),
            9871580
        );
        assert_eq!(liqpool.get_protocol_fees().get_unchecked(0), 0);
        assert_eq!(
            liqpool.get_protocol_fees().get_unchecked(1),
            protocol_fee_amount
        );

        // full withdraw & deposit to reset pool reserves
        liqpool.withdraw(
            &setup.users[0],
            &(SorobanTokenClient::new(&setup.env, &liqpool.share_id()).balance(&setup.users[0])
                as u128),
            &Vec::from_array(&setup.env, [0, 0]),
        );
        liqpool.deposit(
            &setup.users[0],
            &Vec::from_array(&setup.env, [100_0000000, 100_0000000]),
            &0,
        );
        assert_eq!(liqpool.estimate_swap(&0, &1, &1_0000000, &false), 9871580); // re-check swap result didn't change
        assert_eq!(
            liqpool.estimate_swap_strict_receive(&0, &1, &9871580, &false),
            1_0000000
        );
        assert_eq!(
            liqpool.swap_strict_receive(&setup.users[0], &0, &1, &9871580, &1_0000000),
            1_0000000
        );
        // each time we collect protocol fee on input token
        assert_eq!(
            liqpool.get_protocol_fees().get_unchecked(0),
            protocol_fee_amount
        );
        assert_eq!(
            liqpool.get_protocol_fees().get_unchecked(1),
            protocol_fee_amount
        );
    }
}

#[test]
fn test_simple_reward_gauge() {
    let setup = Setup::setup(&TestConfig::default());
    let env = setup.env;
    let liq_pool = setup.liq_pool;
    let user = Address::generate(&env);

    let gauge_reward_token = create_token_contract(&env, &setup.admin);
    let distributor = Address::generate(&env);
    let gauge = deploy_rewards_gauge(&env, &liq_pool.address, &gauge_reward_token.address);
    liq_pool.gauge_add(&setup.admin, &gauge.address);

    // 10 seconds. user depositing
    jump(&env, 10);
    SorobanTokenAdminClient::new(&env, &setup.token1.address).mint(&user, &100);
    SorobanTokenAdminClient::new(&env, &setup.token2.address).mint(&user, &100);
    liq_pool.deposit(&user, &Vec::from_array(&env, [100, 100]), &0);

    // 20 seconds. rewards set up for 60 seconds
    jump(&env, 10);
    let reward_1_tps = 10_5000000_u128;
    let total_reward_1 = reward_1_tps * 60;
    get_token_admin_client(&env, &gauge_reward_token.address)
        .mint(&distributor, &(total_reward_1 as i128));
    liq_pool.gauge_schedule_reward(
        &setup.router,
        &distributor,
        &gauge.address,
        &None,
        &60,
        &reward_1_tps,
    );

    // 90 seconds. rewards ended.
    jump(&env, 70);
    // 100 seconds. user claim reward
    jump(&env, 10);
    assert_eq!(gauge_reward_token.balance(&user), 0);
    // full reward should be available to the user
    assert_eq!(
        liq_pool.gauges_claim(&user),
        Map::from_array(&env, [(gauge_reward_token.address.clone(), total_reward_1)])
    );
    assert_eq!(gauge_reward_token.balance(&user) as u128, total_reward_1);
}

#[test]
fn test_retroactive_reward_gauge() {
    let setup = Setup::setup(&TestConfig::default());
    let env = setup.env;
    let liq_pool = setup.liq_pool;

    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let user3 = Address::generate(&env);
    SorobanTokenAdminClient::new(&env, &setup.token1.address).mint(&user1, &100);
    SorobanTokenAdminClient::new(&env, &setup.token2.address).mint(&user1, &100);
    SorobanTokenAdminClient::new(&env, &setup.token1.address).mint(&user2, &100);
    SorobanTokenAdminClient::new(&env, &setup.token2.address).mint(&user2, &100);
    SorobanTokenAdminClient::new(&env, &setup.token1.address).mint(&user3, &100);
    SorobanTokenAdminClient::new(&env, &setup.token2.address).mint(&user3, &100);

    // 10 seconds. first user depositing before gauge set up
    jump(&env, 10);
    liq_pool.deposit(&user1, &Vec::from_array(&env, [100, 100]), &0);
    liq_pool.deposit(&user3, &Vec::from_array(&env, [100, 100]), &0);

    // 15 seconds. third user withdrawing before gauge set up
    jump(&env, 5);
    liq_pool.withdraw(
        &user3,
        &(SorobanTokenClient::new(&env, &liq_pool.share_id()).balance(&user3) as u128),
        &Vec::from_array(&env, [0, 0]),
    );

    // 20 seconds. gauge set up
    jump(&env, 5);
    let gauge_reward_token = create_token_contract(&env, &setup.admin);
    let distributor = Address::generate(&env);
    let gauge = deploy_rewards_gauge(&env, &liq_pool.address, &gauge_reward_token.address);
    liq_pool.gauge_add(&setup.admin, &gauge.address);

    // 30 seconds. rewards set up for 60 seconds
    jump(&env, 10);
    let reward_1_tps = 10_5000000_u128;
    let total_reward_1 = reward_1_tps * 60;
    get_token_admin_client(&env, &gauge_reward_token.address)
        .mint(&distributor, &(total_reward_1 as i128));
    liq_pool.gauge_schedule_reward(
        &setup.router,
        &distributor,
        &gauge.address,
        &None,
        &60,
        &reward_1_tps,
    );

    // 40 seconds. second user depositing after gauge set up
    jump(&env, 30);
    liq_pool.deposit(&user2, &Vec::from_array(&env, [100, 100]), &0);

    // 100 seconds. rewards ended. third user depositing after rewards ended
    jump(&env, 40);
    liq_pool.deposit(&user3, &Vec::from_array(&env, [100, 100]), &0);

    // 110 seconds. user claims reward
    jump(&env, 10);
    assert_eq!(gauge_reward_token.balance(&user1), 0);
    assert_eq!(gauge_reward_token.balance(&user2), 0);
    assert_eq!(gauge_reward_token.balance(&user3), 0);
    // full reward should be available to users. first receives 3/4 of the reward, second receives 1/4 since it joined later
    assert_eq!(
        liq_pool.gauges_claim(&user1),
        Map::from_array(
            &env,
            [(gauge_reward_token.address.clone(), total_reward_1 / 4 * 3)]
        )
    );
    assert_eq!(
        liq_pool.gauges_claim(&user2),
        Map::from_array(
            &env,
            [(gauge_reward_token.address.clone(), total_reward_1 / 4)]
        )
    );
    assert_eq!(
        liq_pool.gauges_claim(&user3),
        Map::from_array(&env, [(gauge_reward_token.address.clone(), 0)])
    );
    assert_eq!(
        gauge_reward_token.balance(&user1) as u128,
        total_reward_1 / 4 * 3
    );
    assert_eq!(
        gauge_reward_token.balance(&user2) as u128,
        total_reward_1 / 4
    );
    assert_eq!(gauge_reward_token.balance(&user3) as u128, 0);
}

#[test]
fn test_reward_info_getter() {
    let setup = Setup::setup(&TestConfig::default());
    let env = setup.env;
    let liq_pool = setup.liq_pool;
    let user = Address::generate(&env);

    let epoch_start = env.ledger().timestamp();

    let gauge_reward_token = create_token_contract(&env, &setup.admin);
    let distributor = Address::generate(&env);
    let gauge = deploy_rewards_gauge(&env, &liq_pool.address, &gauge_reward_token.address);
    liq_pool.gauge_add(&setup.admin, &gauge.address);

    // 10 seconds. user depositing
    jump(&env, 10);
    SorobanTokenAdminClient::new(&env, &setup.token1.address).mint(&user, &100);
    SorobanTokenAdminClient::new(&env, &setup.token2.address).mint(&user, &100);
    liq_pool.deposit(&user, &Vec::from_array(&env, [100, 100]), &0);

    // 20 seconds. rewards set up for 60 seconds
    jump(&env, 10);
    let reward_1_tps = 10_5000000_u128;
    let total_reward_1 = reward_1_tps * 60;
    get_token_admin_client(&env, &gauge_reward_token.address)
        .mint(&distributor, &(total_reward_1 as i128));
    liq_pool.gauge_schedule_reward(
        &setup.router,
        &distributor,
        &gauge.address,
        &None,
        &60,
        &reward_1_tps,
    );

    jump(&env, 10);
    let data = liq_pool.gauges_get_reward_info(&user);
    assert_eq!(
        data,
        Map::from_array(
            &env,
            [(
                gauge_reward_token.address.clone(),
                Map::from_array(
                    &env,
                    [
                        (Symbol::new(&env, "user_reward"), reward_1_tps as i128 * 10),
                        (Symbol::new(&env, "tps"), reward_1_tps as i128),
                        (
                            Symbol::new(&env, "expired_at"),
                            epoch_start as i128 + 80_i128
                        ),
                    ]
                )
            )]
        )
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #401)")]
fn test_add_gauge_twice() {
    let setup = Setup::setup(&TestConfig::default());
    let gauge_reward_token = create_token_contract(&setup.env, &setup.admin);
    let gauge = deploy_rewards_gauge(
        &setup.env,
        &setup.liq_pool.address,
        &gauge_reward_token.address,
    );
    setup.liq_pool.gauge_add(&setup.admin, &gauge.address);
    setup.liq_pool.gauge_add(&setup.admin, &gauge.address);
}

#[test]
#[should_panic(expected = "Error(Contract, #305)")]
fn test_add_gauges_over_max() {
    let setup = Setup::setup(&TestConfig::default());
    for _ in 0..6 {
        let gauge_reward_token = create_token_contract(&setup.env, &setup.admin);
        let gauge = deploy_rewards_gauge(
            &setup.env,
            &setup.liq_pool.address,
            &gauge_reward_token.address,
        );
        setup.liq_pool.gauge_add(&setup.admin, &gauge.address);
    }
}

#[test]
fn test_remove_gauge() {
    let setup = Setup::setup(&TestConfig::default());
    let gauge_reward_token = create_token_contract(&setup.env, &setup.admin);
    let gauge = deploy_rewards_gauge(
        &setup.env,
        &setup.liq_pool.address,
        &gauge_reward_token.address,
    );
    assert_eq!(setup.liq_pool.get_gauges(), Map::new(&setup.env));
    setup.liq_pool.gauge_add(&setup.admin, &gauge.address);
    assert_eq!(
        setup.liq_pool.get_gauges(),
        Map::from_array(
            &setup.env,
            [(gauge_reward_token.address.clone(), gauge.address.clone())]
        )
    );
    setup
        .liq_pool
        .gauge_remove(&setup.admin, &gauge_reward_token.address);
    assert_eq!(setup.liq_pool.get_gauges(), Map::new(&setup.env));
}

#[test]
#[should_panic(expected = "Error(Contract, #404)")]
fn test_remove_gauge_twice() {
    let setup = Setup::setup(&TestConfig::default());
    let gauge_reward_token = create_token_contract(&setup.env, &setup.admin);
    let gauge = deploy_rewards_gauge(
        &setup.env,
        &setup.liq_pool.address,
        &gauge_reward_token.address,
    );
    setup.liq_pool.gauge_add(&setup.admin, &gauge.address);
    setup
        .liq_pool
        .gauge_remove(&setup.admin, &gauge_reward_token.address);
    setup
        .liq_pool
        .gauge_remove(&setup.admin, &gauge_reward_token.address);
}

#[test]
fn test_gauges_kill_claim() {
    let setup = Setup::setup(&TestConfig::default());
    let user = Address::generate(&setup.env);
    let gauge_reward_token = create_token_contract(&setup.env, &setup.admin);
    let gauge = deploy_rewards_gauge(
        &setup.env,
        &setup.liq_pool.address,
        &gauge_reward_token.address,
    );
    setup.liq_pool.gauge_add(&setup.admin, &gauge.address);
    assert!(setup.liq_pool.try_gauges_claim(&user).is_ok());
    setup.liq_pool.kill_gauges_claim(&setup.admin);
    assert!(setup.liq_pool.try_gauges_claim(&user).is_err());
}

#[test]
fn test_gauges_kill_claim_pause_admin() {
    let setup = Setup::setup(&TestConfig::default());
    let gauge_reward_token = create_token_contract(&setup.env, &setup.admin);
    let gauge = deploy_rewards_gauge(
        &setup.env,
        &setup.liq_pool.address,
        &gauge_reward_token.address,
    );
    setup.liq_pool.gauge_add(&setup.admin, &gauge.address);
    setup.liq_pool.kill_gauges_claim(&setup.pause_admin);
}

#[test]
fn test_gauges_kill_claim_emergency_pause_admin() {
    let setup = Setup::setup(&TestConfig::default());
    let gauge_reward_token = create_token_contract(&setup.env, &setup.admin);
    let gauge = deploy_rewards_gauge(
        &setup.env,
        &setup.liq_pool.address,
        &gauge_reward_token.address,
    );
    setup.liq_pool.gauge_add(&setup.admin, &gauge.address);
    setup
        .liq_pool
        .kill_gauges_claim(&setup.emergency_pause_admin);
}

#[test]
#[should_panic(expected = "Error(Contract, #102)")]
fn test_gauges_kill_claim_third_user() {
    let setup = Setup::setup(&TestConfig::default());
    let gauge_reward_token = create_token_contract(&setup.env, &setup.admin);
    let gauge = deploy_rewards_gauge(
        &setup.env,
        &setup.liq_pool.address,
        &gauge_reward_token.address,
    );
    setup.liq_pool.gauge_add(&setup.admin, &gauge.address);
    setup
        .liq_pool
        .kill_gauges_claim(&Address::generate(&setup.env));
}

#[test]
fn test_gauges_unkill_claim() {
    let setup = Setup::setup(&TestConfig::default());
    let user = Address::generate(&setup.env);
    let gauge_reward_token = create_token_contract(&setup.env, &setup.admin);
    let gauge = deploy_rewards_gauge(
        &setup.env,
        &setup.liq_pool.address,
        &gauge_reward_token.address,
    );
    setup.liq_pool.gauge_add(&setup.admin, &gauge.address);
    assert!(setup.liq_pool.try_gauges_claim(&user).is_ok());
    setup.liq_pool.kill_gauges_claim(&setup.admin);
    assert!(setup.liq_pool.try_gauges_claim(&user).is_err());
    setup.liq_pool.unkill_gauges_claim(&setup.admin);
    assert!(setup.liq_pool.try_gauges_claim(&user).is_ok());
}

#[test]
fn test_gauges_unkill_claim_pause_admin() {
    let setup = Setup::setup(&TestConfig::default());
    let gauge_reward_token = create_token_contract(&setup.env, &setup.admin);
    let gauge = deploy_rewards_gauge(
        &setup.env,
        &setup.liq_pool.address,
        &gauge_reward_token.address,
    );
    setup.liq_pool.gauge_add(&setup.admin, &gauge.address);
    setup.liq_pool.kill_gauges_claim(&setup.pause_admin);
    setup.liq_pool.unkill_gauges_claim(&setup.pause_admin);
}

#[test]
#[should_panic(expected = "Error(Contract, #102)")]
fn test_gauges_unkill_claim_emergency_pause_admin() {
    let setup = Setup::setup(&TestConfig::default());
    let gauge_reward_token = create_token_contract(&setup.env, &setup.admin);
    let gauge = deploy_rewards_gauge(
        &setup.env,
        &setup.liq_pool.address,
        &gauge_reward_token.address,
    );
    setup.liq_pool.gauge_add(&setup.admin, &gauge.address);
    setup.liq_pool.kill_gauges_claim(&setup.admin);
    setup
        .liq_pool
        .unkill_gauges_claim(&setup.emergency_pause_admin);
}

#[test]
#[should_panic(expected = "Error(Contract, #102)")]
fn test_gauges_unkill_claim_third_user() {
    let setup = Setup::setup(&TestConfig::default());
    let gauge_reward_token = create_token_contract(&setup.env, &setup.admin);
    let gauge = deploy_rewards_gauge(
        &setup.env,
        &setup.liq_pool.address,
        &gauge_reward_token.address,
    );
    setup.liq_pool.gauge_add(&setup.admin, &gauge.address);
    setup.liq_pool.kill_gauges_claim(&setup.admin);
    setup
        .liq_pool
        .unkill_gauges_claim(&Address::generate(&setup.env));
}

#[test]
fn test_fix_broken_claim() {
    let setup = Setup::new_with_config(&TestConfig {
        rewards_count: 0,
        reward_tps: 0,
        ..TestConfig::default()
    });
    let e = setup.env;
    let liq_pool = setup.liq_pool;

    // Mint pool tokens to both users and deposit equal amounts
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);
    setup.token1_admin_client.mint(&user1, &1_000_0000000);
    setup.token2_admin_client.mint(&user1, &1_000_0000000);
    setup.token1_admin_client.mint(&user2, &1_000_0000000);
    setup.token2_admin_client.mint(&user2, &1_000_0000000);

    // first user is regular depositor. uses lock tokens normally
    liq_pool.deposit(
        &user1,
        &Vec::from_array(&e, [1_000_0000000, 1_000_0000000]),
        &0,
    );

    // second user is the attacker
    liq_pool.deposit(
        &user2,
        &Vec::from_array(&e, [1_000_0000000, 1_000_0000000]),
        &0,
    );

    jump(&e, 1);

    // Configure rewards for 60 seconds @ 1 token/sec and mint exact amount to pool
    let tps = 1_0000000_u128;
    let duration = 60_u64;
    liq_pool.set_rewards_config(
        &setup.admin,
        &e.ledger().timestamp().saturating_add(duration),
        &tps,
    );
    setup.token_reward_admin_client.mint(
        &liq_pool.address,
        &((liq_pool.get_total_configured_reward() - liq_pool.get_total_claimed_reward()) as i128),
    );

    jump(&e, 1);
    // attacher somehow manages to abuse the system, so reward of two is greater than total configured
    e.as_contract(&liq_pool.address, || {
        let storage = get_rewards_manager(&e).storage();
        let mut user_data = storage.get_user_reward_data(&user2).unwrap();
        user_data.to_claim += 1_0000000;
        storage.set_user_reward_data(&user2, &user_data);
    });

    jump(&e, 60);

    let to_claim1 = liq_pool.get_user_reward(&user1);
    let to_claim2 = liq_pool.get_user_reward(&user2);
    assert!(to_claim1 + to_claim2 > liq_pool.get_total_configured_reward());

    // we've got broken rewards for one of the users.
    // let's say we've fixed the root of the issue, but we still need to deal with consequences
    assert!(to_claim1 + to_claim2 > tps * duration as u128);
    // adjust amount to sync the reward
    assert_ne!(
        liq_pool.get_total_configured_reward(),
        to_claim1 + to_claim2
    );
    liq_pool.adjust_total_accumulated_reward(
        &setup.admin,
        &((to_claim1 + to_claim2 - liq_pool.get_total_configured_reward()) as i128),
    );
    assert_eq!(
        liq_pool.get_total_configured_reward(),
        to_claim1 + to_claim2
    );
    setup.token_reward_admin_client.mint(
        &liq_pool.address,
        &(liq_pool.get_total_configured_reward() as i128
            - setup.token_reward.balance(&liq_pool.address)),
    );

    // now both can claim
    let to_claim1 = liq_pool.claim(&user1);
    let to_claim2 = liq_pool.claim(&user2);
    assert!(to_claim1 > 0 && to_claim2 > 0);
    assert_eq!(
        liq_pool.get_total_configured_reward(),
        liq_pool.get_total_claimed_reward(),
    );
}

#[test]
fn test_fix_locked_reward_tokens() {
    let setup = Setup::new_with_config(&TestConfig {
        rewards_count: 0,
        reward_tps: 0,
        ..TestConfig::default()
    });
    let e = setup.env;
    let liq_pool = setup.liq_pool;

    // Mint pool tokens to both users and deposit equal amounts
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);
    setup.token1_admin_client.mint(&user1, &1_000_0000000);
    setup.token2_admin_client.mint(&user1, &1_000_0000000);
    setup.token1_admin_client.mint(&user2, &1_000_0000000);
    setup.token2_admin_client.mint(&user2, &1_000_0000000);

    // pool reward is overinflated due to historical reasons. we've got 1 locked token we'd like to recover
    e.as_contract(&liq_pool.address, || {
        let storage = get_rewards_manager(&e).storage();
        let mut pool_data = storage.get_pool_reward_data();
        pool_data.accumulated += 1_0000000;
        storage.set_pool_reward_data(&pool_data);
    });

    // users behave normally: simple deposit with boost due to locked tokens
    liq_pool.deposit(
        &user1,
        &Vec::from_array(&e, [1_000_0000000, 1_000_0000000]),
        &0,
    );
    liq_pool.deposit(
        &user2,
        &Vec::from_array(&e, [1_000_0000000, 1_000_0000000]),
        &0,
    );

    jump(&e, 1);

    // Configure rewards for 60 seconds @ 1 token/sec and mint exact amount to pool
    let tps = 1_0000000_u128;
    let duration = 60_u64;
    liq_pool.set_rewards_config(
        &setup.admin,
        &e.ledger().timestamp().saturating_add(duration),
        &tps,
    );
    setup.token_reward_admin_client.mint(
        &liq_pool.address,
        &((liq_pool.get_total_configured_reward() - liq_pool.get_total_claimed_reward()) as i128),
    );

    jump(&e, 60);

    let claimed1 = liq_pool.claim(&user1);
    let claimed2 = liq_pool.claim(&user2);
    assert!(claimed1 + claimed2 < liq_pool.get_total_configured_reward());

    // all users claimed their reward, leaving the frozen token in the pool which we cannot recover
    assert!(liq_pool.get_total_configured_reward() > liq_pool.get_total_claimed_reward());
    // the reward doesn't count as unused either
    assert_eq!(liq_pool.get_unused_reward(), 0);
    liq_pool.adjust_total_accumulated_reward(&setup.admin, &-1_0000000);
    // now we see the reward as unused
    assert_eq!(liq_pool.get_unused_reward(), 1_0000000);
    // it may be used for the future reward distribution. configure reward for 1 second @ 1tps
    liq_pool.set_rewards_config(
        &setup.admin,
        &e.ledger().timestamp().saturating_add(1),
        &1_0000000,
    );
    assert_eq!(liq_pool.get_unused_reward(), 0);
    assert_eq!(
        liq_pool.get_total_configured_reward() - liq_pool.get_total_claimed_reward(),
        1_0000000
    );
}

// /** Custom Tests */
// fn random_bool(env: &Env) -> bool {
//     // Generate 1 random byte deterministically from the Soroban env
//     let rand_bytes: BytesN<1> = env.prng().gen();
//     let b: u8 = rand_bytes.get(0).unwrap();

//     // Even/odd → true/false
//     b % 2 == 0
// }

// fn random_in_range(env: &Env, min: u32, max: u32) -> u32 {
//     assert!(min < max, "min must be less than max");

//     // Generate 4 random bytes
//     let rand_bytes: BytesN<4> = env.prng().gen();
//     let mut raw: [u8; 4] = [0; 4];
//     raw.copy_from_slice(&rand_bytes.to_array());

//     // Convert bytes → u32
//     let n = u32::from_le_bytes(raw);

//     // Map it into [min, max)
//     (n % (max - min)) + min
// }

// fn test_swaps_many_users(seconds_to_simulate: u64) {
//     // first user (protocol admin) comes as initial liquidity provider
//     //  synthetic airdropped to many users
//     //  many users come to swap

//     let setup = Setup::new_with_config(&TestConfig {
//         users_count: 100,
//         ..TestConfig::default()
//     });
//     let env = setup.env;
//     let liq_pool = setup.liq_pool;
//     let users = setup.users;
//     let token1_admin_client = setup.token1_admin_client;
//     let token2_admin_client = setup.token2_admin_client;

//     let admin = users[0].clone();
//     let first_user = Address::generate(&env);

//     let initial_target_price = setup.oracle_client.lastprice(&setup.sol_asset).unwrap().price as u128;
//     let initial_usdc_liquidity = 1_000_000_0000000_u128; // 1 million
//     let initial_synthetic_liquidity = initial_usdc_liquidity / initial_target_price;

//     // Airdrop 10% of the initial liquidity amount of synthetic tokens to users
//     let airdrop_percent_of_liquidity = 0_1000000_u128; // 10%
//     let airdrop_amount = initial_synthetic_liquidity * airdrop_percent_of_liquidity;
//     let airdrop_frequency = 3;

//     for i in 0..101 {
//         let user = match i {
//             0 => &first_user,
//             val => &users[val - 1],
//         };
//         if i % airdrop_frequency == 0 {
//             token1_admin_client.mint(user, &(airdrop_amount as i128)); // synthetic
//         }

//         // usdc
//         if i == 0 {
//             token2_admin_client.mint(user, &1_000_000_000_000_000_000_000);
//         } else {
//             token2_admin_client.mint(user, &10_000_0000000);
//         }

//     }

//     // Protocol-owned liquidity (POL) deposit
//     // Desired amounts set so that initial pool price is at peg (oracle)
//     liq_pool.deposit(
//         &first_user,
//         &Vec::from_array(&env, [initial_synthetic_liquidity, initial_usdc_liquidity]),
//         &0,
//     );
//     jump(&env, 1);

//     // Simulate swaps
//     let rng_seed = 0;
//     let rebase_interval = liq_pool.get_rebase_interval();
//     let usdc_variance = 0_0200000; // 2%
//     let target_variance = 0_1000000; // 10%

//     for i in 1..seconds_to_simulate as usize {
//         let current_time = env.ledger().timestamp();

//         // Update oracle prices
//         let current_sol_oracle_price = setup.oracle_client.lastprice(&setup.sol_asset).unwrap().price;
//         let current_usdc_oracle_price = setup.oracle_client.lastprice(&setup.usdc_asset).unwrap().price;

//         let sol_price_update_direction = if random_bool(&env) { 1_i128 } else { -1_i128 };
//         let usdc_price_update_direction = if random_bool(&env) { 1_i128 } else { -1_i128 };

//         let sol_price_change = current_sol_oracle_price * target_variance * sol_price_update_direction;
//         let usdc_price_change = current_usdc_oracle_price * usdc_variance * usdc_price_update_direction;

//         let new_sol_oracle_price = current_sol_oracle_price + sol_price_change;
//         let new_usdc_oracle_price = current_usdc_oracle_price + usdc_price_change;

//         let updated_prices = Vec::from_array(&env, [new_sol_oracle_price, new_usdc_oracle_price]);
//         setup.oracle_client.set_price(&updated_prices, &current_time);

//         // Swap
//         let will_swap = random_bool(&env);
//         if will_swap {
//             let user = &users[i % 10];

//             let user_token1_balance = setup.token1.balance(&user);
//             let user_token2_balance = setup.token2.balance(&user);

//             let (in_idx, out_idx, in_amount) = if user_token1_balance == 0 {
//                 let usdc_amount = random_in_range(&env, 1_0000000, user_token1_balance as u32); // FIXME:
//                 (1, 0, usdc_amount)
//             } else {
//                 let buy: bool = random_bool(&env);

//                 if buy {
//                     (1, 0, random_in_range(&env, 1_0000000, user_token2_balance as u32))
//                 } else {
//                     (0, 1, random_in_range(&env, 1_0000000, user_token1_balance as u32))
//                 }
//             };

//             liq_pool.swap(
//                 user,
//                 &in_idx,
//                 &out_idx,
//                 &(in_amount as u128),
//                 &0
//             );
//         }

//         if current_time % rebase_interval == 0 {
//             liq_pool.rebase(&admin);
//         }

//         // Increment time by 1 second
//         jump(&env, 1);
//     }

//     jump(&env, 100);
//     env.cost_estimate().budget().reset_default();

//     env.cost_estimate().budget().print();
// }

// #[test]
// fn test_swaps_one_day() {
//     test_swaps_many_users(TWENTY_FOUR_HOUR);
// }

// #[cfg(feature = "slow_tests")]
// #[test]
// fn test_swaps_two_weeks() {
//     test_swaps_many_users(THIRTEEN_DAY);
// }
