#![cfg(test)]
extern crate std;

use crate::testutils::Setup;
use crate::testutils::{self, create_plane_contract};
use access_control::constants::ADMIN_ACTIONS_DELAY;
use soroban_sdk::testutils::{
    AuthorizedFunction, AuthorizedInvocation, Events, MockAuth, MockAuthInvoke,
};
use soroban_sdk::{log, String};
use soroban_sdk::{
    symbol_short, testutils::Address as _, vec, Address, FromVal, IntoVal, Map, Symbol, Val, Vec,
    U256,
};
use utils::state::pool::PoolTier;
// use utils::test_utils::{
//     assert_approx_eq_abs,
//     assert_approx_eq_abs_u256,
//     get_mock_assets,
//     get_mock_lp_token_info,
//     install_dummy_wasm,
//     jump,
// };

#[test]
fn test_pool() {
    let setup = Setup::default();
    let e = setup.env;
    let router = setup.router;
    // let [token1, token2, _, _] = setup.tokens;

    let tokens = Vec::from_array(
        &e,
        [setup.token1.address.clone(), setup.token2.address.clone()],
    );

    let sac = Address::generate(&e);

    // let user1 = Address::generate(&e);
    // setup.reward_token.mint(&user1, &10_0000000);

    let pool_address = router.init_pool(
        &setup.admin,
        &(Symbol::new(&e, "BTC"), Symbol::new(&e, "XLM")),
        &setup.token2.address.clone(),
        &sac,
        &(
            String::from_str(&e, "Pool Share Token"),
            String::from_str(&e, "Pool Share Token"),
        ),
        &30,
        &PoolTier::A,
        &1_000_000_u128,
    );

    // Depopsit
    setup
        .token2_admin_client
        .mint(&setup.users[1], &100_000_0000000);

    // let (x, y) = router.deposit(&setup.users[1], &setup.btc_asset_id, &10000_0000000);

    // let router_str: String = router.address.to_string(&e).into();

    // log!(e, "router", router.address);
    // log!(e, "pool", pool_address);

    // assert_eq!(router_str, "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAFCT4");
    // assert_eq!(
    //     router.address.to_string(),
    //     "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAFCT4"
    // );

    // router.swap()

    assert_eq!(e.auths(), []);
    // let reserves = router.get_reserves(&setup.btc_asset_id);

    // assert_eq!(router.get_total_shares(&setup.btc_asset_id), 10000_0000000);
    // assert_eq!(reserves.get(0).unwrap(), 0_3570000);
    // assert_eq!(reserves.get(1).unwrap(), 10000_0000000);
}

// #[test]
// #[should_panic(expected = "Error(Contract, #103)")]
// fn test_init_admin_twice() {
//     let setup = Setup::default();
//     setup.router.init_admin(&setup.admin);
// }

// #[test]
// fn test_pool() {
//     let setup = Setup::default();
//     let e = setup.env;
//     let router = setup.router;
//     let [token1, token2, _, _] = setup.tokens;

//     let tokens = Vec::from_array(&e, [token1.address.clone(), token2.address.clone()]);

//     let user1 = Address::generate(&e);
//     setup.reward_token.mint(&user1, &10_0000000);

//     let pool_address = router.init_pool(
//         &user1,
//         &get_mock_assets(&e),
//         &tokens,
//         &get_mock_lp_token_info(&e),
//         &30,
//         &PoolTier::A,
//         &1_000_000_u128,
//     );

//     let pools = router.get_pools();

//     assert!(pools.contains(pool_address.clone()));
//     // assert_eq!(pools.get(pool_hash.clone()).unwrap(), pool_address);

//     let token_share = test_token::Client::new(&e, &router.share_id(&setup.btc_asset));

//     token2.mint(&user1, &1000);
//     assert_eq!(token2.balance(&user1), 1000);

//     assert_eq!(token_share.balance(&user1), 0);

//     let desired_amount = 100;
//     router.deposit(&user1, &setup.btc_asset, &desired_amount);
//     // assert_eq!(router.get_total_liquidity(&tokens), U256::from_u32(&e, 2));

//     assert_eq!(token_share.balance(&user1), 100);
//     assert_eq!(router.get_total_shares(&setup.btc_asset), 100);
//     assert_eq!(token_share.balance(&pool_address), 0);
//     assert_eq!(token1.balance(&user1), 0);

//     assert_eq!(token1.balance(&pool_address), 10);
//     assert_eq!(token2.balance(&user1), 900);
//     assert_eq!(token2.balance(&pool_address), 100);

//     assert_eq!(
//         router.get_reserves(&setup.btc_asset),
//         Vec::from_array(&e, [10, 100])
//     );

//     assert_eq!(
//         router.estimate_swap(
//             &tokens,
//             &token2.address,
//             &token1.address,
//             &setup.btc_asset,
//             &97
//         ),
//         (48, 0)
//     );
//     assert_eq!(
//         router.swap(
//             &user1,
//             &tokens,
//             &token2.address,
//             &token1.address,
//             &setup.btc_asset,
//             &97_u128,
//             &48_u128
//         ),
//         48
//     );

//     assert_eq!(token1.balance(&user1), 948);
//     assert_eq!(token1.balance(&pool_address), 52);
//     assert_eq!(token2.balance(&user1), 803);
//     assert_eq!(token2.balance(&pool_address), 197);
//     assert_eq!(
//         router.get_reserves(&setup.btc_asset),
//         Vec::from_array(&e, [52, 197])
//     );

//     assert_eq!(
//         router.estimate_swap(
//             &tokens,
//             &token1.address,
//             &token2.address,
//             &setup.btc_asset,
//             &97
//         ),
//         (48, 0)
//     );
//     assert_eq!(
//         router.swap(
//             &user1,
//             &tokens,
//             &token1.address,
//             &token2.address,
//             &setup.btc_asset,
//             &97_u128,
//             &48_u128
//         ),
//         48
//     );

//     assert_eq!(token1.balance(&user1), 803);
//     assert_eq!(token1.balance(&pool_address), 197);
//     assert_eq!(token2.balance(&user1), 948);
//     assert_eq!(token2.balance(&pool_address), 52);
//     assert_eq!(
//         router.get_reserves(&setup.btc_asset),
//         Vec::from_array(&e, [197, 52])
//     );

//     router.withdraw(
//         &user1,
//         &setup.btc_asset,
//         &100_u128, // &Vec::from_array(&e, [197_u128, 52_u128])
//     );

