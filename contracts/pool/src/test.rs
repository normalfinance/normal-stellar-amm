#![cfg(test)]
extern crate std;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use soroban_sdk::symbol_short;
use utils::constant::{PRICE_PRECISION, PRICE_PRECISION_I128};
use utils::state::pool::SwapDirection;

use crate::testutils::{create_pool_contract, Setup, TestConfig};
use access_control::constants::ADMIN_ACTIONS_DELAY;
use soroban_fixed_point_math::FixedPoint;
use soroban_sdk::testutils::{AuthorizedFunction, AuthorizedInvocation, Events};
use soroban_sdk::token::{
    StellarAssetClient as SorobanTokenAdminClient, TokenClient as SorobanTokenClient,
};
use soroban_sdk::{
    testutils::Address as _, vec, Address, Env, Error, IntoVal, String, Symbol, Val, Vec,
};
use token_lp::Client as LpTokenClient;
use utils::state::{
    access::PrivilegedAddresses,
    pool::{InitializeAllParams, InitializeParams, PoolTier, RewardConfig},
    token::TokenInitInfo,
};
// use utils::test_utils::{
//     assert_approx_eq_abs, create_token_contract, get_mock_lp_token_info, get_token_admin_client,
//     install_dummy_wasm, install_token_wasm, jump,
// };

#[test]
fn test() {
    let setup = Setup::new_with_config(
        &(TestConfig {
            mint_to_user: i128::MAX,
            users_count: 3,
            ..TestConfig::default()
        }),
    );
    let e = setup.env;
    let admin = setup.admin;
    let liq_pool = setup.liq_pool;
    let token1 = setup.token1;
    let token2 = setup.token2;
    let token_reward = setup.token_reward;
    let token_share = setup.token_share;
    let user1 = setup.users[1].clone();
    let user2 = setup.users[2].clone();
    let amount_to_deposit = 100_0000000; // 100.00

    // let btc_price = setup.oracle_client.lastprice(&setup.btc_asset).unwrap();
    // let xlm_price = setup.oracle_client.lastprice(&setup.xlm_asset).unwrap();

    // let target_price = xlm_price
    //     .price
    //     .fixed_div_floor(btc_price.price, PRICE_PRECISION_I128)
    //     .unwrap();

    // let expected_mint_amount = (amount_to_deposit as i128)
    //     .fixed_div_floor(target_price, PRICE_PRECISION_I128)
    //     .unwrap();

    let (x, y) = liq_pool.deposit(&admin, &10_000_0000000);

    assert_eq!(e.auths(), []);
    // fn print_invocation(inv: &AuthorizedInvocation, indent: usize) {
    //     let padding = "  ".repeat(indent);
    //     match &inv.function {
    //         AuthorizedFunction::Contract((addr, func, args)) => {
    //             std::println!("{padding}Contract: {addr}");
    //             std::println!("{padding}Function: {func}");
    //             std::println!("{padding}Args: {:?}", args);
    //         }
    //         _ => std::println!("{padding}Other invocation"),
    //     }
    //     for sub in &inv.sub_invocations {
    //         print_invocation(sub, indent + 1);
    //     }
    // }

    // // Print all auths triggered
    // for auth in e.auths() {
    //     std::println!("Authorized by: {}", auth.0);
    //     print_invocation(&auth.1, 0);
    // }

    // let reserves = liq_pool.get_reserves();

    // assert_eq!(liq_pool.get_total_shares(), amount_to_deposit);
    // // assert_eq!(reserves.get(0).unwrap(), 0_3570000);
    // assert_eq!(reserves.get(1).unwrap(), amount_to_deposit);

    // let swapped = liq_pool.swap(&user1, &SwapDirection::Buy, &10_0000000, &0);

    // assert_eq!(token1.balance(&user1), 0_221164);
    // // assert_eq!(swapped, 10);

    // let new_reserves = liq_pool.get_reserves();

    // // assert_eq!(liq_pool.get_total_shares(), amount_to_deposit);
    // // assert_eq!(new_reserves.get(0).unwrap(), 0_3570000);
    // // assert_eq!(new_reserves.get(1).unwrap(), amount_to_deposit);

    // liq_pool.deposit(&user2, &1_000_0000000);

    // liq_pool.swap(&user1, &SwapDirection::Sell, &0_100000, &0);

    // liq_pool.withdraw(&user2, &500_0000000);
}

// #[test]
// fn test() {
//     let setup = Setup::new_with_config(
//         &(TestConfig {
//             mint_to_user: i128::MAX,
//             users_count: 2,
//             ..TestConfig::default()
//         }),
//     );
//     let e = setup.env;
//     let admin = setup.admin;
//     let liq_pool = setup.liq_pool;
//     let token1 = setup.token1;
//     let token2 = setup.token2;
//     let token_reward = setup.token_reward;
//     let token_share = setup.token_share;
//     let user1 = setup.users[1].clone();
//     let reward_1_tps = 10_5000000_u128;
//     let reward_2_tps = 20_0000000_u128;
//     let reward_3_tps = 6_0000000_u128;
//     let total_reward_1 = reward_1_tps * 60;
//     let amount_to_deposit = 100_0000000; // 100.00

//     let btc_price = setup.oracle_client.lastprice(&setup.btc_asset).unwrap();
//     let xlm_price = setup.oracle_client.lastprice(&setup.xlm_asset).unwrap();

//     let target_price = xlm_price
//         .price
//         .fixed_div_floor(btc_price.price, PRICE_PRECISION_I128)
//         .unwrap();

//     let expected_mint_amount = (amount_to_deposit as i128)
//         .fixed_div_floor(target_price, PRICE_PRECISION_I128)
//         .unwrap();

//     liq_pool.deposit(&admin, &amount_to_deposit);
//     assert_eq!(
//         e.auths()[0],
//         (
//             admin.clone(),
//             AuthorizedInvocation {
//                 function: AuthorizedFunction::Contract((
//                     liq_pool.address.clone(),
//                     Symbol::new(&e, "deposit"),
//                     Vec::from_array(&e, [admin.to_val(), amount_to_deposit.into_val(&e)]),
//                 )),
//                 sub_invocations: std::vec![
//                     AuthorizedInvocation {
//                         function: AuthorizedFunction::Contract((
//                             token2.address.clone(),
//                             Symbol::new(&e, "transfer"),
//                             Vec::from_array(
//                                 &e,
//                                 [
//                                     admin.to_val(),
//                                     liq_pool.address.to_val(),
//                                     (amount_to_deposit as i128).into_val(&e),
//                                 ]
//                             ),
//                         )),
//                         sub_invocations: std::vec![],
//                     } // AuthorizedInvocation {
//                       //     function: AuthorizedFunction::Contract((
//                       //         token1.address.clone(),
//                       //         Symbol::new(&e, "mint"),
//                       //         Vec::from_array(&e, [
//                       //             liq_pool.address.to_val(),
//                       //             (expected_mint_amount as i128).into_val(&e),
//                       //         ]),
//                       //     )),
//                       //     sub_invocations: std::vec![],
//                       // }
//                 ],
//             },
//         )
//     );

//     assert_eq!(token_reward.balance(&admin), 0);
//     // 30 seconds passed, half of the reward is available for the user
//     jump(&e, 30);
//     assert_eq!(liq_pool.claim(&admin).0, total_reward_1 / 2);
//     assert_eq!(token_reward.balance(&admin) as u128, total_reward_1 / 2);
//     // 60 seconds more passed. full reward was available though half already claimed
//     jump(&e, 60);
//     assert_eq!(liq_pool.claim(&admin).0, total_reward_1 / 2);
//     assert_eq!(token_reward.balance(&admin) as u128, total_reward_1);

//     // more rewards added with different configs
//     let total_reward_2 = reward_2_tps * 100;
//     liq_pool.set_incentives_config(
//         &admin,
//         &e.ledger().timestamp().saturating_add(100),
//         &reward_2_tps,
//     );
//     jump(&e, 105);
//     let total_reward_3 = reward_3_tps * 50;
//     liq_pool.set_incentives_config(
//         &admin,
//         &e.ledger().timestamp().saturating_add(50),
//         &reward_3_tps,
//     );
//     jump(&e, 500);
//     // two rewards available for the user
//     assert_eq!(liq_pool.claim(&admin).0, total_reward_2 + total_reward_3);
//     assert_eq!(
//         token_reward.balance(&admin) as u128,
//         total_reward_1 + total_reward_2 + total_reward_3
//     );

//     // nBTC
//     assert_eq!(token1.balance(&admin), 0);
//     assert_eq!(
//         token1.balance(&liq_pool.address),
//         expected_mint_amount as i128
//     );

//     // USDC
//     assert_eq!(
//         token2.balance(&admin),
//         i128::MAX - (amount_to_deposit as i128)
//     );
//     assert_eq!(token2.balance(&liq_pool.address), amount_to_deposit as i128);

//     // LP token
//     let expected_share_amount = 100_0000000; // 1:1 with liquidity deposit

//     assert_eq!(token_share.balance(&admin), expected_share_amount);
//     assert_eq!(token_share.balance(&liq_pool.address), 0);

//     let swap_in_amount: u128 = 1_0000000_u128;
//     let expected_swap_result = 39485148_u128;
//     let expected_mint_amount = 0_i128;

//     // selling quote for base (1 > 0)
//     // expected = (1*4) / (100+1) - ((1*4)/(100+1) * (30 / 10_000))
//     assert_eq!(
//         liq_pool.estimate_swap(&1, &0, &swap_in_amount),
//         (expected_swap_result, expected_mint_amount,)
//     );
//     assert_eq!(
//         liq_pool.swap(&user1, &1, &0, &swap_in_amount, &expected_swap_result),
//         expected_swap_result
//     );

//     let expected_new_mint = 79485148_u128;

//     // assert_eq!(e.auths()[1], (
//     //     user1.clone(),
//     //     AuthorizedInvocation {
//     //         function: AuthorizedFunction::Contract((
//     //             liq_pool.address.clone(),
//     //             Symbol::new(&e, "swap"),
//     //             (&user1, 1_u32, 0_u32, swap_in_amount, expected_swap_result).into_val(&e),
//     //         )),
//     //         sub_invocations: std::vec![
//     //             AuthorizedInvocation {
//     //                 function: AuthorizedFunction::Contract((
//     //                     token2.address.clone(),
//     //                     Symbol::new(&e, "transfer"),
//     //                     Vec::from_array(&e, [
//     //                         user1.to_val(),
//     //                         liq_pool.address.to_val(),
//     //                         (swap_in_amount as i128).into_val(&e),
//     //                     ]),
//     //                 )),
//     //                 sub_invocations: std::vec![],
//     //             },
//     //             AuthorizedInvocation {
//     //                 function: AuthorizedFunction::Contract((
//     //                     token1.address.clone(),
//     //                     Symbol::new(&e, "mint"),
//     //                     Vec::from_array(&e, [
//     //                         liq_pool.address.to_val(),
//     //                         (expected_new_mint as i128).into_val(&e),
//     //                     ]),
//     //                 )),
//     //                 sub_invocations: std::vec![],
//     //             }
//     //         ],
//     //     },
//     // ));

//     // User should have received the expected_swap_result
//     assert_eq!(token1.balance(&user1), expected_swap_result as i128);

//     // Pool should have the initial mint amount, less the swap output, plus the newly minted tokens to balance price
//     assert_eq!(
//         token1.balance(&liq_pool.address),
//         (expected_mint_amount as i128) - (expected_swap_result as i128)
//             + (expected_new_mint as i128)
//     );

//     // assert_eq!(
//     //     token2.balance(&user1),
//     //     i128::MAX - (amount_to_deposit as i128) + (expected_swap_result as i128)
//     // );

//     // User should have initial quote token less what they swapped in
//     assert_eq!(token2.balance(&user1), i128::MAX - (swap_in_amount as i128));
//     // Pool should have initial deposit plus swapped in amount
//     assert_eq!(
//         token2.balance(&liq_pool.address),
//         (amount_to_deposit as i128) + (swap_in_amount as i128)
//     );

//     liq_pool.withdraw(&admin, &(expected_share_amount as u128));
//     assert_eq!(
//         e.auths()[0],
//         (
//             admin.clone(),
//             AuthorizedInvocation {
//                 function: AuthorizedFunction::Contract((
//                     liq_pool.address.clone(),
//                     Symbol::new(&e, "withdraw"),
//                     Vec::from_array(
//                         &e,
//                         [
//                             admin.clone().into_val(&e),
//                             (expected_share_amount as u128).into_val(&e),
//                         ]
//                     ),
//                 )),
//                 sub_invocations: std::vec![AuthorizedInvocation {
//                     function: AuthorizedFunction::Contract((
//                         token_share.address.clone(),
//                         Symbol::new(&e, "burn"),
//                         Vec::from_array(
//                             &e,
//                             [admin.to_val(), (amount_to_deposit as i128).into_val(&e)]
//                         ),
//                     )),
//                     sub_invocations: std::vec![],
//                 }],
//             },
//         )
//     );
//     // TODO: how do we assert the pool burned token1

