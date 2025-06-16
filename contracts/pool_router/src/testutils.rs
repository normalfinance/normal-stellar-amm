#![cfg(test)]
extern crate std;

use crate::PoolRouterClient;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::token::{ TokenClient as SorobanTokenClient };
use soroban_sdk::{ Address, BytesN, Env, Symbol, Vec };
use utils::test_utils::{
    create_token_contract,
    get_mock_lp_token_info,
    get_mock_oracle_registry_ids,
    install_liq_pool_hash,
    install_token_wasm,
};

pub(crate) struct Setup<'a> {
    pub(crate) env: Env,

    // addresses
    pub(crate) admin: Address,
    pub(crate) asset: Address,
    pub(crate) emergency_admin: Address,
    pub(crate) rewards_admin: Address,
    pub(crate) operations_admin: Address,
    pub(crate) pause_admin: Address,
    pub(crate) emergency_pause_admin: Address,

    // contracts
    pub(crate) router: PoolRouterClient<'a>,

    // tokens
    pub(crate) tokens: [SorobanTokenClient<'a>; 4],
    pub(crate) reward_token: SorobanTokenClient<'a>,
}

impl Default for Setup<'_> {
    // Create setup from default config and mint tokens for all users & set rewards config
    fn default() -> Self {
        let e = Env::default();
        e.mock_all_auths();
        e.cost_estimate().budget().reset_unlimited();

        let admin = Address::generate(&e);
        let asset = Address::generate(&e);

        let mut tokens = std::vec![
            create_token_contract(&e, &admin).address,
            create_token_contract(&e, &admin).address,
            create_token_contract(&e, &admin).address,
            create_token_contract(&e, &admin).address
        ];
        tokens.sort();
        let tokens = [
            SorobanTokenClient::new(&e, &tokens[0]),
            SorobanTokenClient::new(&e, &tokens[1]),
            SorobanTokenClient::new(&e, &tokens[2]),
            SorobanTokenClient::new(&e, &tokens[3]),
        ];

        let reward_admin = Address::generate(&e);
        let admin = Address::generate(&e);

        let reward_token = create_token_contract(&e, &reward_admin);

        let pool_hash = install_liq_pool_hash(&e);
        let token_hash = install_token_wasm(&e);
        let router = create_pool_router_contract(&e);
        router.init_admin(&admin);
        let rewards_admin = soroban_sdk::Address::generate(&e);
        let operations_admin = soroban_sdk::Address::generate(&e);
        let pause_admin = soroban_sdk::Address::generate(&e);
        let emergency_pause_admin = soroban_sdk::Address::generate(&e);
        router.set_privileged_addrs(
            &admin,
            &rewards_admin,
            &operations_admin,
            &pause_admin,
            &Vec::from_array(&e, [emergency_pause_admin.clone()])
        );
        router.set_pool_hash(&admin, &pool_hash);
        router.set_token_hash(&admin, &token_hash);
        router.set_reward_token(&admin, &reward_token.address);

        let emergency_admin = Address::generate(&e);
        router.commit_transfer_ownership(
            &admin,
            &Symbol::new(&e, "EmergencyAdmin"),
            &emergency_admin
        );
        router.apply_transfer_ownership(&admin, &Symbol::new(&e, "EmergencyAdmin"));

        let plane = create_plane_contract(&env);
        router.set_pools_plane(&admin, &plane.address);

        let liquidity_calculator = create_liquidity_calculator_contract(&env);
        liquidity_calculator.init_admin(&admin);
        liquidity_calculator.set_pools_plane(&admin, &plane.address);
        router.set_liquidity_calculator(&admin, &liquidity_calculator.address);

        Setup {
            env: e,
            admin,

            asset,
            tokens,
            reward_token,
            router,
            emergency_admin,
            rewards_admin,
            operations_admin,
            pause_admin,
            emergency_pause_admin,
        }
    }
}

mod pool_plane {
    soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/pool_plane.wasm");
}

pub fn create_plane_contract<'a>(e: &Env) -> pool_plane::Client<'a> {
    pool_plane::Client::new(e, &e.register(pool_plane::WASM, ()))
}

mod liquidity_calculator {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/liquidity_calculator.wasm"
    );
}

pub fn create_pool_router_contract<'a>(e: &Env) -> PoolRouterClient<'a> {
    let router = PoolRouterClient::new(e, &e.register(crate::PoolRouter {}, ()));
    router
}

// // create swap pool & deposit initial liquidity
// pub fn setup_mock_pool<'a>(
//     e: &Env,
//     setup: &Setup<'a>,
//     tokens: Option<Vec<Address>
//     // router: &PoolRouterClient<'a>,
//     // admin: &Address,
//     // asset: &Address,
//     // tokens: &Vec<Address>,
//     // token_client: &SorobanTokenAdminClient<'a>
// ) -> (BytesN<32>, Address, pool::Client) {
//     let tokens = if Some(tokens) {}

//     let (pool_hash, pool_address) = setup.router.init_pool(
//         &setup.admin,
//         &get_mock_oracle_registry_ids(&e),
//         asset,
//         tokens,
//         &get_mock_lp_token_info(&e),
//         &30,
//         &PoolTier::A,
//         &1_000_000u128
//     );
//     let pool = pool::Client::new(&e, &pool_address);
//     token_client.mint(&admin, &1_000_000_000_0000000);
//     pool.deposit(&admin, &1_000_000_000_0000000);

//     (pool_hash, pool_address, pool)
// }