//     assert_eq!(token1.balance(&user1), 1000);
//     assert_eq!(token2.balance(&user1), 1000);
//     assert_eq!(token_share.balance(&user1), 0);
//     assert_eq!(token1.balance(&pool_address), 0);
//     assert_eq!(token2.balance(&pool_address), 0);
//     assert_eq!(token_share.balance(&pool_address), 0);
// }

// #[test]
// fn test_add_pool_after_removal() {
//     let setup = Setup::default();
//     let e = setup.env;
//     let router = setup.router;
//     let [token1, token2, _, _] = setup.tokens;

//     let tokens = Vec::from_array(&e, [token1.address.clone(), token2.address.clone()]);
//     let user1 = Address::generate(&e);
//     setup.reward_token.mint(&user1, &10_0000000);

//     let pool_address = router.init_pool(
//         &user1,
//         &get_mock_assets(&e),
//         &tokens,
//         &get_mock_lp_token_info(&e),
//         &30,
//         &PoolTier::A,
//         &1_000_000_u128,
//     );
//     assert!(router.try_remove_pool(&user1, &setup.btc_asset).is_err());
//     assert!(router
//         .try_remove_pool(&setup.rewards_admin, &setup.btc_asset)
//         .is_err());
//     router.remove_pool(&setup.operations_admin, &setup.btc_asset);
//     let pool_address_new = router.init_pool(
//         &user1,
//         &get_mock_assets(&e),
//         &tokens,
//         &get_mock_lp_token_info(&e),
//         &30,
//         &PoolTier::A,
//         &1_000_000_u128,
//     );
//     // assert_eq!(pool_hash, pool_hash_new);
//     assert_ne!(pool_address, pool_address_new);
// }

// #[test]
// fn test_init_pool_twice() {
//     let setup = Setup::default();
//     let e = setup.env;
//     let router = setup.router;
//     let [token1, token2, _, _] = setup.tokens;
//     let reward_token = setup.reward_token;

//     let tokens = Vec::from_array(&e, [token1.address.clone(), token2.address.clone()]);

//     let user1 = Address::generate(&e);
//     reward_token.mint(&user1, &10_0000000);

//     let pool_address1 = router.init_pool(
//         &user1,
//         &get_mock_assets(&e),
//         &tokens,
//         &get_mock_lp_token_info(&e),
//         &30,
//         &PoolTier::A,
//         &1_000_000_u128,
//     );
//     let pool_address2 = router.init_pool(
//         &user1,
//         &get_mock_assets(&e),
//         &tokens,
//         &get_mock_lp_token_info(&e),
//         &30,
//         &PoolTier::A,
//         &1_000_000_u128,
//     );
//     assert_eq!(pool_address1, pool_address2);

//     let pools = router.get_pools();
//     assert_eq!(pools.len(), 1);
// }

// #[should_panic(expected = "Error(WasmVm, MissingValue)")]
// #[test]
// fn test_init_pool_bad_tokens() {
//     let setup = Setup::default();
//     let e = setup.env;
//     let router = setup.router;
//     let [token1, _, _, _] = setup.tokens;
//     let reward_token = setup.reward_token;

//     let tokens = Vec::from_array(
//         &e,
//         [
//             token1.address.clone(),
//             create_plane_contract(&e).address.clone(),
//         ],
//     );

//     let user1 = Address::generate(&e);
//     reward_token.mint(&user1, &10_0000000);

//     router.init_pool(
//         &user1,
//         &get_mock_assets(&e),
//         &tokens,
//         &get_mock_lp_token_info(&e),
//         &30,
//         &PoolTier::A,
//         &1_000_000_u128,
//     );
// }

// #[test]
// fn test_simple_ongoing_reward() {
//     let setup = Setup::default();
//     let e = setup.env;
//     let router = setup.router;
//     let admin = setup.admin;
//     let [token1, token2, _, _] = setup.tokens;
//     let reward_token = setup.reward_token;

//     let tokens = Vec::from_array(&e, [token1.address.clone(), token2.address.clone()]);

//     let user1 = Address::generate(&e);

//     reward_token.mint(&user1, &1000_0000000);
//     reward_token.mint(&router.address, &2_000_000_0000000);
//     reward_token.mint(&admin, &2_000_000_0000000);

//     let pool_address = router.init_pool(
//         &user1,
//         &get_mock_assets(&e),
//         &tokens,
//         &get_mock_lp_token_info(&e),
//         &30,
//         &PoolTier::A,
//         &1_000_000_u128,
//     );

//     let reward_1_tps = 10_5000000_u128;
//     let total_reward_1 = reward_1_tps * 60;

//     token2.mint(&user1, &2000);
//     assert_eq!(token2.balance(&user1), 2000);

//     assert_eq!(router.get_total_accumulated_reward(&setup.btc_asset), 0);
//     assert_eq!(router.get_total_claimed_reward(&setup.btc_asset), 0);
//     assert_eq!(router.get_total_configured_reward(&setup.btc_asset), 0);
//     assert_eq!(router.get_total_outstanding_reward(&setup.btc_asset), 0);

//     // 10 seconds passed since config, user depositing
//     jump(&e, 10);
//     router.deposit(&user1, &setup.btc_asset, &1000);
//     let standard_liquidity = router.get_total_liquidity(&setup.btc_asset);
//     assert_eq!(standard_liquidity, U256::from_u32(&e, 34));

//     assert_eq!(router.get_total_accumulated_reward(&setup.btc_asset), 0);
//     assert_eq!(router.get_total_claimed_reward(&setup.btc_asset), 0);
//     assert_eq!(router.get_total_configured_reward(&setup.btc_asset), 0);

//     let rewards = Vec::from_array(&e, [setup.btc_asset.clone()]);
//     router.config_global_rewards(
//         &admin,
//         &reward_1_tps,
//         &e.ledger().timestamp().saturating_add(60),
//         &rewards,
//     );
//     e.cost_estimate().budget().reset_default();
//     router.fill_liquidity(&setup.btc_asset);
//     e.cost_estimate().budget().print();
//     e.cost_estimate().budget().reset_default();
//     let pool_tps = router.config_pool_rewards(&setup.btc_asset);
//     // e.cost_estimate().budget().print();
//     // e.cost_estimate().budget().reset_unlimited();

//     assert_approx_eq_abs_u256(
//         U256::from_u128(&e, total_reward_1)
//             .mul(&standard_liquidity)
//             .div(&standard_liquidity),
//         U256::from_u128(&e, pool_tps * 60),
//         U256::from_u32(&e, 100),
//     );

//     assert_eq!(reward_token.balance(&user1), 0);
//     // 30 seconds passed, half of the reward is available for the user
//     jump(&e, 30);