//     jump(&e, 600);
//     assert_eq!(liq_pool.claim(&admin).0, 0);
//     assert_eq!(
//         token_reward.balance(&admin) as u128,
//         total_reward_1 + total_reward_2 + total_reward_3
//     );

//     assert_eq!(token1.balance(&admin), 0);
//     assert_eq!(token2.balance(&admin), i128::MAX);
//     assert_eq!(token_share.balance(&admin), 0);
//     assert_eq!(token1.balance(&liq_pool.address), 5000000); // 0.5
//     assert_eq!(token2.balance(&liq_pool.address), 1_0000000); // 1.0
//     assert_eq!(token_share.balance(&liq_pool.address), 0);
// }

// #[test]
// fn test_strict_receive() {
//     let setup = Setup::new_with_config(
//         &(TestConfig {
//             mint_to_user: i128::MAX,
//             ..TestConfig::default()
//         }),
//     );
//     let user1 = setup.users[0].clone();
//     let desired_amount = 100_0000000;
//     setup.liq_pool.deposit(&user1, &desired_amount);
//     let swap_in_amount = 1_0000000_u128;
//     let expected_swap_result = 4935643_u128;
//     //
//     let expected_mint_amount = 0_i128; // TOOD:

//     assert_eq!(
//         setup.liq_pool.estimate_swap(&1, &0, &swap_in_amount),
//         (expected_swap_result, expected_mint_amount,)
//     );
//     assert_eq!(
//         setup
//             .liq_pool
//             .estimate_swap_strict_receive(&1, &0, &expected_swap_result),
//         (swap_in_amount, 0_i128,)
//     );
//     assert_eq!(
//         setup
//             .liq_pool
//             .swap_strict_receive(&user1, &1, &0, &expected_swap_result, &swap_in_amount),
//         swap_in_amount
//     );
// }

// #[test]
// fn test_strict_receive_over_max() {
//     let setup = Setup::new_with_config(
//         &(TestConfig {
//             mint_to_user: i128::MAX,
//             ..TestConfig::default()
//         }),
//     );
//     let user1 = setup.users[0].clone();
//     let desired_amount = 100_0000000;
//     setup.liq_pool.deposit(&user1, &desired_amount);

//     assert!(setup
//         .liq_pool
//         .try_estimate_swap_strict_receive(&1, &0, &100_0000000)
//         .is_err());
//     assert!(setup
//         .liq_pool
//         .try_swap_strict_receive(&user1, &1, &0, &100_0000000, &100_0000000)
//         .is_err());
//     assert!(setup
//         .liq_pool
//         .try_estimate_swap_strict_receive(&1, &0, &4935643)
//         .is_err()); // 99_7000000
//     assert!(
//         setup
//             .liq_pool
//             .try_swap_strict_receive(&user1, &1, &0, &4935643, &100_0000000)
//             .is_err() // 99_7000000
//     );
//     // maximum we're able to buy is `reserve * (1 - fee) - delta`
//     assert_eq!(
//         setup
//             .liq_pool
//             .estimate_swap_strict_receive(&1, &0, &99_6999999),
//         (99999999900_0000001, 0)
//     );
//     assert_eq!(
//         setup
//             .liq_pool
//             .swap_strict_receive(&user1, &1, &0, &99_6999999, &99999999900_0000001),
//         99999999900_0000001
//     );
// }

// #[test]
// fn test_events() {
//     let setup = Setup::new_with_config(
//         &(TestConfig {
//             mint_to_user: i128::MAX,
//             ..TestConfig::default()
//         }),
//     );
//     let e = setup.env;
//     let liq_pool = setup.liq_pool;
//     let token1 = setup.token1;
//     let token2 = setup.token2;
//     let user1 = setup.users[0].clone();
//     let amount_to_deposit = 100_0000000;

//     liq_pool.deposit(&user1, &amount_to_deposit);
//     assert_eq!(
//         vec![&e, e.events().all().last().unwrap()],
//         vec![
//             &e,
//             (
//                 liq_pool.address.clone(),
//                 (Symbol::new(&e, "deposit_liquidity"), token2.address.clone()).into_val(&e),
//                 (amount_to_deposit as i128, amount_to_deposit as i128).into_val(&e),
//             )
//         ]
//     );

//     assert_eq!(liq_pool.swap(&user1, &1, &0, &1, &0), 2);
//     assert_eq!(
//         vec![&e, e.events().all().last().unwrap()],
//         vec![
//             &e,
//             (
//                 liq_pool.address.clone(),
//                 (
//                     Symbol::new(&e, "trade"),
//                     token2.address.clone(),
//                     token1.address.clone(),
//                     user1.clone(),
//                 )
//                     .into_val(&e),
//                 (1_i128, 39485148_i128, 2_i128).into_val(&e),
//             )
//         ]
//     );

//     let amount_out = liq_pool.withdraw(&user1, &amount_to_deposit);
//     assert_eq!(amount_out, 1000000100);
//     assert_eq!(
//         vec![&e, e.events().all().last().unwrap()],
//         vec![
//             &e,
//             (
//                 liq_pool.address.clone(),
//                 (
//                     Symbol::new(&e, "withdraw_liquidity"),
//                     token2.address.clone()
//                 )
//                     .into_val(&e),
//                 (amount_to_deposit as i128, amount_out as i128).into_val(&e),
//             )
//         ]
//     );
// }

// #[test]
// #[should_panic(expected = "Error(Contract, #2004)")]
// fn test_zero_initial_deposit() {
//     let setup = Setup::default();
//     let user1 = setup.users[0].clone();
//     setup.liq_pool.deposit(&user1, &0);
// }

// #[test]
// fn test_zero_deposit_ok() {
//     let setup = Setup::default();
//     let liq_pool = setup.liq_pool;
//     let user1 = setup.users[0].clone();
//     liq_pool.deposit(&user1, &100);
//     liq_pool.deposit(&user1, &0);
// }

// #[test]
// #[should_panic(expected = "Error(Contract, #201)")]
// fn initialize_already_initialized() {
//     let setup = Setup::default();

//     let users = Setup::generate_random_users(&setup.env, 3);
//     let token1 = create_token_contract(&setup.env, &users[1]);
//     let token2 = create_token_contract(&setup.env, &users[2]);

//     let params = InitializeParams {
//         admin: users[0].clone(),
//         privileged_addrs: PrivilegedAddresses {
//             emergency_admin: users[0].clone(),
//             rewards_admin: users[0].clone(),
//             operations_admin: users[0].clone(),
//             pause_admin: users[0].clone(),
//             emergency_pause_admins: Vec::from_array(&setup.env, [users[0].clone()]),
//         },
//         router: users[0].clone(),
//         assets: (setup.btc_asset_id, setup.xlm_asset_id),
//         tokens: Vec::from_array(&setup.env, [token1.address.clone(), token2.address.clone()]),
//         lp_token_info: TokenInitInfo {
//             token_wasm_hash: install_token_wasm(&setup.env),
//             name: String::from_str(&setup.env, "Pool Share Token"),
//             symbol: String::from_str(&setup.env, "Pool Share Token"),
//         },
//         fee_fraction: 10_u32,
//         tier: PoolTier::A,
//         quote_max_insurance: 1_000_000_u128,
//     };

//     setup.liq_pool.initialize(&params);
// }

// #[test]
// #[should_panic(expected = "Error(Contract, #202)")]
// fn initialize_already_initialized_plane() {
//     let setup = Setup::default();

//     let users = Setup::generate_random_users(&setup.env, 3);
//     let token1 = create_token_contract(&setup.env, &users[1]);
//     let token2 = create_token_contract(&setup.env, &users[2]);

//     let params = InitializeAllParams {
//         base: InitializeParams {
//             admin: users[0].clone(),
//             privileged_addrs: PrivilegedAddresses {
//                 emergency_admin: users[0].clone(),
//                 rewards_admin: users[0].clone(),
//                 operations_admin: users[0].clone(),
//                 pause_admin: users[0].clone(),
//                 emergency_pause_admins: Vec::from_array(&setup.env, [users[0].clone()]),
//             },
//             router: users[0].clone(),
//             assets: (setup.btc_asset_id.clone(), setup.xlm_asset_id.clone()),
//             tokens: Vec::from_array(&setup.env, [token1.address.clone(), token2.address.clone()]),
//             lp_token_info: TokenInitInfo {
//                 token_wasm_hash: install_token_wasm(&setup.env),
//                 name: String::from_str(&setup.env, "Pool Share Token"),
//                 symbol: String::from_str(&setup.env, "Pool Share Token"),
//             },
//             fee_fraction: 10_u32,
//             tier: PoolTier::A,
//             quote_max_insurance: 1_000_000_u128,
//         },
//         reward_config: RewardConfig {
//             reward_token: setup.token_reward.address,
//         },
//         plane: setup.plane.address,
//     };

//     setup.liq_pool.initialize_all(&params);
// }

// #[test]
// fn test_custom_fee() {
//     let config = TestConfig {
//         mint_to_user: 1000000_0000000,
//         ..TestConfig::default()
//     };
//     let setup = Setup::new_with_config(&config);

//     // we're checking fraction against output for 1 token
//     // result = (1*4)/(100+1)
//     // 0.00495049505
//     // fee = 0.0396039604 * (10 / 10_000)
//     for fee_config in [
//         (0, 396039_u128, 0_i128),    // 0%
//         (10, 395643_u128, 0_i128),   // 0.1% 0.03956435644
//         (30, 394851_u128, 0_i128),   // 0.3% 0.03948514852
//         (100, 392079_u128, 0_i128),  // 1% 0.0392079208
//         (1000, 356435_u128, 0_i128), // 10% 0.03564356436
//         (3000, 356435_u128, 0_i128), // 30% 0.03564356436
//         (5000, 198019_u128, 0_i128), // 50%  0.0198019802
//     ] {
//         let pool = create_pool_contract(
//             &setup.env,
//             &setup.admin,
//             &setup.plane.address,
//             &setup.router.address,
//             &(setup.btc_asset_id.clone(), setup.xlm_asset_id.clone()),
//             &install_token_wasm(&setup.env),
//             &get_mock_lp_token_info(&setup.env),
//             &Vec::from_array(
//                 &setup.env,
//                 [setup.token1.address.clone(), setup.token2.address.clone()],
//             ),
//             &setup.token_reward.address,
//             fee_config.0, // ten percent
//             &PoolTier::A,
//             1_000_000_u128,
//         );
//         pool.deposit(&setup.users[0], &100_0000000);
//         assert_eq!(pool.estimate_swap(&1, &0, &1_0000000), (fee_config.1, 0));
//         assert_eq!(
//             pool.swap(&setup.users[0], &1, &0, &1_0000000, &0),
//             fee_config.1
//         );

//         // FIXME:
//         // full withdraw & deposit to reset pool reserves
//         pool.withdraw(
//             &setup.users[0],
//             &(SorobanTokenClient::new(&setup.env, &pool.share_id()).balance(&setup.users[0])
//                 as u128),
//         );
//         pool.deposit(&setup.users[0], &100_0000000);
//         assert_eq!(
//             pool.estimate_swap(&1, &0, &1_0000000),
//             (fee_config.1, fee_config.2)
//         ); // re-check swap result didn't change
//         assert_eq!(
//             pool.estimate_swap_strict_receive(&1, &0, &fee_config.1),
//             (1_0000000, 0)
//         );
//         assert_eq!(
//             pool.swap_strict_receive(&setup.users[0], &1, &0, &fee_config.1, &1_0000000),
//             1_0000000
//         );
//     }
// }

// #[test]
// fn test_simple_ongoing_reward() {
//     let setup = Setup::default();
//     let env = setup.env;
//     let liq_pool = setup.liq_pool;
//     let token_reward = setup.token_reward;
//     let users = setup.users;
//     let total_reward_1 = TestConfig::default().reward_tps * 60;
//     assert_eq!(liq_pool.get_total_configured_reward(), total_reward_1);
//     assert_eq!(liq_pool.get_total_accumulated_reward(), 0);
//     assert_eq!(liq_pool.get_total_claimed_reward(), 0);

//     // 10 seconds passed since config, user depositing
//     jump(&env, 10);

//     assert_eq!(liq_pool.get_total_configured_reward(), total_reward_1);
//     assert_eq!(
//         liq_pool.get_total_accumulated_reward(),
//         TestConfig::default().reward_tps * 10
//     );
//     assert_eq!(liq_pool.get_total_claimed_reward(), 0);

//     liq_pool.deposit(&users[0], &100);

//     assert_eq!(token_reward.balance(&users[0]), 0);
//     // 30 seconds passed, half of the reward is available for the user
//     jump(&env, 30);

