#![cfg(test)]
extern crate std;
use crate::PoolSwapFeeCollectorClient;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::token::{
    StellarAssetClient as SorobanTokenAdminClient,
    TokenClient as SorobanTokenClient,
};
use utils::constant::PERCENTAGE_PRECISION_U64;
use utils::storage::PoolTier;
use crate::testutils::oracle_registry::{
    OracleGuardRails,
    PriceDivergenceGuardRails,
    ValidityGuardRails,
};
use sep_40_oracle::testutils::{ Asset as MockAsset, MockPriceOracleClient, MockPriceOracleWASM };
use soroban_sdk::{ Address, Env, Symbol, Vec };
use utils::test_utils::{
    create_token_contract,
    get_mock_lp_token_info,
    get_mock_oracle_registry_ids,
    get_token_admin_client,
    install_liq_pool_hash,
    install_token_wasm,
};

pub(crate) struct TestConfig {
    pub(crate) max_provider_fee: u32,
}

impl Default for TestConfig {
    fn default() -> Self {
        TestConfig {
            max_provider_fee: 100, // 1% fee
        }
    }
}

pub(crate) struct Setup<'a> {
    pub(crate) env: Env,

    // addresses
    pub(crate) admin: Address,
    pub(crate) fee_destination: Address,

    // contracts
    pub(crate) fee_collector: PoolSwapFeeCollectorClient<'a>,
    pub(crate) router: pool_router::Client<'a>,
    pub(crate) buffer: Address,
    pub(crate) registry: oracle_registry::Client<'a>,

    // tokens
    pub(crate) token_a: SorobanTokenClient<'a>,
    pub(crate) token_a_admin_client: SorobanTokenAdminClient<'a>,
    pub(crate) token_b: SorobanTokenClient<'a>,
    pub(crate) token_b_admin_client: SorobanTokenAdminClient<'a>,
}

impl Default for Setup<'_> {
    // Create setup from default config
    fn default() -> Self {
        let default_config = TestConfig::default();
        Self::new_with_config(&default_config)
    }
}

impl Setup<'_> {
    pub(crate) fn new_with_config(config: &TestConfig) -> Self {
        let setup = Self::setup(config);
        setup
    }

    pub(crate) fn setup(config: &TestConfig) -> Self {
        let e: Env = Env::default();
        e.mock_all_auths();
        e.cost_estimate().budget().reset_unlimited();

        let admin = Address::generate(&e);
        let fee_destination = Address::generate(&e);
        let asset = Address::generate(&e);

        let token_a = create_token_contract(&e, &admin);
        let token_b = create_token_contract(&e, &admin);
        let tokens = Vec::from_array(&e, [token_a.address.clone(), token_b.address.clone()]);

        let reward_token = create_token_contract(&e, &reward_admin);

        let token_a_admin_client = get_token_admin_client(&e, &token_a.address.clone());
        let token_b_admin_client = get_token_admin_client(&e, &token_b.address.clone());

        // Setup auxilary contracts
        // Pool Router
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

        // Pool Plane
        let plane = create_plane_contract(&e);
        router.set_pools_plane(&admin, &plane.address);

        // Liquidity Calculator
        let liquidity_calculator = create_liquidity_calculator_contract(&e);
        liquidity_calculator.init_admin(&admin);
        liquidity_calculator.set_pools_plane(&admin, &plane.address);
        router.set_liquidity_calculator(&admin, &liquidity_calculator.address);

        // Oracle Registry
        let btc_addr = Address::generate(&e);
        let eth_addr = Address::generate(&e);
        let btc_asset_id = Symbol::new(&e, "BTC");
        let eth_asset_id = Symbol::new(&e, "ETH");
        let btc_asset = MockAsset::Stellar(btc_addr.clone());
        let eth_asset = MockAsset::Stellar(eth_addr.clone());

        let usdc_addr = Address::generate(&e);

        let base = MockAsset::Other(Symbol::new(&e, "USD"));

        let (oracle_id, oracle_client) = setup_price_feed_oracle(
            &e,
            &admin,
            &base,
            &Vec::from_array(&e, [btc_asset.clone(), eth_asset.clone()]),
            7,
            300
        );

        // prices
        let start_time = e.ledger().timestamp();
        let init_btc_price = 50000_0000000_i128; // $50,000
        let init_eth_price = 3000_0000000_i128; // $3,000
        let prices: Vec<i128> = Vec::from_array(&e, [init_btc_price, init_eth_price]);
        oracle_client.set_price(&prices, &start_time);

        let registry = create_oracle_registry_contract(&e);
        registry.initialize(&admin, &emergency_admin);
        registry.set_oracle_guardrails(
            &admin,
            &(OracleGuardRails {
                price_divergence: PriceDivergenceGuardRails {
                    oracle_twap_percent_divergence: PERCENTAGE_PRECISION_U64 / 2,
                },
                validity: ValidityGuardRails {
                    slots_before_stale_for_pool: 10, // ~5 seconds
                    confidence_interval_max_size: 20_000, // 2% of price
                    too_volatile_ratio: 5, // 5x or 80% down
                },
            })
        );
        registry.register_oracle(&admin, &btc_asset_id, &oracle_id, &btc_addr, &7, &0);

        // Buffer
        let buffer = Address::generate(&e);

        // Pool Swap Fee
        let fee_collector = create_pool_swap_fee_contract(&e);
        fee_collector.init_admin(&admin);
        fee_collector.set_router(&admin, &router.address);
        fee_collector.set_buffer(&admin, &buffer);
        fee_collector.set_fee_destination(&admin, &fee_destination);

        //  pool
        let (pool_hash, pool_address) = router.init_pool(
            &user1,
            &get_mock_oracle_registry_ids(&e),
            &btc_addr,
            &tokens,
            &get_mock_lp_token_info(&e),
            &30,
            &PoolTier::A,
            &1_000_000_u128
        );

        Self {
            env: e,
            admin,
            fee_destination,
            fee_collector,
            router,
            buffer,
            token_a,
            token_a_admin_client,
            token_b,
            token_b_admin_client,
        }
    }
}

pub mod pool {
    soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/pool.wasm");
}

mod pool_swap_fee {
    soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/pool_swap_fee.wasm");
}

pub fn create_pool_swap_fee_contract<'a>(e: &Env) -> PoolSwapFeeCollectorClient<'a> {
    PoolSwapFeeCollectorClient::new(e, &e.register(crate::PoolSwapFeeCollector, ()))
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

pub mod pool_router {
    soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/pool_router.wasm");
}

fn create_pool_router_contract<'a>(e: Env) -> pool_router::Client<'a> {
    pool_router::Client::new(&e, &e.register(pool_router::WASM, ()))
}

mod oracle_registry {
    soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/oracle_registry.wasm");
}

pub fn create_oracle_registry_contract<'a>(e: &Env) -> oracle_registry::Client<'a> {
    oracle_registry::Client::new(e, &e.register(oracle_registry::WASM, ()))
}

// (https://github.com/script3/sep-40-oracle/blob/d2d9a19079d95f79c16c3ff506416346d75b537f/mock-sep-40/src/test.rs)
fn setup_price_feed_oracle<'a>(
    env: &Env,
    admin: &Address,
    base: &MockAsset,
    assets: &Vec<MockAsset>,
    decimals: u32,
    resolution: u32
) -> (Address, MockPriceOracleClient<'a>) {
    let oracle_id = env.register(MockPriceOracleWASM, ());
    let oracle_client = MockPriceOracleClient::new(env, &oracle_id);
    oracle_client.set_data(admin, base, assets, &decimals, &resolution);
    (oracle_id, oracle_client)
}