//     assert_eq!(
//         router.get_total_accumulated_reward(&setup.btc_asset),
//         pool_tps * 30
//     );
//     assert_eq!(router.get_total_claimed_reward(&setup.btc_asset), 0);
//     assert_eq!(
//         router.get_total_configured_reward(&setup.btc_asset),
//         pool_tps * 60
//     );
//     assert_eq!(
//         router.get_total_outstanding_reward(&setup.btc_asset),
//         pool_tps * 60
//     );

//     assert_eq!(reward_token.balance(&pool_address), 0);
//     assert_eq!(
//         router.distribute_outstanding_reward(&admin, &router.address, &setup.btc_asset),
//         pool_tps * 60
//     );
//     // distribute second part from admin's balance

//     assert_eq!(
//         router.distribute_outstanding_reward(&admin, &router.address, &setup.btc_asset),
//         0
//     );

//     assert_eq!(reward_token.balance(&pool_address) as u128, pool_tps * 60);

//     assert_eq!(router.claim(&user1, &setup.btc_asset), pool_tps * 30);

//     assert_eq!(
//         router.get_total_accumulated_reward(&setup.btc_asset),
//         pool_tps * 30
//     );
//     assert_eq!(
//         router.get_total_claimed_reward(&setup.btc_asset),
//         pool_tps * 30
//     );
//     assert_eq!(
//         router.get_total_configured_reward(&setup.btc_asset),
//         pool_tps * 60
//     );

//     assert_approx_eq_abs(
//         reward_token.balance(&user1) as u128,
//         total_reward_1 / 2,
//         100,
//     );
//     jump(&e, 60);
//     router.claim(&user1, &setup.btc_asset);
//     assert_approx_eq_abs(reward_token.balance(&user1) as u128, total_reward_1, 100);

//     assert_eq!(
//         router.get_total_accumulated_reward(&setup.btc_asset),
//         pool_tps * 60
//     );
//     assert_eq!(
//         router.get_total_claimed_reward(&setup.btc_asset),
//         pool_tps * 60
//     );
//     assert_eq!(
//         router.get_total_configured_reward(&setup.btc_asset),
//         pool_tps * 60
//     );
// }

// #[test]
// fn test_rewards_distribution() {
//     let setup = Setup::default();
//     let e = setup.env;
//     let router = setup.router;
//     let admin = setup.admin;
//     let [token1, token2, _, _] = setup.tokens;
//     let reward_token = setup.reward_token;

//     let user1 = Address::generate(&e);

//     let tokens1 = Vec::from_array(&e, [token1.address.clone(), token2.address.clone()]);
//     let tokens2 = Vec::from_array(&e, [token1.address.clone(), reward_token.address.clone()]);

//     reward_token.mint(&user1, &2000_0000000);
//     reward_token.mint(&router.address, &2_000_000_0000000);

//     let pool_address1 = router.init_pool(
//         &user1,
//         &get_mock_assets(&e),
//         &tokens1,
//         &get_mock_lp_token_info(&e),
//         &30,
//         &PoolTier::A,
//         &1_000_000_u128,
//     );
//     let pool_address2 = router.init_pool(
//         &user1,
//         &(Symbol::new(&e, "ETH"), Symbol::new(&e, "XLM")),
//         &tokens2,
//         &get_mock_lp_token_info(&e),
//         &30,
//         &PoolTier::A,
//         &1_000_000_u128,
//     );

//     let reward_tps = 10_5000000_u128;

//     token2.mint(&user1, &2000);
//     reward_token.mint(&user1, &2000);

//     assert_eq!(router.get_total_outstanding_reward(&setup.btc_asset), 0);
//     assert_eq!(router.get_total_outstanding_reward(&setup.eth_asset), 0);

//     // 10 seconds passed since config, user depositing
//     jump(&e, 10);
//     router.deposit(&user1, &setup.btc_asset, &1000);
//     router.deposit(&user1, &setup.eth_asset, &1000);
//     let standard_liquidity1 = router.get_total_liquidity(&setup.btc_asset);
//     let standard_liquidity2 = router.get_total_liquidity(&setup.eth_asset);
//     assert_eq!(standard_liquidity1, U256::from_u32(&e, 34));
//     assert_eq!(standard_liquidity2, U256::from_u32(&e, 34));

//     let rewards = Vec::from_array(
//         &e,
//         [(tokens1.clone(), 0_5000000), (tokens2.clone(), 0_5000000)],
//     );
//     router.config_global_rewards(
//         &admin,
//         &reward_tps,
//         &e.ledger().timestamp().saturating_add(60),
//         &rewards,
//     );
//     router.fill_liquidity(&setup.btc_asset);
//     router.fill_liquidity(&setup.eth_asset);
//     let pool_tps1 = router.config_pool_rewards(&setup.btc_asset);
//     let pool_tps2 = router.config_pool_rewards(&setup.eth_asset);
//     assert_eq!(pool_tps1, pool_tps2);

//     let pool_tps = pool_tps1;

//     assert_eq!(reward_token.balance(&user1), 0);
//     // 30 seconds passed, half of the reward is available for the user
//     jump(&e, 30);

//     assert_eq!(
//         router.get_total_accumulated_reward(&setup.btc_asset),
//         pool_tps * 30
//     );
//     assert_eq!(
//         router.get_total_configured_reward(&setup.btc_asset),
//         pool_tps * 60
//     );
//     assert_eq!(
//         router.get_total_outstanding_reward(&setup.btc_asset),
//         pool_tps * 60
//     );

//     assert_eq!(
//         router.get_total_accumulated_reward(&setup.eth_asset),
//         pool_tps * 30
//     );
//     assert_eq!(
//         router.get_total_configured_reward(&setup.eth_asset),
//         pool_tps * 60
//     );
//     assert_eq!(
//         router.get_total_outstanding_reward(&setup.eth_asset),
//         pool_tps * 60
//     );

//     assert_eq!(reward_token.balance(&pool_address1), 0);
//     assert_eq!(reward_token.balance(&pool_address2), 1000);
//     assert_eq!(
//         router.distribute_outstanding_reward(&admin, &router.address, &setup.btc_asset),
//         pool_tps * 60
//     );

//     assert_eq!(
//         router.distribute_outstanding_reward(&admin, &router.address, &setup.eth_asset),
//         pool_tps * 60
//     );
//     assert_eq!(
//         router.distribute_outstanding_reward(&admin, &router.address, &setup.btc_asset),
//         0
//     );
//     assert_eq!(
//         router.distribute_outstanding_reward(&admin, &router.address, &setup.eth_asset),
//         0
//     );