//     assert_eq!(liq_pool.get_total_configured_reward(), total_reward_1);
//     assert_eq!(
//         liq_pool.get_total_accumulated_reward(),
//         TestConfig::default().reward_tps * 40
//     );
//     assert_eq!(liq_pool.get_total_claimed_reward(), 0);

//     assert_eq!(liq_pool.claim(&users[0]).0, total_reward_1 / 2);
//     assert_eq!(token_reward.balance(&users[0]) as u128, total_reward_1 / 2);

//     assert_eq!(liq_pool.get_total_configured_reward(), total_reward_1);
//     assert_eq!(
//         liq_pool.get_total_accumulated_reward(),
//         TestConfig::default().reward_tps * 40
//     );
//     assert_eq!(
//         liq_pool.get_total_claimed_reward(),
//         TestConfig::default().reward_tps * 30
//     );

//     // 40 seconds passed, reward config ended
//     //  5/6 of the reward is available for the user since he has missed first 10 seconds
//     jump(&env, 40);

//     assert_eq!(liq_pool.get_total_configured_reward(), total_reward_1);
//     assert_eq!(liq_pool.get_total_accumulated_reward(), total_reward_1);
//     assert_eq!(
//         liq_pool.get_total_claimed_reward(),
//         TestConfig::default().reward_tps * 30
//     );

//     assert_eq!(liq_pool.claim(&users[0]).0, (total_reward_1 * 2) / 6);
//     assert_eq!(
//         token_reward.balance(&users[0]) as u128,
//         (total_reward_1 * 5) / 6
//     );

//     assert_eq!(liq_pool.get_total_configured_reward(), total_reward_1);
//     assert_eq!(liq_pool.get_total_accumulated_reward(), total_reward_1);
//     assert_eq!(
//         liq_pool.get_total_claimed_reward(),
//         TestConfig::default().reward_tps * 50
//     );
// }

// #[test]
// fn test_estimate_ongoing_reward() {
//     let setup = Setup::default();
//     let env = setup.env;
//     let liq_pool = setup.liq_pool;
//     let token_reward = setup.token_reward;
//     let users = setup.users;

//     // 10 seconds passed since config, user depositing
//     jump(&env, 10);
//     liq_pool.deposit(&users[0], &100);

//     assert_eq!(token_reward.balance(&users[0]), 0);
//     // 30 seconds passed, half of the reward is available for the user
//     jump(&env, 30);
//     let total_reward_1 = TestConfig::default().reward_tps * 60;
//     assert_eq!(liq_pool.get_user_reward(&users[0]), total_reward_1 / 2);
//     assert_eq!(token_reward.balance(&users[0]) as u128, 0);
// }

// #[test]
// fn test_simple_reward() {
//     let setup = Setup::setup(&TestConfig::default());
//     setup.mint_tokens_for_users(TestConfig::default().mint_to_user);
//     let env = setup.env;
//     let liq_pool = setup.liq_pool;
//     let token_reward = setup.token_reward;
//     let users = setup.users;

//     // 10 seconds. user depositing
//     jump(&env, 10);
//     liq_pool.deposit(&users[0], &100);

//     // 20 seconds. rewards set up for 60 seconds
//     jump(&env, 10);
//     let reward_1_tps = 10_5000000_u128;
//     let total_reward_1 = reward_1_tps * 60;
//     liq_pool.set_incentives_config(
//         &users[0],
//         &env.ledger().timestamp().saturating_add(60),
//         &reward_1_tps,
//     );

//     // 90 seconds. rewards ended.
//     jump(&env, 70);
//     // calling set rewards config to checkpoint. should be removed
//     liq_pool.set_incentives_config(
//         &users[0],
//         &env.ledger().timestamp().saturating_add(60),
//         &0_u128,
//     );

//     // 100 seconds. user claim reward
//     jump(&env, 10);
//     assert_eq!(token_reward.balance(&users[0]), 0);
//     // full reward should be available to the user
//     assert_eq!(liq_pool.claim(&users[0]).0, total_reward_1);
//     assert_eq!(token_reward.balance(&users[0]) as u128, total_reward_1);
// }

// #[test]
// fn test_two_users_rewards() {
//     let setup = Setup::default();
//     let env = setup.env;
//     let liq_pool = setup.liq_pool;
//     let token_reward = setup.token_reward;
//     let users = setup.users;

//     let total_reward_1 = &TestConfig::default().reward_tps * 60;

//     // two users make deposit for equal value. second after 30 seconds after rewards start,
//     //  so it gets only 1/4 of total reward
//     liq_pool.deposit(&users[0], &100);
//     jump(&env, 30);
//     assert_eq!(liq_pool.claim(&users[0]).0, total_reward_1 / 2);
//     liq_pool.deposit(&users[1], &100);
//     jump(&env, 100);
//     assert_eq!(liq_pool.claim(&users[0]).0, total_reward_1 / 4);
//     assert_eq!(liq_pool.claim(&users[1]).0, total_reward_1 / 4);
//     assert_eq!(
//         token_reward.balance(&users[0]) as u128,
//         (total_reward_1 / 4) * 3
//     );
//     assert_eq!(token_reward.balance(&users[1]) as u128, total_reward_1 / 4);
// }

// #[test]
// fn test_lazy_user_rewards() {
//     let setup = Setup::default();
//     let env = setup.env;
//     let liq_pool = setup.liq_pool;
//     let token_reward = setup.token_reward;
//     let users = setup.users;

//     let total_reward_1 = &TestConfig::default().reward_tps * 60;

//     liq_pool.deposit(&users[0], &100);
//     jump(&env, 59);
//     liq_pool.deposit(&users[1], &1000);
//     jump(&env, 100);
//     let user1_claim = liq_pool.claim(&users[0]).0;
//     let user2_claim = liq_pool.claim(&users[1]).0;
//     assert_approx_eq_abs(
//         user1_claim,
//         (total_reward_1 * 59) / 60 + ((total_reward_1 / 1100) * 100) / 60,
//         1000,
//     );
//     assert_approx_eq_abs(user2_claim, ((total_reward_1 / 1100) * 1000) / 60, 1000);
//     assert_approx_eq_abs(token_reward.balance(&users[0]) as u128, user1_claim, 1000);
//     assert_approx_eq_abs(token_reward.balance(&users[1]) as u128, user2_claim, 1000);
//     assert_approx_eq_abs(user1_claim + user2_claim, total_reward_1, 1000);
// }

// #[test]
// fn test_rewards_disable_before_expiration() {
//     let setup = Setup::new_with_config(
//         &(TestConfig {
//             users_count: 3,
//             reward_tps: 0,
//             ..TestConfig::default()
//         }),
//     );
//     let env = setup.env;
//     let liq_pool = setup.liq_pool;
//     let users = setup.users;

//     // user 1 has 10% of total reward
//     liq_pool.deposit(&users[0], &900);
//     liq_pool.deposit(&users[1], &100);

//     jump(&env, 10);
//     let admin = users[0].clone();
//     let tps = 1_0000000;
//     // admin sets rewards distribution a bit in the future from the expected point
//     liq_pool.set_incentives_config(&admin, &env.ledger().timestamp().saturating_add(100), &tps);

//     // user 2 enters. now user 1 gets 5% of total reward, user 2 receives 50%
//     jump(&env, 20);
//     liq_pool.deposit(&users[2], &1000);

//     jump(&env, 10);
//     liq_pool.withdraw(&users[2], &1000);

//     // before config expiration, admin decides to stop as it's time to reward other pools
//     jump(&env, 50);
//     liq_pool.set_incentives_config(&admin, &env.ledger().timestamp().saturating_add(10), &0);

//     // user decides to claim in far future
//     jump(&env, 1000);
//     assert_eq!(
//         liq_pool.claim(&users[1]).0,
//         (tps * 20) / 10 + (tps * 10) / 20 + (tps * 50) / 10
//     );
//     assert_eq!(liq_pool.claim(&users[2]).0, (tps * 10) / 2);
// }

// #[test]
// fn test_rewards_disable_after_expiration() {
//     let setup = Setup::new_with_config(
//         &(TestConfig {
//             reward_tps: 0,
//             ..TestConfig::default()
//         }),
//     );
//     let env = setup.env;
//     let liq_pool = setup.liq_pool;
//     let users = setup.users;

//     // user 1 has 10% of total reward
//     liq_pool.deposit(&users[0], &900);
//     liq_pool.deposit(&users[1], &100);

//     jump(&env, 10);
//     let admin = users[0].clone();
//     let tps = 1_0000000;
//     // admin sets rewards distribution, then decides to stop rewards after expiration
//     liq_pool.set_incentives_config(&admin, &env.ledger().timestamp().saturating_add(100), &tps);
//     jump(&env, 150);
//     liq_pool.set_incentives_config(&admin, &env.ledger().timestamp().saturating_add(100), &0);

//     // user decides to claim in far future
//     jump(&env, 1000);
//     assert_eq!(liq_pool.claim(&users[1]).0, (tps * 100) / 10);
// }

// #[test]
// fn test_rewards_set_new_after_expiration() {
//     let setup = Setup::new_with_config(
//         &(TestConfig {
//             reward_tps: 0,
//             ..TestConfig::default()
//         }),
//     );
//     let env = setup.env;
//     let liq_pool = setup.liq_pool;
//     let users = setup.users;

//     // user 1 has 10% of total reward
//     liq_pool.deposit(&users[0], &900);
//     liq_pool.deposit(&users[1], &100);

//     jump(&env, 10);
//     let admin = users[0].clone();
//     let tps_1 = 1_0000000;
//     let tps_2 = 10000;
//     // admin configures first rewards distribution, then it ends and admin sets new one which also expires
//     liq_pool.set_incentives_config(
//         &admin,
//         &env.ledger().timestamp().saturating_add(100),
//         &tps_1,
//     );
//     jump(&env, 150);
//     liq_pool.set_incentives_config(
//         &admin,
//         &env.ledger().timestamp().saturating_add(100),
//         &tps_2,
//     );

//     // user decides to claim in far future
//     jump(&env, 1000);
//     assert_eq!(
//         liq_pool.claim(&users[1]).0,
//         (tps_1 * 100) / 10 + (tps_2 * 100) / 10
//     );
// }

// #[test]
// #[should_panic(expected = "Error(Contract, #702)")]
// fn test_rewards_same_expiration_time() {
//     let setup = Setup::default();
//     let env = setup.env;
//     let liq_pool = setup.liq_pool;
//     let users = setup.users;

//     jump(&env, 10);
//     liq_pool.set_incentives_config(&users[0], &env.ledger().timestamp().saturating_add(100), &1);
//     jump(&env, 10);
//     liq_pool.set_incentives_config(&users[0], &env.ledger().timestamp().saturating_add(90), &2);
// }

// #[test]
// #[should_panic(expected = "Error(Contract, #701)")]
// fn test_rewards_past() {
//     let setup = Setup::default();
//     let env = setup.env;
//     let liq_pool = setup.liq_pool;
//     let users = setup.users;

//     jump(&env, 10);
//     let original_expiration_time = env.ledger().timestamp().saturating_add(100);
//     liq_pool.set_incentives_config(&users[0], &original_expiration_time, &1);
//     jump(&env, 1000);
//     liq_pool.set_incentives_config(&users[0], &original_expiration_time.saturating_add(90), &2);
// }

// fn test_rewards_many_users(iterations_to_simulate: u32) {
//     // first user comes as initial liquidity provider
//     //  many users come
//     //  user does withdraw

//     let setup = Setup::new_with_config(
//         &(TestConfig {
//             users_count: 100,
//             ..TestConfig::default()
//         }),
//     );
//     let env = setup.env;
//     let liq_pool = setup.liq_pool;
//     let users = setup.users;
//     let token2_admin_client = setup.token2_admin_client;
//     let token_reward_admin_client = setup.token_reward_admin_client;

//     let admin = users[0].clone();
//     let first_user = Address::generate(&env);

//     for i in 0..101 {
//         let user = match i {
//             0 => &first_user,
//             val => &users[val - 1],
//         };
//         token2_admin_client.mint(user, &1_000_000_000_000_000_000_000);
//     }

//     token_reward_admin_client.mint(&liq_pool.address, &1_000_000_000_000_0000000);

//     let reward_1_tps = 10_5000000_u128;
//     liq_pool.set_incentives_config(
//         &admin,
//         &env.ledger()
//             .timestamp()
//             .saturating_add((iterations_to_simulate * 2 + 110).into()),
//         &reward_1_tps,
//     );
//     jump(&env, 10);

//     // we have this because of last jump(100)
//     let mut expected_reward = (100 * reward_1_tps) / (iterations_to_simulate as u128);
//     for i in 0..iterations_to_simulate as u128 {
//         expected_reward += reward_1_tps / (i + 1);
//     }

//     liq_pool.deposit(&first_user, &1_000_000_000_000_0000000);
//     jump(&env, 1);

