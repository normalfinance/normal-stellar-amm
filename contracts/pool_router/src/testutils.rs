#![cfg(test)]
extern crate std;

use crate::testutils::oracle_registry::{
    OracleGuardRails,
    PriceDivergenceGuardRails,
    ValidityGuardRails,
};
use crate::PoolRouterClient;
use sep_40_oracle::testutils::{ Asset as MockAsset, MockPriceOracleClient, MockPriceOracleWASM };
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{ Address, Env, Symbol, Vec };
use utils::constant::PERCENTAGE_PRECISION_U64;
use utils::test_utils::{ install_liq_pool_hash, install_token_wasm };

pub(crate) mod test_token {
    use soroban_sdk::contractimport;
    contractimport!(file = "../../target/wasm32v1-none/release/soroban_token_contract.wasm");
}

pub fn create_token_contract<'a>(e: &Env, admin: &Address) -> test_token::Client<'a> {
    test_token::Client::new(e, &e.register_stellar_asset_contract_v2(admin.clone()).address())
}

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
    pub(crate) registry: oracle_registry::Client<'a>,

    // tokens
    pub(crate) tokens: [test_token::Client<'a>; 4],
    pub(crate) reward_token: test_token::Client<'a>,

    // oracle
    pub(crate) btc_addr: Address,
    pub(crate) btc_asset: Symbol,
    pub(crate) btc_asset: MockAsset,
    pub(crate) eth_addr: Address,
    pub(crate) eth_asset: Symbol,
    pub(crate) eth_asset: MockAsset,
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
            test_token::Client::new(&e, &tokens[0]),
            test_token::Client::new(&e, &tokens[1]),
            test_token::Client::new(&e, &tokens[2]),
            test_token::Client::new(&e, &tokens[3]),
        ];

        let reward_admin = Address::generate(&e);
        let admin = Address::generate(&e);

        let reward_token = create_token_contract(&e, &reward_admin);

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

        let btc_asset = Symbol::new(&e, "BTC");
        let eth_asset = Symbol::new(&e, "ETH");

        let btc_asset = MockAsset::Stellar(btc_addr.clone());
        let eth_asset = MockAsset::Stellar(eth_addr.clone());

        let (oracle_id, oracle_client) = setup_price_feed_oracle(
            &e,
            &admin,
            &MockAsset::Other(Symbol::new(&e, "USD")),
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
        registry.set_oracle_guard_rails(
            &admin,
            &(OracleGuardRails {
                price_divergence: PriceDivergenceGuardRails {
                    oracle_twap_percent_divergence: PERCENTAGE_PRECISION_U64 / 2,
                },
                validity: ValidityGuardRails {
                    seconds_before_stale_for_pool: 10, // ~5 seconds
                    too_volatile_ratio: 5, // 5x or 80% down
                },
            })
        );
        registry.register_oracle(&admin, &btc_asset, &oracle, &14, &0);

        Setup {
            env: e,
            admin,

            asset,
            tokens,
            reward_token,

            router,
            registry,

            emergency_admin,
            rewards_admin,
            operations_admin,
            pause_admin,
            emergency_pause_admin,

            // oracle
            btc_addr,
            btc_asset,
            btc_asset,
            eth_addr,
            eth_asset,
            eth_asset,
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

pub fn create_liquidity_calculator_contract<'a>(e: &Env) -> liquidity_calculator::Client<'a> {
    liquidity_calculator::Client::new(e, &e.register(liquidity_calculator::WASM, ()))
}

pub mod pool {
    soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/pool.wasm");
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

mod oracle_registry {
    soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/oracle_registry.wasm");
}

pub fn create_oracle_registry_contract<'a>(e: &Env) -> oracle_registry::Client<'a> {
    oracle_registry::Client::new(e, &e.register(oracle_registry::WASM, ()))
}

pub fn create_pool_router_contract<'a>(e: &Env) -> PoolRouterClient<'a> {
    let router = PoolRouterClient::new(e, &e.register(crate::PoolRouter {}, ()));
    router
}