//     // deposit again to check how reserves being calculated
//     token2.mint(&user1, &2000);
//     reward_token.mint(&user1, &2000);
//     router.deposit(&user1, &setup.btc_asset, &1000);
//     router.deposit(&user1, &setup.eth_asset, &1000);

//     // reward balance of pools2 equals to total reward + reserves
//     assert_eq!(reward_token.balance(&pool_address1) as u128, pool_tps * 60);
//     assert_eq!(
//         reward_token.balance(&pool_address2) as u128,
//         pool_tps * 60 + 2000
//     );

//     // reserves don't include rewards
//     assert_eq!(
//         router.get_reserves(&setup.btc_asset),
//         Vec::from_array(&e, [2000, 2000])
//     );
//     assert_eq!(
//         router.get_reserves(&setup.eth_asset),
//         Vec::from_array(&e, [2000, 2000])
//     );
// }

// #[test]
// fn test_rewards_distribution_as_operator() {
//     let setup = Setup::default();
//     let e = setup.env;
//     let router = setup.router;
//     let admin = setup.admin;
//     let [token1, token2, _, _] = setup.tokens;
//     let reward_token = setup.reward_token;

//     let tokens = Vec::from_array(&e, [token1.address.clone(), token2.address.clone()]);

//     let user1 = Address::generate(&e);

//     reward_token.mint(&user1, &1000_0000000);
//     reward_token.mint(&router.address, &2_000_000_0000000);
//     reward_token.mint(&admin, &2_000_000_0000000);

//     let _pool_address = router.init_pool(
//         &user1,
//         &get_mock_assets(&e),
//         &tokens,
//         &get_mock_lp_token_info(&e),
//         &30,
//         &PoolTier::A,
//         &1_000_000_u128,
//     );

//     let reward_1_tps = 10_5000000_u128;

//     token2.mint(&user1, &2000);

//     // 10 seconds passed since config, user depositing
//     jump(&e, 10);
//     router.deposit(&user1, &setup.btc_asset, &1000);

//     let rewards = Vec::from_array(&e, [setup.btc_asset.clone()]);
//     router.config_global_rewards(
//         &admin,
//         &reward_1_tps,
//         &e.ledger().timestamp().saturating_add(60),
//         &rewards,
//     );
//     router.fill_liquidity(&setup.btc_asset);
//     let pool_tps = router.config_pool_rewards(&setup.btc_asset);

//     // 30 seconds passed, half of the reward is available for the user
//     jump(&e, 30);

//     // operator not set yet. admin should be able to distribute rewards but no one else should
//     let operator = Address::generate(&e);
//     assert!(router
//         .try_distribute_outstanding_reward(&user1, &router.address, &setup.btc_asset)
//         .is_err());
//     assert!(router
//         .try_distribute_outstanding_reward(&operator, &router.address, &setup.btc_asset)
//         .is_err());
//     router.set_privileged_addrs(
//         &admin,
//         &operator,
//         &admin,
//         &admin,
//         &Vec::from_array(&e, [admin.clone()]),
//     );
//     assert!(router
//         .try_distribute_outstanding_reward(&user1, &router.address, &setup.btc_asset)
//         .is_err());
//     assert_eq!(
//         router.distribute_outstanding_reward(&operator, &router.address, &setup.btc_asset),
//         pool_tps * 60
//     );
// }

// #[test]
// fn test_rewards_distribution_override() {
//     let setup = Setup::default();
//     let e = setup.env;
//     let router = setup.router;
//     let admin = setup.admin;
//     let [token1, token2, _, _] = setup.tokens;
//     let reward_token = setup.reward_token;

//     let tokens = Vec::from_array(&e, [token1.address.clone(), token2.address.clone()]);

//     let user1 = Address::generate(&e);

//     reward_token.mint(&user1, &1000_0000000);
//     reward_token.mint(&router.address, &2_000_000_0000000);
//     reward_token.mint(&admin, &2_000_000_0000000);

//     let _pool_address = router.init_pool(
//         &user1,
//         &get_mock_assets(&e),
//         &tokens,
//         &get_mock_lp_token_info(&e),
//         &30,
//         &PoolTier::A,
//         &1_000_000_u128,
//     );

//     let reward_1_tps = 10_5000000_u128;

//     token2.mint(&user1, &2000);

//     // 10 seconds passed since config, user depositing
//     jump(&e, 10);
//     router.deposit(&user1, &setup.btc_asset, &1000);

//     let rewards = Vec::from_array(&e, [setup.btc_asset.clone()]);
//     router.config_global_rewards(
//         &admin,
//         &reward_1_tps,
//         &e.ledger().timestamp().saturating_add(60),
//         &rewards,
//     );
//     router.fill_liquidity(&setup.btc_asset);
//     let pool_tps = router.config_pool_rewards(&setup.btc_asset);

//     // 30 seconds passed, half of the reward is available
//     jump(&e, 30);

//     // tps * 60 configured in total & outstanding since there were no claims
//     assert_eq!(
//         router.get_total_configured_reward(&setup.btc_asset),
//         pool_tps * 60
//     );

//     // however since just 30 seconds passed, only half of the reward accumulated
//     assert_eq!(
//         router.get_total_accumulated_reward(&setup.btc_asset),
//         pool_tps * 30
//     );

//     router.config_global_rewards(
//         &admin,
//         &0,
//         &e.ledger().timestamp().saturating_add(10),
//         &rewards,
//     );
//     router.fill_liquidity(&setup.btc_asset);
//     router.config_pool_rewards(&setup.btc_asset);

//     // half of the reward accumulated
//     assert_eq!(
//         router.get_total_accumulated_reward(&setup.btc_asset),
//         pool_tps * 30
//     );

//     // but since we've re-configured reward in the middle, the total configured reward should be tps * 30 as well as outstanding balance
//     assert_eq!(
//         router.get_total_configured_reward(&setup.btc_asset),
//         pool_tps * 30
//     );
//     assert_eq!(
//         router.get_total_outstanding_reward(&setup.btc_asset),
//         pool_tps * 30
//     );

//     // operator not set yet. admin should be able to distribute rewards but no one else should
//     let rewards_admin = Address::generate(&e);
//     router.set_privileged_addrs(
//         &admin,
//         &rewards_admin,
//         &admin,
//         &admin,
//         &Vec::from_array(&e, [admin.clone()]),
//     );
//     assert_eq!(
//         router.distribute_outstanding_reward(&rewards_admin, &router.address, &setup.btc_asset),
//         pool_tps * 30
//     );
// }