//     for i in 1..iterations_to_simulate as usize {
//         let user = &users[i % 10];
//         liq_pool.deposit(user, &1_000_000_000_000_0000000);
//         jump(&env, 1);
//     }

//     jump(&env, 100);
//     env.cost_estimate().budget().reset_default();
//     let (user1_claim, _fees_owed) = liq_pool.claim(&first_user);
//     env.cost_estimate().budget().print();
//     assert_approx_eq_abs(user1_claim, expected_reward, 10000); // small loss because of rounding is fine
// }

// fn test_swaps_many_users(iterations_to_simulate: u32) {
//     // first user comes as initial liquidity provider
//     //  many users come
//     //  user does withdraw

//     let setup = Setup::new_with_config(
//         &(TestConfig {
//             users_count: 100,
//             ..TestConfig::default()
//         }),
//     );
//     let env = setup.env;
//     let liq_pool = setup.liq_pool;
//     let users = setup.users;
//     let token2_admin_client = setup.token2_admin_client;

//     let admin = users[0].clone();

//     let rng_seed = 42;
//     let mut rng = StdRng::seed_from_u64(rng_seed);

//     // Admin seeds initial liquidity
//     token2_admin_client.mint(&admin, &1_000_000_0000000); // 1 million
//     liq_pool.deposit(&admin, &1_000_000_0000000);

//     // Mint token B to users
//     let first_user = Address::generate(&env);

//     for i in 0..101 {
//         let user = match i {
//             0 => &first_user,
//             val => &users[val - 1],
//         };
//         token2_admin_client.mint(user, &100_000_000_0000000); // 1 million
//     }

//     // Track user token_a balances for accurate sells
//     let mut token_a_balances: Vec<u128> = Vec::new(&env);
//     for _ in 0..users.len() {
//         token_a_balances.push_back(0);
//     }

//     let max_token_a_volatility = 1_000_000_u128; // ±10%
//     let max_token_b_volatility = 1_000_000_u128; // ±10%
//     let max_b = 100_000_000_u128; // 0.1 token B (e.g., USDC)

//     // Test pool price peg
//     let pool_price = 0;

//     let expected_price = setup
//         .init_xlm_price
//         .fixed_div_floor(setup.init_btc_price, PRICE_PRECISION_I128)
//         .unwrap();

//     assert_approx_eq_abs(pool_price, expected_price as u128, 10_000); // allow small rounding tolerance

//     for i in 1..iterations_to_simulate as usize {
//         // Simulate oracle price with volatility
//         let token_a_vol =
//             rng.gen_range(-(max_token_a_volatility as i128)..max_token_a_volatility as i128);

//         let btc_price = setup.oracle_client.lastprice(&setup.btc_asset).unwrap();
//         let new_token_a_price = btc_price.price + token_a_vol;

//         // Update the quote asset price
//         let token_b_vol =
//             rng.gen_range(-(max_token_b_volatility as i128)..max_token_b_volatility as i128);
//         let xlm_price = setup.oracle_client.lastprice(&setup.xlm_asset).unwrap();
//         let new_token_b_price = xlm_price.price + token_b_vol;

//         setup.oracle_client.set_price(
//             &Vec::from_array(&env, [new_token_a_price, new_token_b_price]),
//             &env.ledger().timestamp(),
//         );

//         let user_index = (i as usize) % users.len();
//         let user = &users[user_index];
//         let user_index_u32 = user_index as u32;
//         let action_roll: i32 = rng.gen_range(0..100);

//         let mut check_price = true;

//         if action_roll < 20 {
//             // Deposit more liquidity
//             let amount_to_deposit = rng.gen_range(1..=max_b);
//             liq_pool.deposit(user, &amount_to_deposit);
//         } else if action_roll < 60 {
//             // Buy Token A using Token B (only if enough A in reserve)
//             let reserves = liq_pool.get_reserves();
//             let reserve_a = reserves.get(0).unwrap();

//             if reserve_a > 0 {
//                 let amount_b = rng.gen_range(1..=1_000_000_0000000);
//                 let out_a = liq_pool.swap(user, &1, &0, &amount_b, &0);

//                 let prev_balance = token_a_balances.get(user_index_u32).unwrap_or(0);
//                 token_a_balances.set(user_index_u32, prev_balance + out_a);
//             }
//         } else if action_roll < 90 {
//             // Swap Token A to B (sell)
//             let balance_a = token_a_balances.get(user_index_u32).unwrap_or(0);
//             let max_sell = balance_a.min(100_000_000);

//             if max_sell > 0 {
//                 let sell_amount = rng.gen_range(1..=max_sell);

//                 liq_pool.swap(user, &0, &1, &sell_amount, &0);
//                 token_a_balances.set(user_index_u32, balance_a - sell_amount);
//             } else {
//                 check_price = false;
//             }
//         } else {
//             // Withdraw some liquidity
//             let lp_balance = setup.token_share.balance(user);
//             if lp_balance > 0 {
//                 let max_withdraw = lp_balance;
//                 let withdraw_amount = rng.gen_range(1..=max_withdraw);
//                 liq_pool.withdraw(user, &(withdraw_amount as u128));
//             } else {
//                 check_price = false;
//             }
//         }

//         // Test pool price peg
//         if check_price {
//             let reserves = liq_pool.get_reserves();
//             let pool_price = reserves.get(0).unwrap() / reserves.get(1).unwrap();
//             let expected_price = new_token_b_price
//                 .fixed_div_floor(new_token_a_price, PRICE_PRECISION_I128)
//                 .unwrap();
//             assert_approx_eq_abs(pool_price, expected_price as u128, 10_000); // allow small rounding tolerance
//         }

//         jump(&env, 1);
//     }

//     jump(&env, 100);

//     // // Withdraw every user's liquidity
//     // for i in 0..101 {
//     //     let user = match i {
//     //         0 => &first_user,
//     //         val => &users[val - 1],
//     //     };
//     //     let lp_balance = setup.token_share.balance(user);
//     //     liq_pool.withdraw(user, &(lp_balance as u128));
//     // }

//     // // TODO: reserve expects

//     // // Ensure token_a holders can still sell
//     // for i in 0..101 {
//     //     let user = match i {
//     //         0 => &first_user,
//     //         val => &users[val - 1],
//     //     };

//     //     let amt_a = token_a_balances[i];
//     //     if amt_a > 0 {
//     //         let result = liq_pool.swap(user, &0, &1, &amt_a, &0);
//     //         assert!(result > 0, "User {} could not redeem token A back", i);
//     //     }
//     // }
// }

// #[test]
// fn test_bank_run_many_users() {}

// #[test]
// fn test_large_swap_many_users() {}

// #[test]
// fn test_deposit_inequal_return_change() {
//     let setup = Setup::default();
//     // let e = setup.env;
//     let liq_pool = setup.liq_pool;
//     let token1 = setup.token1;
//     let token2 = setup.token2;
//     let users = setup.users;
//     let user1 = users[0].clone();
//     liq_pool.deposit(&user1, &100);
//     assert_eq!(token1.balance(&liq_pool.address), 400);
//     assert_eq!(token2.balance(&liq_pool.address), 100);
//     liq_pool.deposit(&user1, &100); // 200?
//     assert_eq!(token1.balance(&liq_pool.address), 800);
//     assert_eq!(token2.balance(&liq_pool.address), 200);
// }

// // #[test]
// // fn test_rewards_1k() {
// //     test_rewards_many_users(1_000);
// // }

// // #[cfg(feature = "slow_tests")]
// // #[test]
// // fn test_rewards_50k() {
// //     test_rewards_many_users(50_000);
// // }

// #[test]
// fn test_swaps_1k() {
//     test_swaps_many_users(1_000);
// }

// // #[cfg(feature = "slow_tests")]
// // #[test]
// // fn test_swaps_50k() {
// //     test_swaps_many_users(50_000);
// // }

// #[test]
// #[should_panic(expected = "Error(Contract, #102)")]
// fn test_config_rewards_not_admin() {
//     let setup = Setup::setup(&TestConfig::default());
//     setup.mint_tokens_for_users(TestConfig::default().mint_to_user);
//     let env = setup.env;
//     let liq_pool = setup.liq_pool;
//     let users = setup.users;

//     liq_pool.set_incentives_config(
//         &users[1],
//         &env.ledger().timestamp().saturating_add(60),
//         &10_5000000_u128,
//     );
// }

// #[test]
// fn test_config_rewards_router() {
//     let setup = Setup::setup(&TestConfig::default());
//     setup.mint_tokens_for_users(TestConfig::default().mint_to_user);
//     let env = setup.env;
//     let liq_pool = setup.liq_pool;
//     let router = setup.router;

//     liq_pool.set_incentives_config(
//         &router.address,
//         &env.ledger().timestamp().saturating_add(60),
//         &10_5000000_u128,
//     );
// }

// #[test]
// fn test_config_rewards_override() {
//     let setup = Setup::setup(&TestConfig::default());
//     setup.mint_tokens_for_users(TestConfig::default().mint_to_user);
//     let env = setup.env;
//     let liq_pool = setup.liq_pool;
//     let users = setup.users;
//     let router = setup.router;

//     liq_pool.deposit(&users[1], &100);

//     assert_eq!(liq_pool.get_total_accumulated_reward(), 0);
//     assert_eq!(liq_pool.get_total_configured_reward(), 0);
//     let tps = 10_5000000_u128;
//     liq_pool.set_incentives_config(
//         &router.address,
//         &env.ledger().timestamp().saturating_add(60),
//         &tps,
//     );

//     jump(&env, 30);
//     assert_eq!(liq_pool.get_total_accumulated_reward(), tps * 30);
//     assert_eq!(liq_pool.get_total_configured_reward(), tps * 60);
//     liq_pool.set_incentives_config(
//         &router.address,
//         &env.ledger().timestamp().saturating_add(0),
//         &0,
//     );

//     assert_eq!(liq_pool.get_total_accumulated_reward(), tps * 30);
//     assert_eq!(liq_pool.get_total_configured_reward(), tps * 30);

//     jump(&env, 5);

//     assert_eq!(liq_pool.get_total_accumulated_reward(), tps * 30);
//     assert_eq!(liq_pool.get_total_configured_reward(), tps * 30);
// }

// #[should_panic(expected = "Error(Contract, #2018)")]
// #[test]
// fn test_zero_swap() {
//     let setup = Setup::new_with_config(
//         &(TestConfig {
//             mint_to_user: i128::MAX,
//             ..TestConfig::default()
//         }),
//     );
//     // let e = setup.env;
//     let liq_pool = setup.liq_pool;
//     let users = setup.users;
//     let user1 = users[0].clone();
//     let desired_amount = 1_0000000;

//     liq_pool.deposit(&user1, &desired_amount);
//     liq_pool.swap(&user1, &1, &0, &0, &0);
// }

// #[test]
// fn test_large_numbers() {
//     let setup = Setup::new_with_config(
//         &(TestConfig {
//             mint_to_user: i128::MAX,
//             ..TestConfig::default()
//         }),
//     );
//     let e = setup.env;
//     let liq_pool = setup.liq_pool;
//     let token1 = setup.token1;
//     let token2 = setup.token2;
//     let token_share = setup.token_share;
//     let users = setup.users;
//     let user1 = users[0].clone();
//     let amount_to_deposit = u128::MAX / 1_000_000;
//     let desired_amount = amount_to_deposit;

//     let btc_price = setup.oracle_client.lastprice(&setup.btc_asset).unwrap();
//     let xlm_price = setup.oracle_client.lastprice(&setup.xlm_asset).unwrap();
//     let target_price = xlm_price
//         .price
//         .fixed_div_floor(btc_price.price, PRICE_PRECISION_I128)
//         .unwrap();

//     let expected_mint_amount = amount_to_deposit
//         .fixed_div_floor(target_price as u128, PRICE_PRECISION)
//         .unwrap();

//     liq_pool.deposit(&user1, &desired_amount);

//     // when we deposit equal amounts, we gotta have deposited amount of share tokens
//     let expected_share_amount = amount_to_deposit as i128;
//     assert_eq!(token_share.balance(&user1), expected_share_amount);
//     assert_eq!(token_share.balance(&liq_pool.address), 0);
//     assert_eq!(token1.balance(&user1), 0);
//     assert_eq!(
//         token1.balance(&liq_pool.address),
//         expected_mint_amount as i128
//     );
//     assert_eq!(
//         token2.balance(&user1),
//         i128::MAX - (amount_to_deposit as i128)
//     );
//     assert_eq!(token2.balance(&liq_pool.address), amount_to_deposit as i128);

