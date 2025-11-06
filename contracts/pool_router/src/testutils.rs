#![allow(dead_code)]
#![cfg(test)]
extern crate std;

use crate::PoolRouterClient;
use pool_config_storage::testutils::deploy_config_storage;
use sep_40_oracle::testutils::{Asset as MockAsset, MockPriceOracleClient, MockPriceOracleWASM};
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, BytesN, Env, Symbol, Vec};

pub(crate) mod test_token {
    use soroban_sdk::contractimport;
    contractimport!(file = "../../target/wasm32v1-none/release/soroban_token_contract.wasm");
}

pub fn create_token_contract<'a>(e: &Env, admin: &Address) -> test_token::Client<'a> {
    test_token::Client::new(
        e,
        &e.register_stellar_asset_contract_v2(admin.clone())
            .address(),
    )
}

pub fn create_liqpool_router_contract<'a>(e: &Env) -> PoolRouterClient<'a> {
    let router = PoolRouterClient::new(e, &e.register(crate::PoolRouter {}, ()));
    router
}

pub fn install_token_wasm(e: &Env) -> BytesN<32> {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/soroban_token_contract.wasm"
    );
    e.deployer().upload_contract_wasm(WASM)
}

pub mod standard_pool {
    soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/pool.wasm");
}

pub fn install_liq_pool_hash(e: &Env) -> BytesN<32> {
    e.deployer().upload_contract_wasm(standard_pool::WASM)
}

pub mod elastic_pool {
    soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/pool_elastic.wasm");
}