// #[test]
// #[should_panic(expected = "Error(Contract, #309)")]
// fn test_liqidity_not_filled() {
//     let setup = Setup::default();
//     let e = setup.env;
//     let router = setup.router;
//     let admin = setup.admin;
//     let [token1, token2, _, _] = setup.tokens;
//     let reward_token = setup.reward_token;

//     let tokens = Vec::from_array(&e, [token1.address.clone(), token2.address.clone()]);

//     let user1 = Address::generate(&e);

//     let _pool_address = router.init_pool(
//         &user1,
//         &get_mock_assets(&e),
//         &tokens,
//         &get_mock_lp_token_info(&e),
//         &30,
//         &PoolTier::A,
//         &1_000_000_u128,
//     );

//     token2.mint(&user1, &2000);

//     router.deposit(&user1, &setup.btc_asset, &1000);
//     let rewards = Vec::from_array(&e, [setup.btc_asset.clone()]);
//     router.config_global_rewards(
//         &admin,
//         &1,
//         &e.ledger().timestamp().saturating_add(60),
//         &rewards,
//     );
//     router.config_pool_rewards(&setup.btc_asset);
// }

// #[test]
// #[should_panic(expected = "Error(Contract, #310)")]
// fn test_fill_liqidity_reentrancy() {
//     let setup = Setup::default();
//     let e = setup.env;
//     let router = setup.router;
//     let admin = setup.admin;
//     let [token1, token2, _, _] = setup.tokens;
//     let reward_token = setup.reward_token;

//     let tokens = Vec::from_array(&e, [token1.address.clone(), token2.address.clone()]);

//     let user1 = Address::generate(&e);

//     let _pool_address = router.init_pool(
//         &user1,
//         &get_mock_assets(&e),
//         &tokens,
//         &get_mock_lp_token_info(&e),
//         &30,
//         &PoolTier::A,
//         &1_000_000_u128,
//     );
//     token2.mint(&user1, &2000);

//     router.deposit(&user1, &setup.btc_asset, &1000);
//     let rewards = Vec::from_array(&e, [setup.btc_asset.clone()]);
//     router.config_global_rewards(
//         &admin,
//         &1,
//         &e.ledger().timestamp().saturating_add(60),
//         &rewards,
//     );
//     router.fill_liquidity(&setup.btc_asset);
//     router.fill_liquidity(&setup.btc_asset);
// }

// #[test]
// #[should_panic(expected = "Error(Contract, #314)")]
// fn test_config_pool_rewards_reentrancy() {
//     let setup = Setup::default();
//     let e = setup.env;
//     let router = setup.router;
//     let admin = setup.admin;
//     let [token1, token2, _, _] = setup.tokens;
//     let reward_token = setup.reward_token;

//     let tokens = Vec::from_array(&e, [token1.address.clone(), token2.address.clone()]);

//     let user1 = Address::generate(&e);

//     let _pool_address = router.init_pool(
//         &user1,
//         &get_mock_assets(&e),
//         &tokens,
//         &get_mock_lp_token_info(&e),
//         &30,
//         &PoolTier::A,
//         &1_000_000_u128,
//     );
//     token2.mint(&user1, &2000);

//     router.deposit(&user1, &setup.btc_asset, &1000);
//     let rewards = Vec::from_array(&e, [setup.btc_asset.clone()]);
//     router.config_global_rewards(
//         &admin,
//         &1,
//         &e.ledger().timestamp().saturating_add(60),
//         &rewards,
//     );
//     router.fill_liquidity(&setup.btc_asset);
//     router.config_pool_rewards(&setup.btc_asset);
//     router.config_pool_rewards(&setup.btc_asset);
// }

// #[test]
// fn test_config_pool_rewards_after_new_global_config() {
//     let setup = Setup::default();
//     let e = setup.env;
//     let router = setup.router;
//     let admin = setup.admin;
//     let [token1, token2, _, _] = setup.tokens;
//     let reward_token = setup.reward_token;

//     let tokens = Vec::from_array(&e, [token1.address.clone(), token2.address.clone()]);

//     let user1 = Address::generate(&e);

//     let _pool_address = router.init_pool(
//         &user1,
//         &get_mock_assets(&e),
//         &tokens,
//         &get_mock_lp_token_info(&e),
//         &30,
//         &PoolTier::A,
//         &1_000_000_u128,
//     );
//     token2.mint(&user1, &2000);

//     router.deposit(&user1, &setup.btc_asset, &1000);
//     let rewards = Vec::from_array(&e, [setup.btc_asset.clone()]);
//     router.config_global_rewards(
//         &admin,
//         &1,
//         &e.ledger().timestamp().saturating_add(60),
//         &rewards,
//     );
//     router.fill_liquidity(&setup.btc_asset);
//     assert_eq!(router.config_pool_rewards(&setup.btc_asset), 1);

//     jump(&e, 300);
//     router.config_global_rewards(
//         &admin,
//         &1,
//         &e.ledger().timestamp().saturating_add(60),
//         &rewards,
//     );
//     router.fill_liquidity(&setup.btc_asset);
//     assert_eq!(router.config_pool_rewards(&setup.btc_asset), 1);
// }

// #[test]
// fn test_config_pool_after_liquidity_fill() {
//     // if pool is created after liquidity filled for tokens, it may be configured, but should receive no rewards

//     let setup = Setup::default();
//     let e = setup.env;
//     let router = setup.router;
//     let admin = setup.admin;
//     let [token1, token2, _, _] = setup.tokens;
//     let reward_token = setup.reward_token;

//     let tokens = Vec::from_array(&e, [token1.address.clone(), token2.address.clone()]);

//     let user1 = Address::generate(&e);

//     token2.mint(&user1, &2000);

//     router.init_pool(
//         &user1,
//         &get_mock_assets(&e),
//         &tokens,
//         &get_mock_lp_token_info(&e),
//         &30,
//         &PoolTier::A,
//         &1_000_000_u128,
//     );
//     router.deposit(&user1, &setup.btc_asset, &1000);

//     let rewards = Vec::from_array(&e, [setup.btc_asset.clone()]);
//     router.config_global_rewards(
//         &admin,
//         &1_0000000,
//         &e.ledger().timestamp().saturating_add(60),
//         &rewards,
//     );
//     router.fill_liquidity(&setup.btc_asset);
//     assert_eq!(router.config_pool_rewards(&tokens, &pool_1_hash), 1_0000000);

//     let (pool_2_hash, _pool_2_address) = router.init_pool(
//         &user1,
//         &get_mock_assets(&e),
//         &tokens,
//         &get_mock_lp_token_info(&e),
//         &10,
//         &PoolTier::A,
//         &1_000_000_u128,
//     );
//     router.deposit(&user1, &tokens, &pool_2_hash, &1000);
//     assert_eq!(router.config_pool_rewards(&tokens, &pool_2_hash), 0);
// }