//     let swap_in = amount_to_deposit / 1_000;
//     // swap out shouldn't differ for more than 0.4% since fee is 0.3%
//     let expected_swap_result_delta = swap_in / 250;
//     let (estimate_swap_result, estimate_mint_result) = liq_pool.estimate_swap(&1, &0, &swap_in);
//     assert_approx_eq_abs(estimate_swap_result, swap_in, expected_swap_result_delta);
//     assert_eq!(
//         liq_pool.swap(&user1, &1, &0, &swap_in, &estimate_swap_result),
//         estimate_swap_result
//     );

//     assert_eq!(token1.balance(&user1), estimate_swap_result as i128);
//     assert_eq!(
//         token1.balance(&liq_pool.address),
//         (expected_mint_amount as i128) - (estimate_swap_result as i128) + 0
//     ); // next mint amount
//     assert_eq!(
//         token2.balance(&user1),
//         i128::MAX - (amount_to_deposit as i128) - (swap_in as i128)
//     );
//     assert_eq!(
//         token2.balance(&liq_pool.address),
//         (amount_to_deposit as i128) + (swap_in as i128)
//     );

//     // let withdraw_amounts = [amount_to_deposit + swap_in, amount_to_deposit - estimate_swap_result];
//     liq_pool.withdraw(&user1, &(expected_share_amount as u128));

//     assert_eq!(token1.balance(&user1), i128::MAX);
//     assert_eq!(token2.balance(&user1), i128::MAX);
//     assert_eq!(token_share.balance(&user1), 0);
//     assert_eq!(token1.balance(&liq_pool.address), 0);
//     assert_eq!(token2.balance(&liq_pool.address), 0);
//     assert_eq!(token_share.balance(&liq_pool.address), 0);
// }

// #[test]
// fn test_swap_killed() {
//     let setup = Setup::new_with_config(
//         &(TestConfig {
//             mint_to_user: i128::MAX,
//             ..TestConfig::default()
//         }),
//     );
//     let e = setup.env;
//     let liq_pool = setup.liq_pool;
//     let users = setup.users;

//     assert_eq!(liq_pool.get_is_killed_deposit(), false);
//     assert_eq!(liq_pool.get_is_killed_withdraw(), false);
//     assert_eq!(liq_pool.get_is_killed_swap(), false);
//     assert_eq!(liq_pool.get_is_killed_claim(), false);

//     let admin = users[0].clone();

//     liq_pool.kill_swap(&admin);
//     assert_eq!(
//         vec![&e, e.events().all().last().unwrap()],
//         vec![
//             &e,
//             (
//                 liq_pool.address.clone(),
//                 (Symbol::new(&e, "kill_swap"),).into_val(&e),
//                 Val::VOID.into_val(&e),
//             )
//         ]
//     );
//     assert_eq!(liq_pool.get_is_killed_deposit(), false);
//     assert_eq!(liq_pool.get_is_killed_withdraw(), false);
//     assert_eq!(liq_pool.get_is_killed_swap(), true);
//     assert_eq!(liq_pool.get_is_killed_claim(), false);

//     let user1 = users[1].clone();
//     let desired_amount = 1_0000000;

//     liq_pool.deposit(&user1, &desired_amount);

//     assert_eq!(
//         liq_pool.try_swap(&user1, &1, &0, &1, &0).unwrap_err(),
//         Ok(Error::from_contract_error(206))
//     );

//     liq_pool.unkill_swap(&admin);
//     assert_eq!(
//         vec![&e, e.events().all().last().unwrap()],
//         vec![
//             &e,
//             (
//                 liq_pool.address.clone(),
//                 (Symbol::new(&e, "unkill_swap"),).into_val(&e),
//                 Val::VOID.into_val(&e),
//             )
//         ]
//     );
//     assert_eq!(liq_pool.get_is_killed_deposit(), false);
//     assert_eq!(liq_pool.get_is_killed_withdraw(), false);
//     assert_eq!(liq_pool.get_is_killed_swap(), false);
//     assert_eq!(liq_pool.get_is_killed_claim(), false);

//     liq_pool.swap(&user1, &1, &0, &1, &0);
// }

// #[test]
// fn test_deposit_killed() {
//     let setup = Setup::new_with_config(
//         &(TestConfig {
//             mint_to_user: i128::MAX,
//             ..TestConfig::default()
//         }),
//     );
//     let e = setup.env;
//     let liq_pool = setup.liq_pool;
//     let users = setup.users;

//     assert_eq!(liq_pool.get_is_killed_deposit(), false);
//     assert_eq!(liq_pool.get_is_killed_withdraw(), false);
//     assert_eq!(liq_pool.get_is_killed_swap(), false);
//     assert_eq!(liq_pool.get_is_killed_claim(), false);

//     let admin = users[0].clone();

//     liq_pool.kill_deposit(&admin);
//     assert_eq!(
//         vec![&e, e.events().all().last().unwrap()],
//         vec![
//             &e,
//             (
//                 liq_pool.address.clone(),
//                 (Symbol::new(&e, "kill_deposit"),).into_val(&e),
//                 Val::VOID.into_val(&e),
//             )
//         ]
//     );
//     assert_eq!(liq_pool.get_is_killed_deposit(), true);
//     assert_eq!(liq_pool.get_is_killed_withdraw(), false);
//     assert_eq!(liq_pool.get_is_killed_swap(), false);
//     assert_eq!(liq_pool.get_is_killed_claim(), false);

//     let user1 = users[1].clone();
//     let desired_amount = 1_0000000;

//     assert_eq!(
//         liq_pool.try_deposit(&user1, &desired_amount).unwrap_err(),
//         Ok(Error::from_contract_error(205))
//     );

//     liq_pool.unkill_deposit(&admin);
//     assert_eq!(
//         vec![&e, e.events().all().last().unwrap()],
//         vec![
//             &e,
//             (
//                 liq_pool.address.clone(),
//                 (Symbol::new(&e, "unkill_deposit"),).into_val(&e),
//                 Val::VOID.into_val(&e),
//             )
//         ]
//     );
//     assert_eq!(liq_pool.get_is_killed_deposit(), false);
//     assert_eq!(liq_pool.get_is_killed_withdraw(), false);
//     assert_eq!(liq_pool.get_is_killed_swap(), false);
//     assert_eq!(liq_pool.get_is_killed_claim(), false);

//     liq_pool.deposit(&user1, &desired_amount);
// }

// #[test]
// fn test_withdraw_killed() {
//     let setup = Setup::new_with_config(
//         &(TestConfig {
//             mint_to_user: i128::MAX,
//             ..TestConfig::default()
//         }),
//     );
//     let e = setup.env;
//     let liq_pool = setup.liq_pool;
//     let users = setup.users;

//     assert_eq!(liq_pool.get_is_killed_deposit(), false);
//     assert_eq!(liq_pool.get_is_killed_withdraw(), false);
//     assert_eq!(liq_pool.get_is_killed_swap(), false);
//     assert_eq!(liq_pool.get_is_killed_claim(), false);

//     let admin = users[0].clone();

//     liq_pool.kill_withdraw(&admin);
//     assert_eq!(
//         vec![&e, e.events().all().last().unwrap()],
//         vec![
//             &e,
//             (
//                 liq_pool.address.clone(),
//                 (Symbol::new(&e, "kill_withdraw"),).into_val(&e),
//                 Val::VOID.into_val(&e),
//             )
//         ]
//     );
//     assert_eq!(liq_pool.get_is_killed_deposit(), false);
//     assert_eq!(liq_pool.get_is_killed_withdraw(), true);
//     assert_eq!(liq_pool.get_is_killed_swap(), false);
//     assert_eq!(liq_pool.get_is_killed_claim(), false);

//     let user1 = users[1].clone();
//     let desired_amount = 1_0000000;

//     assert_eq!(
//         liq_pool.try_withdraw(&user1, &desired_amount).unwrap_err(),
//         Ok(Error::from_contract_error(209))
//     );

//     liq_pool.unkill_withdraw(&admin);
//     assert_eq!(
//         vec![&e, e.events().all().last().unwrap()],
//         vec![
//             &e,
//             (
//                 liq_pool.address.clone(),
//                 (Symbol::new(&e, "unkill_withdraw"),).into_val(&e),
//                 Val::VOID.into_val(&e),
//             )
//         ]
//     );
//     assert_eq!(liq_pool.get_is_killed_deposit(), false);
//     assert_eq!(liq_pool.get_is_killed_withdraw(), false);
//     assert_eq!(liq_pool.get_is_killed_swap(), false);
//     assert_eq!(liq_pool.get_is_killed_claim(), false);

//     liq_pool.deposit(&user1, &desired_amount);
// }

// #[test]
// fn test_claim_killed() {
//     let setup = Setup::setup(&TestConfig::default());
//     setup.mint_tokens_for_users(TestConfig::default().mint_to_user);
//     let env = setup.env;
//     let liq_pool = setup.liq_pool;
//     let users = setup.users;
//     assert_eq!(liq_pool.get_is_killed_deposit(), false);
//     assert_eq!(liq_pool.get_is_killed_withdraw(), false);
//     assert_eq!(liq_pool.get_is_killed_swap(), false);
//     assert_eq!(liq_pool.get_is_killed_claim(), false);

//     liq_pool.kill_claim(&users[0]);
//     assert_eq!(
//         vec![&env, env.events().all().last().unwrap()],
//         vec![
//             &env,
//             (
//                 liq_pool.address.clone(),
//                 (Symbol::new(&env, "kill_claim"),).into_val(&env),
//                 Val::VOID.into_val(&env),
//             )
//         ]
//     );
//     assert_eq!(liq_pool.get_is_killed_deposit(), false);
//     assert_eq!(liq_pool.get_is_killed_withdraw(), false);
//     assert_eq!(liq_pool.get_is_killed_swap(), false);
//     assert_eq!(liq_pool.get_is_killed_claim(), true);

//     // 10 seconds. user depositing
//     jump(&env, 10);
//     liq_pool.deposit(&users[1], &100);

//     // 20 seconds. rewards set up for 60 seconds
//     jump(&env, 10);
//     let reward_1_tps = 10_5000000_u128;
//     let total_reward_1 = reward_1_tps * 60;
//     liq_pool.set_incentives_config(
//         &users[0],
//         &env.ledger().timestamp().saturating_add(60),
//         &reward_1_tps,
//     );

//     // 90 seconds. rewards ended.
//     jump(&env, 70);

//     // 100 seconds. user claim reward
//     jump(&env, 10);

//     assert_eq!(
//         liq_pool.try_claim(&users[1]).unwrap_err(),
//         Ok(Error::from_contract_error(207))
//     );
//     liq_pool.unkill_claim(&users[0]);
//     assert_eq!(
//         vec![&env, env.events().all().last().unwrap()],
//         vec![
//             &env,
//             (
//                 liq_pool.address.clone(),
//                 (Symbol::new(&env, "unkill_claim"),).into_val(&env),
//                 Val::VOID.into_val(&env),
//             )
//         ]
//     );
//     assert_eq!(liq_pool.get_is_killed_deposit(), false);
//     assert_eq!(liq_pool.get_is_killed_withdraw(), false);
//     assert_eq!(liq_pool.get_is_killed_swap(), false);
//     assert_eq!(liq_pool.get_is_killed_claim(), false);
//     assert_eq!(liq_pool.claim(&users[1]).0, total_reward_1);
// }

// #[test]
// fn test_withdraw_rewards() {
//     let setup = Setup::setup(&TestConfig::default());
//     // test user cannot withdraw reward tokens from the pool
//     let e = setup.env;
//     let liq_pool = setup.liq_pool;

//     let admin = Address::generate(&e);
//     let user1 = Address::generate(&e);
//     let user2 = Address::generate(&e);

//     let mut token1 = create_token_contract(&e, &admin);
//     let mut token2 = create_token_contract(&e, &admin);

//     if &token2.address < &token1.address {
//         std::mem::swap(&mut token1, &mut token2);
//     }
//     let token2_admin_client = get_token_admin_client(&e, &token2.address);
//     let token_reward_admin_client = SorobanTokenAdminClient::new(&e, &token1.address.clone());

//     let token_share = LpTokenClient::new(&e, &liq_pool.share_id());

//     token2_admin_client.mint(&user1, &100_0000000);
//     liq_pool.deposit(&user1, &100_0000000);
//     assert_eq!(
//         liq_pool.get_reserves(),
//         Vec::from_array(&e, [50_0000000, 100_0000000])
//     );

//     liq_pool.set_incentives_config(
//         &admin,
//         &e.ledger().timestamp().saturating_add(100),
//         &1_000_0000000,
//     );
//     token_reward_admin_client.mint(&liq_pool.address, &(1_000_0000000 * 100));
//     jump(&e, 100);

//     token2_admin_client.mint(&user2, &1_000_0000000);
//     liq_pool.deposit(&user2, &1_000_0000000);
//     assert_eq!(
//         liq_pool.get_reserves(),
//         Vec::from_array(&e, [1_100_0000000, 1_100_0000000])
//     ); // FIXME:

//     assert_eq!(
//         liq_pool.get_reserves(),
//         Vec::from_array(&e, [1_100_0000000, 1_100_0000000])
//     ); // FIXME:
//     assert_eq!(
//         token1.balance(&liq_pool.address),
//         1_100_0000000 + 1_000_0000000 * 100
//     );
//     assert_eq!(token2.balance(&liq_pool.address), 1_100_0000000);

//     assert_eq!(
//         liq_pool.withdraw(&user2, &(token_share.balance(&user2) as u128)),
//         1_000_0000000
//     );
//     assert_eq!(
//         liq_pool.get_reserves(),
//         Vec::from_array(&e, [100_0000000, 100_0000000])
//     ); // FIXME:
//     assert_eq!(
//         token1.balance(&liq_pool.address),
//         100_0000000 + 1_000_0000000 * 100
//     );
//     assert_eq!(token2.balance(&liq_pool.address), 100_0000000);
//     assert_eq!(token1.balance(&user2), 1_000_0000000);
//     assert_eq!(token2.balance(&user2), 1_000_0000000);

//     assert_eq!(liq_pool.claim(&user1).0, 1_000_0000000 * 100);
//     assert_eq!(liq_pool.claim(&user2).0, 0);
// }

// // // #[test]
// // // fn test_deposit_rewards() {
// // //     // test pool reserves are not affected by rewards if reward token is one of pool tokens and presented in pool balance
// // //     let e = Env::default();
// // //     e.mock_all_auths();

// // //     let admin = Address::generate(&e);
// // //     let user1 = Address::generate(&e);

// // //     let mut token1 = create_token_contract(&e, &admin);
// // //     let mut token2 = create_token_contract(&e, &admin);

// // //     if &token2.address < &token1.address {
// // //         std::mem::swap(&mut token1, &mut token2);
// // //     }
// // //     // let token1_admin_client = get_token_admin_client(&e, &token1.address);
// // //     let token2_admin_client = get_token_admin_client(&e, &token2.address);
// // //     let token_reward_admin_client = SorobanTokenAdminClient::new(&e, &token1.address.clone());

// // //     let router = Address::generate(&e);
// // //     let oracle = Address::generate(&e);
// // //     let asset = Asset::Other(Symbol::new(&e, "SOL"));

// // //     let liq_pool = create_pool_contract(
// // //         &e,
// // //         &admin,
// // //         &router,
// // //         &oracle,
// // //         &asset,
// // //         &install_token_wasm(&e),
// // //         &Vec::from_array(&e, [token1.address.clone(), token2.address.clone()]),
// // //         &token_reward_admin_client.address,
// // //         30
// // //     );

// // //     liq_pool.set_incentives_config(
// // //         &admin,
// // //         &e.ledger().timestamp().saturating_add(100),
// // //         &1_000_0000000
// // //     );
// // //     token_reward_admin_client.mint(&liq_pool.address, &(1_000_0000000 * 100));
// // //     assert_eq!(liq_pool.get_reserves(), Vec::from_array(&e, [0, 0]));

// // //     // token1_admin_client.mint(&user1, &1000_0000000);
// // //     token2_admin_client.mint(&user1, &1000_0000000);
// // //     liq_pool.deposit(&user1, &Vec::from_array(&e, [1_0000000, 100_0000000]), &0);
// // //     assert_eq!(liq_pool.get_reserves(), Vec::from_array(&e, [1_0000000, 100_0000000]));
// // //     liq_pool.deposit(&user1, &Vec::from_array(&e, [1_0000000, 100_0000000]), &0);
// // //     assert_eq!(liq_pool.get_reserves(), Vec::from_array(&e, [2_0000000, 200_0000000]));
// // // }

// // // #[test]
// // // fn test_swap_rewards() {
// // //     // check that swap rewards are calculated correctly if reward token is one of pool tokens
// // //     let e = Env::default();
// // //     e.mock_all_auths();

// // //     let admin = Address::generate(&e);
// // //     let user1 = Address::generate(&e);
// // //     let user2 = Address::generate(&e);

// // //     let mut token1 = create_token_contract(&e, &admin);
// // //     let mut token2 = create_token_contract(&e, &admin);

// // //     if &token2.address < &token1.address {
// // //         std::mem::swap(&mut token1, &mut token2);
// // //     }
// // //     let token1_admin_client = get_token_admin_client(&e, &token1.address);
// // //     let token2_admin_client = get_token_admin_client(&e, &token2.address);
// // //     let token_reward_admin_client = SorobanTokenAdminClient::new(&e, &token1.address.clone());

// // //     let router = Address::generate(&e);
// // //     let oracle = Address::generate(&e);
// // //     let asset = Asset::Other(Symbol::new(&e, "SOL"));

// // //     // we compare two pools to check swap in both directions
// // //     let liq_pool1 = create_pool_contract(
// // //         &e,
// // //         &admin,
// // //         &router,
// // //         &oracle,
// // //         &asset,
// // //         &install_token_wasm(&e),
// // //         &Vec::from_array(&e, [token1.address.clone(), token2.address.clone()]),
// // //         &token_reward_admin_client.address,
// // //         30
// // //     );
// // //     let liq_pool2 = create_pool_contract(
// // //         &e,
// // //         &admin,
// // //         &router,
// // //         &oracle,
// // //         &asset,
// // //         &install_token_wasm(&e),
// // //         &Vec::from_array(&e, [token1.address.clone(), token2.address.clone()]),
// // //         &token_reward_admin_client.address,
// // //         30
// // //     );
// // //     // token1_admin_client.mint(&user1, &200_0000000);
// // //     token2_admin_client.mint(&user1, &200_0000000);
// // //     liq_pool1.deposit(&user1, &100_0000000, &0);
// // //     liq_pool2.deposit(&user1, &100_0000000, &0);
// // //     assert_eq!(liq_pool1.get_reserves(), Vec::from_array(&e, [100_0000000, 100_0000000]));
// // //     assert_eq!(liq_pool2.get_reserves(), Vec::from_array(&e, [100_0000000, 100_0000000]));

// // //     let estimate1_before_rewards = liq_pool1.estimate_swap(&0, &1, &10_0000000);
// // //     let estimate2_before_rewards = liq_pool1.estimate_swap(&1, &0, &10_0000000);
// // //     // swap is balanced, so values should be the same
// // //     assert_eq!(estimate1_before_rewards, estimate2_before_rewards);

// // //     liq_pool1.set_incentives_config(
// // //         &admin,
// // //         &e.ledger().timestamp().saturating_add(100),
// // //         &1_000_0000000
// // //     );
// // //     liq_pool2.set_incentives_config(
// // //         &admin,
// // //         &e.ledger().timestamp().saturating_add(100),
// // //         &1_000_0000000
// // //     );
// // //     token_reward_admin_client.mint(&liq_pool1.address, &(1_000_0000000 * 100));
// // //     token_reward_admin_client.mint(&liq_pool2.address, &(1_000_0000000 * 100));
// // //     jump(&e, 100);

// // //     let estimate1_after_rewards = liq_pool1.estimate_swap(&0, &1, &10_0000000);
// // //     let estimate2_after_rewards = liq_pool1.estimate_swap(&1, &0, &10_0000000);
// // //     // balances are out of balance, but reserves are balanced.
// // //     assert_eq!(estimate1_after_rewards, estimate2_after_rewards);
// // //     assert_eq!(estimate1_before_rewards, estimate1_after_rewards);

// // //     // token1_admin_client.mint(&user2, &10_0000000);
// // //     token2_admin_client.mint(&user2, &10_0000000);
// // //     // in case of disbalance, user may receive much more tokens than he sent as reward is included
// // //     let swap_result1 = liq_pool1.swap(&user2, &0, &1, &10_0000000, &estimate1_after_rewards);
// // //     let swap_result2 = liq_pool2.swap(&user2, &1, &0, &10_0000000, &estimate1_after_rewards);
// // //     assert_eq!(swap_result1, estimate1_after_rewards);
// // //     assert_eq!(swap_result2, estimate1_after_rewards);

// // //     let reserves1 = liq_pool1.get_reserves();

// // //     // check that balance minus rewards is equal to reserves as they should also have fee and it's same for both pools but in different order
// // //     assert_eq!(
// // //         liq_pool1.get_reserves(),
// // //         Vec::from_array(&e, [
// // //             (token1.balance(&liq_pool1.address) as u128) - 1_000_0000000 * 100,
// // //             token2.balance(&liq_pool1.address) as u128,
// // //         ])
// // //     );
// // //     // reverse pool1 reserves to check swap in other direction gave same results
// // //     assert_eq!(
// // //         liq_pool2.get_reserves(),
// // //         Vec::from_array(&e, [reserves1.get(1).unwrap(), reserves1.get(0).unwrap()])
// // //     );
// // // }

// // // #[test]
// // // fn test_claim_rewards() {
// // //     // test user cannot claim from pool if rewards configured but not distributed
// // //     let e = Env::default();
// // //     e.mock_all_auths();

// // //     let admin = Address::generate(&e);
// // //     let user1 = Address::generate(&e);

// // //     let mut token1 = create_token_contract(&e, &admin);
// // //     let mut token2 = create_token_contract(&e, &admin);

// // //     if &token2.address < &token1.address {
// // //         std::mem::swap(&mut token1, &mut token2);
// // //     }
// // //     let token1_admin_client = get_token_admin_client(&e, &token1.address);
// // //     let token2_admin_client = get_token_admin_client(&e, &token2.address);
// // //     let token_reward_admin_client = SorobanTokenAdminClient::new(&e, &token1.address.clone());

// // //     let router = Address::generate(&e);
// // //     let oracle = Address::generate(&e);
// // //     let asset = Asset::Other(Symbol::new(&e, "SOL"));

// // //     let liq_pool = create_pool_contract(
// // //         &e,
// // //         &admin,
// // //         &router,
// // //         &oracle,
// // //         &asset,
// // //         &install_token_wasm(&e),
// // //         &Vec::from_array(&e, [token1.address.clone(), token2.address.clone()]),
// // //         &token_reward_admin_client.address,
// // //         30
// // //     );

// // //     // token1_admin_client.mint(&user1, &100_0000000);
// // //     token2_admin_client.mint(&user1, &100_0000000);
// // //     liq_pool.deposit(&user1, &100_0000000, &0);
// // //     assert_eq!(liq_pool.get_reserves(), Vec::from_array(&e, [100_0000000, 100_0000000]));

// // //     liq_pool.set_incentives_config(&admin, &e.ledger().timestamp().saturating_add(100), &1000);
// // //     jump(&e, 100);

// // //     assert!(liq_pool.try_claim(&user1).is_err());
// // //     token_reward_admin_client.mint(&liq_pool.address, &(1000 * 100));
// // //     assert_eq!(liq_pool.claim(&user1), 1000 * 100);
// // // }

// // // #[test]
// // // fn test_drain_reward() {
// // //     let setup = Setup::new_with_config(
// // //         &(TestConfig {
// // //             users_count: 5,
// // //             reward_tps: 10_5000000,
// // //             rewards_count: 10_5000000 * 60,
// // //             mint_to_user: 1000_0000000,
// // //             ..TestConfig::default()
// // //         })
// // //     );
// // //     let env = setup.env;
// // //     let liq_pool = setup.liq_pool;
// // //     let token_share = setup.token_share;
// // //     let users = setup.users;

// // //     // 10 seconds passed since config, user depositing
// // //     jump(&env, 10);

// // //     liq_pool.deposit(&users[0], &1000_0000000, &0);
// // //     let (_, lp_amount) = liq_pool.deposit(
// // //         &users[1],
// // //         &100_0000000,
// // //         &0
// // //     );

// // //     jump(&env, 10);

// // //     for i in 2..5 {
// // //         token_share.transfer(&users[i - 1], &users[i], &(lp_amount as i128));
// // //         // liq_pool.get_user_reward(&users[i]);
// // //         // liq_pool.claim(&users[i]);
// // //         liq_pool.deposit(&users[i], &1, &0);
// // //     }

// // //     jump(&env, 50);
// // //     assert_eq!(liq_pool.claim(&users[4]), 381818182);
// // //     token_share.transfer(&users[4], &users[3], &(lp_amount as i128));
// // //     assert_eq!(liq_pool.claim(&users[3]), 0);
// // //     token_share.transfer(&users[3], &users[2], &(lp_amount as i128));
// // //     assert_eq!(liq_pool.claim(&users[2]), 0);
// // //     token_share.transfer(&users[2], &users[1], &(lp_amount as i128));
// // //     assert_eq!(liq_pool.claim(&users[1]), 95454545);
// // //     assert_eq!(liq_pool.claim(&users[0]), 4772727271);
// // // }

// // #[test]
// // fn test_drain_reserves() {
// //     // test pool reserves are not affected by rewards if reward token is one of pool tokens and presented in pool balance
// //     let e = Env::default();
// //     e.mock_all_auths();
// //     e.cost_estimate().budget().reset_unlimited();