pub fn install_elastic_liq_pool_hash(e: &Env) -> BytesN<32> {
    e.deployer().upload_contract_wasm(elastic_pool::WASM)
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

pub fn create_liquidity_calculator_contract<'a>(e: &Env) -> liquidity_calculator::Client<'a> {
    liquidity_calculator::Client::new(e, &e.register(liquidity_calculator::WASM, ()))
}

pub(crate) mod rewards_gauge {
    soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/rewards_gauge.wasm");
}

pub(crate) mod config_storage {
    soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/config_storage.wasm");
}

pub(crate) struct Setup<'a> {
    pub(crate) env: Env,

    pub(crate) admin: Address,

    pub(crate) tokens: [test_token::Client<'a>; 4],
    pub(crate) reward_token: test_token::Client<'a>,

    pub(crate) router: PoolRouterClient<'a>,

    pub(crate) emergency_admin: Address,
    pub(crate) rewards_admin: Address,
    pub(crate) operations_admin: Address,
    pub(crate) pause_admin: Address,
    pub(crate) emergency_pause_admin: Address,
    pub(crate) system_fee_admin: Address,

    pub(crate) oracle_addr: Address,
    pub(crate) oracle_client: MockPriceOracleClient<'a>,
    pub(crate) sol_asset: MockAsset,
    pub(crate) usdc_asset: MockAsset,
    pub(crate) sol_symbol: Symbol,
    pub(crate) usdc_symbol: Symbol,
    pub(crate) eth_symbol: Symbol,
}

impl Default for Setup<'_> {
    // Create setup from default config and mint tokens for all users & set rewards config
    fn default() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        env.cost_estimate().budget().reset_unlimited();

        // Jump time so oracle works
        let start_time = 1755271154;
        // jump(&e, start_time);

        let admin = Address::generate(&env);

        let mut tokens = std::vec![
            create_token_contract(&env, &admin).address,
            create_token_contract(&env, &admin).address,
            create_token_contract(&env, &admin).address,
            create_token_contract(&env, &admin).address,
        ];
        tokens.sort();
        let tokens = [
            test_token::Client::new(&env, &tokens[0]),
            test_token::Client::new(&env, &tokens[1]),
            test_token::Client::new(&env, &tokens[2]),
            test_token::Client::new(&env, &tokens[3]),
        ];

        let reward_admin = Address::generate(&env);
        let admin = Address::generate(&env);
        let emergency_admin = Address::generate(&env);
        let payment_for_creation_address = Address::generate(&env);

        let reward_token = create_token_contract(&env, &reward_admin);

        let pool_hash = install_liq_pool_hash(&env);
        let token_hash = install_token_wasm(&env);
        let router = create_liqpool_router_contract(&env);
        router.init_admin(&admin);
        router.init_config_storage(
            &admin,
            &deploy_config_storage(&env, &admin, &emergency_admin).address,
        );
        let rewards_admin = soroban_sdk::Address::generate(&env);
        let operations_admin = soroban_sdk::Address::generate(&env);
        let pause_admin = soroban_sdk::Address::generate(&env);
        let emergency_pause_admin = soroban_sdk::Address::generate(&env);
        let system_fee_admin = soroban_sdk::Address::generate(&env);
        router.set_privileged_addrs(
            &admin,
            &rewards_admin,
            &operations_admin,
            &pause_admin,
            &Vec::from_array(&env, [emergency_pause_admin.clone()]),
            &system_fee_admin,
        );
        router.set_pool_hash(&admin, &pool_hash);
        router.set_elastic_pool_hash(&admin, &install_elastic_liq_pool_hash(&env));
        router.set_token_hash(&admin, &token_hash);
        router.set_reward_token(&admin, &reward_token.address);
        router.set_protocol_fee_fraction(&admin, &5000);
        // min equivalent amount of 10 reward token per day. min tps is ~1157
        router.pool_gauge_set_reward_thresholds(
            &admin,
            &10_0000000,
            &(60 * 60 * 24), // 1 day in seconds
        );
        router.set_rewards_gauge_hash(
            &admin,
            &env.deployer().upload_contract_wasm(rewards_gauge::WASM),
        );

        router.commit_transfer_ownership(
            &admin,
            &Symbol::new(&env, "EmergencyAdmin"),
            &emergency_admin,
        );
        router.apply_transfer_ownership(&admin, &Symbol::new(&env, "EmergencyAdmin"));

        let plane = create_plane_contract(&env);
        router.set_pools_plane(&admin, &plane.address);

        let liquidity_calculator = create_liquidity_calculator_contract(&env);
        liquidity_calculator.init_admin(&admin);
        liquidity_calculator.set_pools_plane(&admin, &plane.address);
        router.set_liquidity_calculator(&admin, &liquidity_calculator.address);

        // Setup oracle
        let eth_symbol = Symbol::new(&env, "ETH");
        let sol_symbol = Symbol::new(&env, "SOL");
        let usdc_symbol = Symbol::new(&env, "USDC");
        let usd_sybmol = Symbol::new(&env, "USD");

        let sol_asset = MockAsset::Other(sol_symbol.clone());
        let usdc_asset = MockAsset::Other(usdc_symbol.clone());
        let usd_asset = MockAsset::Other(usd_sybmol);

        let (oracle_addr, oracle_client) = setup_price_feed_oracle(
            &env,
            &admin,
            &usd_asset,
            &Vec::from_array(&env, [sol_asset.clone(), usdc_asset.clone()]),
            14,
            300,
        );

        let prices_1: Vec<i128> = Vec::from_array(&env, [230_00000000000000, 1_00000000000000]);
        oracle_client.set_price(&prices_1, &start_time);

        // verify price data can be fetched
        let result_1 = oracle_client.lastprice(&sol_asset).unwrap();
        assert_eq!(result_1.price, prices_1.get_unchecked(0));
        // assert_eq!(result_1.timestamp, start_time);

        let result_2 = oracle_client.lastprice(&usdc_asset).unwrap();
        assert_eq!(result_2.price, prices_1.get_unchecked(1));

        Setup {
            env,
            admin,
            tokens,
            reward_token,
            router,
            emergency_admin,
            rewards_admin,
            operations_admin,
            pause_admin,
            emergency_pause_admin,
            system_fee_admin,

            oracle_addr,
            oracle_client,
            sol_asset,
            usdc_asset,
            sol_symbol,
            usdc_symbol,
            eth_symbol,
        }
    }
}

pub fn setup_price_feed_oracle<'a>(
    env: &Env,
    admin: &Address,
    base: &MockAsset,
    assets: &Vec<MockAsset>,
    decimals: u32,
    resolution: u32,
) -> (Address, MockPriceOracleClient<'a>) {
    let oracle_addr = env.register(MockPriceOracleWASM, ());
    let oracle_client = MockPriceOracleClient::new(env, &oracle_addr);
    oracle_client.set_data(admin, base, assets, &decimals, &resolution);
    (oracle_addr, oracle_client)
}