// #[test]
// #[should_panic(expected = "Error(Contract, #313)")]
// fn test_fill_liquidity_no_config() {
//     let setup = Setup::default();
//     let e = setup.env;
//     let router = setup.router;
//     let admin = setup.admin;
//     let [token1, token2, _, _] = setup.tokens;
//     let reward_token = setup.reward_token;

//     let tokens = Vec::from_array(&e, [token1.address.clone(), token2.address.clone()]);

//     let user1 = Address::generate(&e);

//     let _pool_address = router.init_pool(
//         &user1,
//         &get_mock_assets(&e),
//         &tokens,
//         &get_mock_lp_token_info(&e),
//         &30,
//         &PoolTier::A,
//         &1_000_000_u128,
//     );
//     token2.mint(&user1, &2000);

//     router.deposit(&user1, &setup.btc_asset, &1000);
//     router.fill_liquidity(&setup.btc_asset);
// }

// #[test]
// #[should_panic(expected = "Error(Contract, #102)")]
// fn test_config_rewards_not_admin() {
//     let setup = Setup::default();
//     let e = setup.env;
//     let router = setup.router;
//     let admin = setup.admin;
//     let [token1, token2, _, _] = setup.tokens;
//     let reward_token = setup.reward_token;

//     let tokens = Vec::from_array(&e, [token1.address.clone(), token2.address.clone()]);

//     let user1 = Address::generate(&e);

//     reward_token.mint(&user1, &1000_0000000);
//     router.init_pool(
//         &user1,
//         &get_mock_assets(&e),
//         &tokens,
//         &get_mock_lp_token_info(&e),
//         &30,
//         &PoolTier::A,
//         &1_000_000_u128,
//     );

//     let rewards = Vec::from_array(&e, [setup.btc_asset.clone()]);
//     router.config_global_rewards(
//         &user1,
//         &1,
//         &e.ledger().timestamp().saturating_add(60),
//         &rewards,
//     );
// }

// #[test]
// #[should_panic(expected = "Error(Contract, #315)")]
// fn test_config_rewards_duplicated_tokens() {
//     let setup = Setup::default();
//     let e = setup.env;
//     let router = setup.router;
//     let admin = setup.admin;
//     let [token1, token2, _, _] = setup.tokens;
//     let reward_token = setup.reward_token;

//     let tokens = Vec::from_array(&e, [token1.address.clone(), token2.address.clone()]);

//     let user1 = Address::generate(&e);

//     reward_token.mint(&user1, &1000_0000000);
//     router.init_pool(
//         &user1,
//         &get_mock_assets(&e),
//         &tokens,
//         &get_mock_lp_token_info(&e),
//         &30,
//         &PoolTier::A,
//         &1_000_000_u128,
//     );

//     let rewards = Vec::from_array(
//         &e,
//         [(
//             Vec::from_array(&e, [token1.address.clone(), token1.address.clone()]),
//             1_0000000,
//         )],
//     );
//     router.config_global_rewards(
//         &admin,
//         &1,
//         &e.ledger().timestamp().saturating_add(60),
//         &rewards,
//     );
// }

// #[test]
// #[should_panic(expected = "Error(Contract, #2002)")]
// fn test_config_rewards_tokens_not_sorted() {
//     let setup = Setup::default();
//     let e = setup.env;
//     let router = setup.router;
//     let admin = setup.admin;
//     let [token1, token2, _, _] = setup.tokens;
//     let reward_token = setup.reward_token;

//     let tokens = Vec::from_array(&e, [token1.address.clone(), token2.address.clone()]);

//     let user1 = Address::generate(&e);

//     reward_token.mint(&user1, &1000_0000000);
//     router.init_pool(
//         &user1,
//         &get_mock_assets(&e),
//         &tokens,
//         &get_mock_lp_token_info(&e),
//         &30,
//         &PoolTier::A,
//         &1_000_000_u128,
//     );

//     let rewards = Vec::from_array(
//         &e,
//         [(
//             Vec::from_array(&e, [token2.address, token1.address]),
//             1_0000000,
//         )],
//     );
//     router.config_global_rewards(
//         &admin,
//         &1,
//         &e.ledger().timestamp().saturating_add(60),
//         &rewards,
//     );
// }

// #[test]
// fn test_config_rewards_no_pools_for_tokens() {
//     let setup = Setup::default();
//     let e = setup.env;
//     let router = setup.router;
//     let admin = setup.admin;
//     let [token1, token2, _, _] = setup.tokens;
//     let reward_token = setup.reward_token;

//     let tokens = Vec::from_array(&e, [token1.address.clone(), token2.address.clone()]);

//     let rewards = Vec::from_array(&e, [(tokens.clone(), 1_0000000)]);
//     router.config_global_rewards(
//         &admin,
//         &1,
//         &e.ledger().timestamp().saturating_add(60),
//         &rewards,
//     );
//     assert_eq!(
//         router.get_tokens_for_reward(),
//         Map::from_array(
//             &e,
//             [(tokens.clone(), (1_0000000, false, U256::from_u32(&e, 0)))]
//         )
//     );
//     router.fill_liquidity(&setup.btc_asset);
//     assert_eq!(
//         router.get_tokens_for_reward(),
//         Map::from_array(
//             &e,
//             [(tokens.clone(), (1_0000000, true, U256::from_u32(&e, 0)))]
//         )
//     );
// }

// // TODO: change to fee over max fee
// #[test]
// #[should_panic(expected = "Error(Contract, #302)")]
// fn test_unexpected_fee() {
//     let setup = Setup::default();
//     let e = setup.env;
//     let router = setup.router;
//     let [token1, token2, _, _] = setup.tokens;
//     let reward_token = setup.reward_token;

//     let tokens = Vec::from_array(&e, [token1.address.clone(), token2.address.clone()]);

//     let user1 = Address::generate(&e);
//     reward_token.mint(&user1, &10_0000000);

//     let fee = 30 + 1;
//     router.init_pool(
//         &user1,
//         &get_mock_assets(&e),
//         &tokens,
//         &get_mock_lp_token_info(&e),
//         &fee,
//         &PoolTier::A,
//         &1_000_000_u128,
//     );
// }

// #[test]
// fn test_event_correct() {
//     let setup = Setup::default();
//     let e = setup.env;
//     let router = setup.router;
//     let admin = setup.admin;
//     let [token1, token2, _, _] = setup.tokens;
//     let reward_token = setup.reward_token;

//     let tokens = Vec::from_array(&e, [token1.address.clone(), token2.address.clone()]);