// //     let admin = Address::generate(&e);
// //     let user1 = Address::generate(&e);
// //     let user2 = Address::generate(&e);
// //     let user3 = Address::generate(&e);
// //     let user4 = Address::generate(&e);

// //     let mut token1 = create_token_contract(&e, &admin);
// //     let mut token2 = create_token_contract(&e, &admin);

// //     // if &token2.address < &token1.address {
// //     //     std::mem::swap(&mut token1, &mut token2);
// //     // }
// //     let token2_admin_client = get_token_admin_client(&e, &token2.address);
// //     let token_reward_admin_client = SorobanTokenAdminClient::new(&e, &token1.address.clone());

// //     let router = Address::generate(&e);

// //     let asset = Asset::Other(Symbol::new(&e, "SOL"));

// //     let liq_pool = create_pool_contract(
// //         &e,
// //         &admin,
// //         &router,
// //         &("", ""),
// //         &asset,
// //         &install_token_wasm(&e),
// //         &String::from_str(&e, "Pool Share Token"),
// //         &String::from_str(&e, "Pool Share Token"),
// //         &Vec::from_array(&e, [token1.address.clone(), token2.address.clone()]),
// //         &token_reward_admin_client.address,
// //         30
// //     );

// //     liq_pool.set_incentives_config(
// //         &admin,
// //         &e.ledger().timestamp().saturating_add(100),
// //         &1_000_0000000
// //     );
// //     token_reward_admin_client.mint(&liq_pool.address, &(1_000_0000000 * 100));
// //     assert_eq!(liq_pool.get_reserves(), Vec::from_array(&e, [0, 0]));

// //     // first user deposits
// //     token2_admin_client.mint(&user1, &1_000_000_0000000);
// //     liq_pool.deposit(&user1, &1_000_000_0000000);

// //     // first exploiter deposits
// //     token2_admin_client.mint(&user2, &1_000_000_0000000);
// //     let (_, lp_amount) = liq_pool.deposit(&user2, &300_000_0000000);

// //     let token_share = SorobanTokenClient::new(&e, &liq_pool.share_id());

// //     token_share.transfer(&user2, &user3, &(lp_amount as i128));
// //     liq_pool.claim(&user3);
// //     token_share.transfer(&user3, &user4, &(lp_amount as i128));
// //     liq_pool.claim(&user4);

// //     jump(&e, 100);

// //     // exploit starts
// //     assert_eq!(liq_pool.claim(&user4), 230769230769);
// //     token_share.transfer(&user4, &user3, &(lp_amount as i128));
// //     assert_eq!(liq_pool.claim(&user3), 0);
// //     token_share.transfer(&user3, &user2, &(lp_amount as i128));
// //     assert_eq!(liq_pool.claim(&user2), 0);

// //     // first user claims
// //     assert_eq!(liq_pool.claim(&user1), 769230769230);

// //     // check reserves
// //     assert_eq!(
// //         liq_pool.get_reserves(),
// //         Vec::from_array(&e, [1_300_000_0000000, 1_300_000_0000000]) // FIXME:
// //     );
// //     assert_eq!(token1.balance(&liq_pool.address), 1_300_000_0000001); // 1 token left on balance because of rounding
// //     assert_eq!(token2.balance(&liq_pool.address), 1_300_000_0000000);
// // }

// // // #[test]
// // // fn test_return_unused_reward() {
// // //     let setup = Setup::new_with_config(
// // //         &(TestConfig {
// // //             reward_tps: 0,
// // //             reward_token_in_pool: false,
// // //             mint_to_user: 0,
// // //             rewards_count: 0,
// // //             ..TestConfig::default()
// // //         })
// // //     );
// // //     assert_ne!(setup.token1.address, setup.token_reward.address);
// // //     let e = setup.env;
// // //     let admin = setup.admin;
// // //     let liq_pool = setup.liq_pool;
// // //     let router = setup.router;
// // //     // let token_1_admin_client = SorobanTokenAdminClient::new(&e, &setup.token1.address.clone());
// // //     let token_2_admin_client = SorobanTokenAdminClient::new(&e, &setup.token2.address.clone());
// // //     let token_reward_admin_client = SorobanTokenAdminClient::new(
// // //         &e,
// // //         &setup.token_reward.address.clone()
// // //     );
// // //     let user = Address::generate(&e);

// // //     // token_1_admin_client.mint(&user, &1000_0000000);
// // //     token_2_admin_client.mint(&user, &1000_0000000);
// // //     liq_pool.deposit(&user, &1000_0000000, &0);

// // //     liq_pool.set_incentives_config(&admin, &e.ledger().timestamp().saturating_add(60), &1_0000000);
// // //     // pool has configured rewards, but not minted
// // //     assert_eq!(liq_pool.get_unused_reward(), 0);

// // //     token_reward_admin_client.mint(&liq_pool.address, &(1_0000000 * 100));

// // //     // we've configured rewards for 60 seconds, but minted for 100. 40 surplus
// // //     assert_eq!(liq_pool.get_unused_reward(), 1_0000000 * 40);

// // //     // 10 seconds passed
// // //     jump(&e, 10);
// // //     liq_pool.claim(&user);

// // //     assert_eq!(liq_pool.get_unused_reward(), 1_0000000 * 40);
// // //     assert_eq!(setup.token_reward.balance(&router), 0);
// // //     jump(&e, 10);

// // //     // pool stops rewards on new iteration
// // //     liq_pool.set_incentives_config(&admin, &e.ledger().timestamp().saturating_add(0), &0);
// // //     assert_eq!(liq_pool.get_unused_reward(), 1_0000000 * 80);

// // //     jump(&e, 10);
// // //     // new config iteration. pool got 50 seconds of rewards. 100 - 20 - 50 = 30 unused
// // //     liq_pool.set_incentives_config(&admin, &e.ledger().timestamp().saturating_add(50), &1_0000000);

// // //     // neither time nor claim should affect unused rewards
// // //     assert_eq!(liq_pool.get_unused_reward(), 1_0000000 * 30);
// // //     jump(&e, 10);
// // //     assert_eq!(liq_pool.get_unused_reward(), 1_0000000 * 30);
// // //     liq_pool.claim(&user);
// // //     assert_eq!(liq_pool.get_unused_reward(), 1_0000000 * 30);
// // //     jump(&e, 10);
// // //     assert_eq!(liq_pool.get_unused_reward(), 1_0000000 * 30);
// // //     assert_eq!(setup.token_reward.balance(&router), 0);
// // //     assert_eq!(liq_pool.return_unused_reward(&admin), 1_0000000 * 30);
// // //     assert_eq!(setup.token_reward.balance(&router), 1_0000000 * 30);
// // // }

// // // #[test]
// // // fn test_return_unused_reward_reward_token_in_pool() {
// // //     let setup = Setup::new_with_config(
// // //         &(TestConfig {
// // //             reward_tps: 0,
// // //             reward_token_in_pool: true,
// // //             mint_to_user: 0,
// // //             rewards_count: 0,
// // //             ..TestConfig::default()
// // //         })
// // //     );
// // //     assert_eq!(setup.token1.address, setup.token_reward.address);
// // //     let e = setup.env;
// // //     let admin = setup.admin;
// // //     let liq_pool = setup.liq_pool;
// // //     let router = setup.router;
// // //     let token_1_admin_client = SorobanTokenAdminClient::new(&e, &setup.token1.address.clone());
// // //     let token_2_admin_client = SorobanTokenAdminClient::new(&e, &setup.token2.address.clone());
// // //     let token_reward_admin_client = SorobanTokenAdminClient::new(
// // //         &e,
// // //         &setup.token_reward.address.clone()
// // //     );
// // //     let user = Address::generate(&e);

// // //     // token_1_admin_client.mint(&user, &1000_0000000);
// // //     token_2_admin_client.mint(&user, &1000_0000000);
// // //     liq_pool.deposit(&user, &1000_0000000, &0);

// // //     liq_pool.set_incentives_config(&admin, &e.ledger().timestamp().saturating_add(60), &1_0000000);
// // //     // pool has configured rewards, but not minted
// // //     assert_eq!(liq_pool.get_unused_reward(), 0);

// // //     token_reward_admin_client.mint(&liq_pool.address, &(1_0000000 * 100));

// // //     // we've configured rewards for 60 seconds, but minted for 100. 40 surplus
// // //     assert_eq!(liq_pool.get_unused_reward(), 1_0000000 * 40);

// // //     // 10 seconds passed
// // //     jump(&e, 10);
// // //     liq_pool.claim(&user);

// // //     assert_eq!(liq_pool.get_unused_reward(), 1_0000000 * 40);
// // //     assert_eq!(setup.token_reward.balance(&router), 0);
// // //     jump(&e, 10);

// // //     // pool stops rewards on new iteration
// // //     liq_pool.set_incentives_config(&admin, &e.ledger().timestamp().saturating_add(0), &0);
// // //     assert_eq!(liq_pool.get_unused_reward(), 1_0000000 * 80);

// // //     jump(&e, 10);
// // //     // new config iteration. pool got 50 seconds of rewards. 100 - 20 - 50 = 30 unused
// // //     liq_pool.set_incentives_config(&admin, &e.ledger().timestamp().saturating_add(50), &1_0000000);

// // //     // neither time nor claim should affect unused rewards
// // //     assert_eq!(liq_pool.get_unused_reward(), 1_0000000 * 30);
// // //     jump(&e, 10);
// // //     assert_eq!(liq_pool.get_unused_reward(), 1_0000000 * 30);
// // //     liq_pool.claim(&user);
// // //     assert_eq!(liq_pool.get_unused_reward(), 1_0000000 * 30);
// // //     jump(&e, 10);
// // //     assert_eq!(liq_pool.get_unused_reward(), 1_0000000 * 30);
// // //     assert_eq!(setup.token_reward.balance(&router), 0);
// // //     assert_eq!(liq_pool.return_unused_reward(&admin), 1_0000000 * 30);
// // //     assert_eq!(setup.token_reward.balance(&router), 1_0000000 * 30);
// // // }

// //     ______     _______        __       ______   ___       _______
// //    /    " \   /"      \      /""\     /" _  "\ |"  |     /"     "|
// //   // ____  \ |:        |    /    \   (: ( \___)||  |    (: ______)
// //  /  /    ) :)|_____/   )   /' /\  \   \/ \     |:  |     \/    |
// // (: (____/ //  //      /   //  __'  \  //  \ _   \  |___  // ___)_
// //  \        /  |:  __   \  /   /  \\  \(:   _) \ ( \_|:  \(:      "|
// //   \"_____/   |__|  \___)(___/    \___)\_______) \_______)\_______)

// #[test]
// fn test_swap_with_invalid_oracle() {
//     let setup = Setup::default();
//     let users = setup.users;

//     // Collect pre-swap values
//     let last_price = setup.registry.get_last_price(&setup.btc_asset_id);

//     // Invalidate the oracle

//     // Swap
//     let amount_out = setup.router.swap(
//         &users[1],
//         &tokens,
//         &setup.token1.address,
//         &setup.token2.address,
//         &setup.btc_asset_id,
//         &10_0000000,
//         &2_8952731,
//     );

//     // [ ] Ensure swap was executed at the last valid oracle price and NOT the invalid price
// }

// // paused ops

// #[test]
// fn test_kill_deposit_event() {
//     let setup = Setup::default();
//     let pool = setup.liq_pool;

//     pool.kill_deposit(&setup.admin);
//     assert_eq!(
//         vec![&setup.env, setup.env.events().all().last().unwrap()],
//         vec![
//             &setup.env,
//             (
//                 pool.address.clone(),
//                 (Symbol::new(&setup.env, "kill_deposit"),).into_val(&setup.env),
//                 ().into_val(&setup.env),
//             )
//         ]
//     );
// }

// #[test]
// fn test_kill_swap_event() {
//     let setup = Setup::default();
//     let pool = setup.liq_pool;

//     pool.kill_swap(&setup.admin);
//     assert_eq!(
//         vec![&setup.env, setup.env.events().all().last().unwrap()],
//         vec![
//             &setup.env,
//             (
//                 pool.address.clone(),
//                 (Symbol::new(&setup.env, "kill_swap"),).into_val(&setup.env),
//                 ().into_val(&setup.env),
//             )
//         ]
//     );
// }

// #[test]
// fn test_kill_claim_event() {
//     let setup = Setup::default();
//     let pool = setup.liq_pool;

//     pool.kill_claim(&setup.admin);
//     assert_eq!(
//         vec![&setup.env, setup.env.events().all().last().unwrap()],
//         vec![
//             &setup.env,
//             (
//                 pool.address.clone(),
//                 (Symbol::new(&setup.env, "kill_claim"),).into_val(&setup.env),
//                 ().into_val(&setup.env),
//             )
//         ]
//     );
// }