//     let user1 = Address::generate(&e);

//     reward_token.mint(&user1, &10000000_0000000);
//     let fee = 30;

//     let pool_address = router.init_pool(
//         &user1,
//         &get_mock_assets(&e),
//         &tokens,
//         &get_mock_lp_token_info(&e),
//         &fee,
//         &PoolTier::A,
//         &1_000_000_u128,
//     );

//     let init_pool_event = e.events().all().last().unwrap();

//     assert_eq!(
//         vec![&e, init_pool_event],
//         vec![
//             &e,
//             (
//                 router.address.clone(),
//                 (Symbol::new(&e, "add_pool"), tokens.clone()).into_val(&e),
//                 (
//                     pool_address.clone(),
//                     setup.btc_asset.clone(),
//                     Vec::<Val>::from_array(&e, [fee.into_val(&e)]),
//                 )
//                     .into_val(&e),
//             )
//         ]
//     );

//     reward_token.mint(&router.address, &1_000_000_0000000);
//     let reward_1_tps = 10_5000000_u128;
//     let rewards = Vec::from_array(&e, [setup.btc_asset.clone()]);
//     router.config_global_rewards(
//         &admin,
//         &reward_1_tps,
//         &e.ledger().timestamp().saturating_add(60),
//         &rewards,
//     );
//     router.fill_liquidity(&setup.btc_asset);
//     router.config_pool_rewards(&setup.btc_asset);

//     token2.mint(&user1, &1000);
//     assert_eq!(token2.balance(&user1), 1000);

//     // 10 seconds passed since config, user depositing
//     jump(&e, 10);

//     let desired_amount = 100;

//     let (amounts, share_amount) = router.deposit(&user1, &setup.btc_asset, &desired_amount);
//     let deposit_event = e.events().all().last().unwrap();
//     assert_eq!(
//         router.get_total_liquidity(&setup.btc_asset),
//         U256::from_u32(&e, 2)
//     );

//     let pool_id = router.get_pool(&setup.btc_asset);

//     assert_eq!(
//         vec![&e, deposit_event],
//         vec![
//             &e,
//             (
//                 router.address.clone(),
//                 (
//                     Symbol::new(&e, "deposit"),
//                     setup.btc_asset.clone(),
//                     user1.clone()
//                 )
//                     .into_val(&e),
//                 (pool_id.clone(), amounts, share_amount).into_val(&e),
//             )
//         ]
//     );

//     let out_amt = router.swap(
//         &user1,
//         &tokens,
//         &token2.address,
//         &token1.address,
//         &setup.btc_asset,
//         &97_u128,
//         &48_u128,
//     );
//     let swap_event = e.events().all().last().unwrap();

//     assert_eq!(
//         vec![&e, swap_event],
//         vec![
//             &e,
//             (
//                 router.address.clone(),
//                 (Symbol::new(&e, "swap"), tokens.clone(), user1.clone()).into_val(&e),
//                 (
//                     pool_id.clone(),
//                     &token2.address,
//                     &token1.address,
//                     97_u128,
//                     out_amt
//                 )
//                     .into_val(&e),
//             )
//         ]
//     );

//     let amounts = router.withdraw(
//         &user1,
//         &setup.btc_asset,
//         &100_u128, // &Vec::from_array(&e, [197_u128, 51_u128])
//     );
//     let withdraw_event = e.events().all().last().unwrap();

//     assert_eq!(
//         vec![&e, withdraw_event],
//         vec![
//             &e,
//             (
//                 router.address.clone(),
//                 (Symbol::new(&e, "withdraw"), tokens.clone(), user1.clone()).into_val(&e),
//                 (pool_id.clone(), 100_u128, amounts).into_val(&e),
//             )
//         ]
//     );
// }

// #[test]
// fn test_tokens_storage() {
//     let setup = Setup::default();
//     let e = setup.env;
//     let router = setup.router;
//     let [token1, token2, token3, _] = setup.tokens;
//     let reward_token = setup.reward_token;

//     let tokens = [
//         token1.address.clone(),
//         token2.address.clone(),
//         token3.address.clone(),
//     ];

//     let user1 = Address::generate(&e);
//     reward_token.mint(&user1, &100_0000000);

//     let pairs = [
//         Vec::from_array(&e, [tokens[0].clone(), tokens[1].clone()]),
//         Vec::from_array(&e, [tokens[1].clone(), tokens[2].clone()]),
//         Vec::from_array(&e, [tokens[0].clone(), tokens[2].clone()]),
//         Vec::from_array(
//             &e,
//             [tokens[0].clone(), tokens[1].clone(), tokens[2].clone()],
//         ),
//     ];
//     for pair in pairs.clone() {
//         if pair.len() == 2 {
//             router.init_pool(
//                 &user1,
//                 &get_mock_assets(&e),
//                 &pair,
//                 &get_mock_lp_token_info(&e),
//                 &30,
//                 &PoolTier::A,
//                 &1_000_000_u128,
//             );
//         }
//     }
//     let pools_vec = router.get_pools();
//     assert_eq!(pools_vec.len(), 4);
// }

// #[test]
// fn test_rewards_distribution_without_outstanding_rewards() {
//     let setup = Setup::default();
//     let e = setup.env;
//     let router = setup.router;
//     let admin = setup.admin;

//     let [token, _, _, _] = setup.tokens;
//     let reward_token = setup.reward_token;

//     let tokens = Vec::from_array(&e, [token.address.clone(), reward_token.address.clone()]);
//     let user = Address::generate(&e);

//     reward_token.mint(&user, &200000_0000000);
//     reward_token.mint(&router.address, &20_000_000_0000000);

//     let pool_address1 = router.init_pool(
//         &user,
//         &get_mock_assets(&e),
//         &tokens,
//         &get_mock_lp_token_info(&e),
//         &30,
//         &PoolTier::A,
//         &1_000_000_u128,
//     );

//     let reward_tps = 1_5000000_u128;

//     token.mint(&user, &(i128::MAX / 100));
//     reward_token.mint(&user, &(i128::MAX / 100));

//     // 10 seconds passed since config, user depositing
//     jump(&e, 10);
//     router.deposit(
//         &user,
//         &setup.btc_asset,
//         &2420176738, // [30399483, 2420176738]
//     );

//     reward_token.mint(&pool_address1, &(3888205486 - 2420176738));
//     let rewards = Vec::from_array(&e, [setup.btc_asset.clone()]);
//     router.config_global_rewards(
//         &admin,
//         &reward_tps,
//         &e.ledger().timestamp().saturating_add(60),
//         &rewards,
//     );

//     router.fill_liquidity(&setup.btc_asset);
//     router.config_pool_rewards(&setup.btc_asset);

//     // check that we don't need to add rewards to pool
//     assert_eq!(router.get_total_outstanding_reward(&setup.btc_asset), 0);

//     // check that it works without panicking
//     assert_eq!(
//         router.distribute_outstanding_reward(&admin, &router.address, &setup.btc_asset),
//         0
//     );
// }

// #[test]
// fn test_privileged_users() {
//     let setup = Setup::default();
//     let e = setup.env;
//     let router = setup.router;

//     let [token1, token2, _, _] = setup.tokens;
//     let reward_token = setup.reward_token;

//     let tokens = Vec::from_array(&e, [token1.address.clone(), token2.address.clone()]);

//     let user1 = Address::generate(&e);
//     reward_token.mint(&user1, &10_0000000);

//     let standard_address = router.init_pool(
//         &user1,
//         &get_mock_assets(&e),
//         &tokens,
//         &get_mock_lp_token_info(&e),
//         &30,
//         &PoolTier::A,
//         &1_000_000_u128,
//     );
//     let privileged_addrs: Map<Symbol, Vec<Address>> = Map::from_array(
//         &e,
//         [
//             (Symbol::new(&e, "Admin"), Vec::from_array(&e, [setup.admin])),
//             (
//                 Symbol::new(&e, "EmergencyAdmin"),
//                 Vec::from_array(&e, [setup.emergency_admin]),
//             ),
//             (
//                 Symbol::new(&e, "RewardsAdmin"),
//                 Vec::from_array(&e, [setup.rewards_admin]),
//             ),
//             (
//                 Symbol::new(&e, "OperationsAdmin"),
//                 Vec::from_array(&e, [setup.operations_admin]),
//             ),
//             (
//                 Symbol::new(&e, "PauseAdmin"),
//                 Vec::from_array(&e, [setup.pause_admin]),
//             ),
//             (
//                 Symbol::new(&e, "EmergencyPauseAdmin"),
//                 Vec::from_array(&e, [setup.emergency_pause_admin]),
//             ),
//         ],
//     );
//     assert_eq!(privileged_addrs, router.get_privileged_addrs());
//     // test addresses inheritance
//     assert_eq!(
//         privileged_addrs,
//         testutils::pool::Client::new(&e, &standard_address).get_privileged_addrs()
//     );
// }

// #[test]
// fn test_set_privileged_addresses_event() {
//     let setup = Setup::default();
//     let router = setup.router;

//     router.set_privileged_addrs(
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
//                 router.address.clone(),
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
// fn test_transfer_ownership_events() {
//     let setup = Setup::default();
//     let router = setup.router;
//     let new_admin = Address::generate(&setup.env);

//     router.commit_transfer_ownership(&setup.admin, &symbol_short!("Admin"), &new_admin);
//     assert_eq!(
//         vec![&setup.env, setup.env.events().all().last().unwrap()],
//         vec![
//             &setup.env,
//             (
//                 router.address.clone(),
//                 (
//                     Symbol::new(&setup.env, "commit_transfer_ownership"),
//                     symbol_short!("Admin")
//                 )
//                     .into_val(&setup.env),
//                 (new_admin.clone(),).into_val(&setup.env),
//             )
//         ]
//     );

//     router.revert_transfer_ownership(&setup.admin, &symbol_short!("Admin"));
//     assert_eq!(
//         vec![&setup.env, setup.env.events().all().last().unwrap()],
//         vec![
//             &setup.env,
//             (
//                 router.address.clone(),
//                 (
//                     Symbol::new(&setup.env, "revert_transfer_ownership"),
//                     symbol_short!("Admin")
//                 )
//                     .into_val(&setup.env),
//                 ().into_val(&setup.env),
//             )
//         ]
//     );

//     router.commit_transfer_ownership(&setup.admin, &symbol_short!("Admin"), &new_admin);
//     jump(&setup.env, ADMIN_ACTIONS_DELAY + 1);
//     router.apply_transfer_ownership(&setup.admin, &symbol_short!("Admin"));
//     assert_eq!(
//         vec![&setup.env, setup.env.events().all().last().unwrap()],
//         vec![
//             &setup.env,
//             (
//                 router.address.clone(),
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
//     let contract = setup.router;
//     let new_wasm_hash = install_dummy_wasm(&setup.env);

//     contract.commit_upgrade(&setup.admin, &new_wasm_hash);
//     assert_eq!(
//         vec![&setup.env, setup.env.events().all().last().unwrap()],
//         vec![
//             &setup.env,
//             (
//                 contract.address.clone(),
//                 (Symbol::new(&setup.env, "commit_upgrade"),).into_val(&setup.env),
//                 (new_wasm_hash.clone(),).into_val(&setup.env),
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

//     contract.commit_upgrade(&setup.admin, &new_wasm_hash);
//     jump(&setup.env, ADMIN_ACTIONS_DELAY + 1);
//     contract.apply_upgrade(&setup.admin);
//     assert_eq!(
//         vec![&setup.env, setup.env.events().all().last().unwrap()],
//         vec![
//             &setup.env,
//             (
//                 contract.address.clone(),
//                 (Symbol::new(&setup.env, "apply_upgrade"),).into_val(&setup.env),
//                 (new_wasm_hash.clone(),).into_val(&setup.env),
//             )
//         ]
//     );
// }

// #[test]
// fn test_emergency_mode_events() {
//     let setup = Setup::default();
//     let contract = setup.router;

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
//     let contract = setup.router;
//     let new_wasm = install_dummy_wasm(&setup.env);

//     assert_eq!(contract.get_emergency_mode(), false);
//     assert_ne!(contract.version(), 130);
//     contract.set_emergency_mode(&setup.emergency_admin, &true);

//     contract.commit_upgrade(&setup.admin, &new_wasm);
//     contract.apply_upgrade(&setup.admin);

//     assert_eq!(contract.version(), 130)
// }

// #[test]
// fn test_regular_upgrade() {
//     let setup = Setup::default();
//     let contract = setup.router;
//     let new_wasm = install_dummy_wasm(&setup.env);

//     assert_eq!(contract.get_emergency_mode(), false);
//     assert_ne!(contract.version(), 130);

//     contract.commit_upgrade(&setup.admin, &new_wasm);
//     assert!(contract.try_apply_upgrade(&setup.admin).is_err());
//     jump(&setup.env, ADMIN_ACTIONS_DELAY + 1);
//     contract.apply_upgrade(&setup.admin);

//     assert_eq!(contract.version(), 130)
// }