// #[test]
// fn test_unkill_deposit_event() {
//     let setup = Setup::default();
//     let pool = setup.liq_pool;

//     pool.unkill_deposit(&setup.admin);
//     assert_eq!(
//         vec![&setup.env, setup.env.events().all().last().unwrap()],
//         vec![
//             &setup.env,
//             (
//                 pool.address.clone(),
//                 (Symbol::new(&setup.env, "unkill_deposit"),).into_val(&setup.env),
//                 ().into_val(&setup.env),
//             )
//         ]
//     );
// }

// #[test]
// fn test_unkill_swap_event() {
//     let setup = Setup::default();
//     let pool = setup.liq_pool;

//     pool.unkill_swap(&setup.admin);
//     assert_eq!(
//         vec![&setup.env, setup.env.events().all().last().unwrap()],
//         vec![
//             &setup.env,
//             (
//                 pool.address.clone(),
//                 (Symbol::new(&setup.env, "unkill_swap"),).into_val(&setup.env),
//                 ().into_val(&setup.env),
//             )
//         ]
//     );
// }

// #[test]
// fn test_unkill_claim_event() {
//     let setup = Setup::default();
//     let pool = setup.liq_pool;

//     pool.unkill_claim(&setup.admin);
//     assert_eq!(
//         vec![&setup.env, setup.env.events().all().last().unwrap()],
//         vec![
//             &setup.env,
//             (
//                 pool.address.clone(),
//                 (Symbol::new(&setup.env, "unkill_claim"),).into_val(&setup.env),
//                 ().into_val(&setup.env),
//             )
//         ]
//     );
// }

// #[test]
// fn test_set_privileged_addresses_event() {
//     let setup = Setup::default();
//     let pool = setup.liq_pool;

//     pool.set_privileged_addrs(
//         &setup.admin.clone(),
//         &setup.rewards_admin.clone(),
//         &setup.operations_admin.clone(),
//         &setup.pause_admin.clone(),
//         &Vec::from_array(&setup.env, [setup.emergency_pause_admin.clone()]),
//     );

//     assert_eq!(
//         vec![&setup.env, setup.env.events().all().last().unwrap()],
//         vec![
//             &setup.env,
//             (
//                 pool.address.clone(),
//                 (Symbol::new(&setup.env, "set_privileged_addrs"),).into_val(&setup.env),
//                 (
//                     setup.rewards_admin,
//                     setup.operations_admin,
//                     setup.pause_admin,
//                     Vec::from_array(&setup.env, [setup.emergency_pause_admin]),
//                 )
//                     .into_val(&setup.env),
//             )
//         ]
//     );
// }

// #[test]
// fn test_set_rewards_config() {
//     let setup = Setup::default();
//     let pool = setup.liq_pool;

//     pool.set_incentives_config(
//         &setup.admin.clone(),
//         &setup.env.ledger().timestamp().saturating_add(100),
//         &1_0000000,
//     );

//     assert_eq!(
//         vec![&setup.env, setup.env.events().all().last().unwrap()],
//         vec![
//             &setup.env,
//             (
//                 pool.address.clone(),
//                 (Symbol::new(&setup.env, "set_rewards_config"),).into_val(&setup.env),
//                 (
//                     setup.env.ledger().timestamp().saturating_add(100),
//                     1_0000000_u128
//                 )
//                     .into_val(&setup.env),
//             )
//         ]
//     );
// }

// #[test]
// fn test_transfer_ownership_events() {
//     let setup = Setup::default();
//     let pool = setup.liq_pool;
//     let new_admin = Address::generate(&setup.env);

//     pool.commit_transfer_ownership(&setup.admin, &symbol_short!("Admin"), &new_admin);
//     assert_eq!(
//         vec![&setup.env, setup.env.events().all().last().unwrap()],
//         vec![
//             &setup.env,
//             (
//                 pool.address.clone(),
//                 (
//                     Symbol::new(&setup.env, "commit_transfer_ownership"),
//                     symbol_short!("Admin")
//                 )
//                     .into_val(&setup.env),
//                 (new_admin.clone(),).into_val(&setup.env),
//             )
//         ]
//     );

//     pool.revert_transfer_ownership(&setup.admin, &symbol_short!("Admin"));
//     assert_eq!(
//         vec![&setup.env, setup.env.events().all().last().unwrap()],
//         vec![
//             &setup.env,
//             (
//                 pool.address.clone(),
//                 (
//                     Symbol::new(&setup.env, "revert_transfer_ownership"),
//                     symbol_short!("Admin")
//                 )
//                     .into_val(&setup.env),
//                 ().into_val(&setup.env),
//             )
//         ]
//     );

//     pool.commit_transfer_ownership(&setup.admin, &symbol_short!("Admin"), &new_admin);
//     jump(&setup.env, ADMIN_ACTIONS_DELAY + 1);
//     pool.apply_transfer_ownership(&setup.admin, &symbol_short!("Admin"));
//     assert_eq!(
//         vec![&setup.env, setup.env.events().all().last().unwrap()],
//         vec![
//             &setup.env,
//             (
//                 pool.address.clone(),
//                 (
//                     Symbol::new(&setup.env, "apply_transfer_ownership"),
//                     symbol_short!("Admin")
//                 )
//                     .into_val(&setup.env),
//                 (new_admin.clone(),).into_val(&setup.env),
//             )
//         ]
//     );
// }

// #[test]
// fn test_upgrade_events() {
//     let setup = Setup::default();
//     let contract = setup.liq_pool;
//     let new_wasm_hash = install_dummy_wasm(&setup.env);
//     let token_new_wasm_hash = install_dummy_wasm(&setup.env);

//     contract.commit_upgrade(&setup.admin, &new_wasm_hash, &token_new_wasm_hash);
//     assert_eq!(
//         vec![&setup.env, setup.env.events().all().last().unwrap()],
//         vec![
//             &setup.env,
//             (
//                 contract.address.clone(),
//                 (Symbol::new(&setup.env, "commit_upgrade"),).into_val(&setup.env),
//                 (new_wasm_hash.clone(), token_new_wasm_hash.clone()).into_val(&setup.env),
//             )
//         ]
//     );

//     contract.revert_upgrade(&setup.admin);
//     assert_eq!(
//         vec![&setup.env, setup.env.events().all().last().unwrap()],
//         vec![
//             &setup.env,
//             (
//                 contract.address.clone(),
//                 (Symbol::new(&setup.env, "revert_upgrade"),).into_val(&setup.env),
//                 ().into_val(&setup.env),
//             )
//         ]
//     );

//     contract.commit_upgrade(&setup.admin, &new_wasm_hash, &token_new_wasm_hash);
//     jump(&setup.env, ADMIN_ACTIONS_DELAY + 1);
//     contract.apply_upgrade(&setup.admin);
//     assert_eq!(
//         vec![&setup.env, setup.env.events().all().last().unwrap()],
//         vec![
//             &setup.env,
//             (
//                 contract.address.clone(),
//                 (Symbol::new(&setup.env, "apply_upgrade"),).into_val(&setup.env),
//                 (new_wasm_hash.clone(), token_new_wasm_hash.clone()).into_val(&setup.env),
//             )
//         ]
//     );
// }

// #[test]
// fn test_emergency_mode_events() {
//     let setup = Setup::default();
//     let contract = setup.liq_pool;

//     contract.set_emergency_mode(&setup.emergency_admin, &true);
//     assert_eq!(
//         vec![&setup.env, setup.env.events().all().last().unwrap()],
//         vec![
//             &setup.env,
//             (
//                 contract.address.clone(),
//                 (Symbol::new(&setup.env, "enable_emergency_mode"),).into_val(&setup.env),
//                 ().into_val(&setup.env),
//             )
//         ]
//     );
//     contract.set_emergency_mode(&setup.emergency_admin, &false);
//     assert_eq!(
//         vec![&setup.env, setup.env.events().all().last().unwrap()],
//         vec![
//             &setup.env,
//             (
//                 contract.address.clone(),
//                 (Symbol::new(&setup.env, "disable_emergency_mode"),).into_val(&setup.env),
//                 ().into_val(&setup.env),
//             )
//         ]
//     );
// }

// #[test]
// fn test_emergency_upgrade() {
//     let setup = Setup::default();
//     let contract = setup.liq_pool;
//     let token = LpTokenClient::new(&setup.env, &contract.share_id());

//     let new_wasm = install_dummy_wasm(&setup.env);
//     let new_token_wasm = install_dummy_wasm(&setup.env);

//     assert_eq!(contract.get_emergency_mode(), false);
//     assert_ne!(contract.version(), 130);
//     assert_ne!(token.version(), 130);
//     contract.set_emergency_mode(&setup.emergency_admin, &true);

//     contract.commit_upgrade(&setup.admin, &new_wasm, &new_token_wasm);
//     contract.apply_upgrade(&setup.admin);

//     assert_eq!(contract.version(), 130);
//     assert_eq!(token.version(), 130);
// }

// #[test]
// fn test_regular_upgrade_token() {
//     let setup = Setup::default();
//     let contract = setup.liq_pool;
//     let token = LpTokenClient::new(&setup.env, &contract.share_id());

//     let token_wasm = setup
//         .env
//         .deployer()
//         .upload_contract_wasm(pool_tokens::token::WASM);
//     let new_wasm = install_dummy_wasm(&setup.env);

//     // dummy wasm has version 130, everything else has greater version
//     assert_eq!(contract.get_emergency_mode(), false);
//     assert_ne!(contract.version(), 130);
//     assert_ne!(token.version(), 130);

//     contract.commit_upgrade(&setup.admin, &new_wasm, &token_wasm);
//     assert!(contract.try_apply_upgrade(&setup.admin).is_err());
//     jump(&setup.env, ADMIN_ACTIONS_DELAY + 1);
//     assert_eq!(
//         contract.apply_upgrade(&setup.admin),
//         (new_wasm.clone(), token_wasm.clone())
//     );

//     assert_eq!(contract.version(), 130);
//     assert_ne!(token.version(), 130);
// }

// #[test]
// fn test_regular_upgrade_pool() {
//     let setup = Setup::default();
//     let contract = setup.liq_pool;
//     let token = ShareTokenClient::new(&setup.env, &contract.share_id());

//     let new_wasm = install_dummy_wasm(&setup.env);
//     let new_token_wasm = install_dummy_wasm(&setup.env);

//     // dummy wasm has version 130, everything else has greater version
//     assert_eq!(contract.get_emergency_mode(), false);
//     assert_ne!(contract.version(), 130);
//     assert_ne!(token.version(), 130);

//     contract.commit_upgrade(&setup.admin, &new_wasm, &new_token_wasm);
//     assert!(contract.try_apply_upgrade(&setup.admin).is_err());
//     jump(&setup.env, ADMIN_ACTIONS_DELAY + 1);
//     assert_eq!(
//         contract.apply_upgrade(&setup.admin),
//         (new_wasm.clone(), new_token_wasm.clone())
//     );

//     assert_eq!(contract.version(), 130);
//     assert_eq!(token.version(), 130);
// }

// // // #[test]
// // // fn test_claim_event() {
// // //     let setup = Setup::default();
// // //     let liq_pool = setup.liq_pool;
// // //     let token_1_admin_client = SorobanTokenAdminClient::new(
// // //         &setup.env,
// // //         &setup.token1.address.clone()
// // //     );
// // //     let token_2_admin_client = SorobanTokenAdminClient::new(
// // //         &setup.env,
// // //         &setup.token2.address.clone()
// // //     );
// // //     let token_reward_admin_client = SorobanTokenAdminClient::new(
// // //         &setup.env,
// // //         &setup.token_reward.address.clone()
// // //     );

// // //     let user = Address::generate(&setup.env);

// // //     // token_1_admin_client.mint(&user, &1000);
// // //     token_2_admin_client.mint(&user, &1000);
// // //     liq_pool.deposit(&user, &1000, &0);
// // //     token_reward_admin_client.mint(&liq_pool.address, &1_000_000_0000000);
// // //     let reward_1_tps = 10_5000000_u128;
// // //     let total_reward_1 = reward_1_tps * 70;
// // //     liq_pool.set_incentives_config(
// // //         &setup.admin,
// // //         &setup.env.ledger().timestamp().saturating_add(70),
// // //         &reward_1_tps
// // //     );
// // //     jump(&setup.env, 70);
// // //     liq_pool.claim(&user);

// // //     assert_eq!(
// // //         vec![&setup.env, setup.env.events().all().last().unwrap()],
// // //         vec![&setup.env, (
// // //             liq_pool.address.clone(),
// // //             (
// // //                 Symbol::new(&setup.env, "claim_reward"),
// // //                 setup.token_reward.address.clone(),
// // //                 user.clone(),
// // //             ).into_val(&setup.env),
// // //             (total_reward_1 as i128,).into_val(&setup.env),
// // //         )]
// // //     );
// // // }
