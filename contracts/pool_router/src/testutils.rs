#![cfg(test)]
extern crate std;

use crate::testutils::oracle_registry::{
    OracleGuardRails,
    PriceDivergenceGuardRails,
    ValidityGuardRails,
};
use std::vec;
use crate::PoolRouterClient;
use sep_40_oracle::testutils::{ Asset as MockAsset, MockPriceOracleClient, MockPriceOracleWASM };
use soroban_sdk::testutils::{ Address as _, Ledger, LedgerInfo };
use soroban_sdk::{ Address, Env, Symbol, Vec, BytesN };
use soroban_sdk::token::{
    StellarAssetClient as SorobanTokenAdminClient,
    TokenClient as SorobanTokenClient,
};
use utils::test_utils::jump;

pub(crate) fn get_token_admin_client<'a>(
    e: &Env,
    address: &Address
) -> SorobanTokenAdminClient<'a> {
    SorobanTokenAdminClient::new(e, address)
}

pub(crate) mod test_token {
    use soroban_sdk::contractimport;
    contractimport!(file = "../../target/wasm32v1-none/release/soroban_token_contract.wasm");
}

pub fn create_token_contract<'a>(e: &Env, admin: &Address) -> SorobanTokenClient<'a> {
    SorobanTokenClient::new(e, &e.register_stellar_asset_contract_v2(admin.clone()).address())
}

pub mod pool_plane {
    soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/pool_plane.wasm");
}

pub fn create_plane_contract<'a>(e: &Env) -> pool_plane::Client<'a> {
    pool_plane::Client::new(e, &e.register(pool_plane::WASM, ()))
}

pub mod liquidity_calculator {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/liquidity_calculator.wasm"
    );
}

pub fn create_liquidity_calculator_contract<'a>(e: &Env) -> liquidity_calculator::Client<'a> {
    liquidity_calculator::Client::new(e, &e.register(liquidity_calculator::WASM, ()))
}

pub mod pool {
    soroban_sdk::contractimport!(file = "../../wasm/pool.wasm");
}

pub fn install_liq_pool_hash(e: &Env) -> BytesN<32> {
    e.deployer().upload_contract_wasm(pool::WASM)
}

pub fn install_lp_token_wasm(e: &Env) -> BytesN<32> {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/lp_token.wasm"
    );
    e.deployer().upload_contract_wasm(WASM)
}

pub fn install_token_wasm(e: &Env) -> BytesN<32> {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/soroban_token_contract.wasm"
    );
    e.deployer().upload_contract_wasm(WASM)
}

// (https://github.com/script3/sep-40-oracle/blob/d2d9a19079d95f79c16c3ff506416346d75b537f/mock-sep-40/src/test.rs)
pub fn setup_price_feed_oracle<'a>(
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

pub mod oracle_registry {
    soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/oracle_registry.wasm");
}

pub fn create_oracle_registry_contract<'a>(e: &Env) -> oracle_registry::Client<'a> {
    oracle_registry::Client::new(e, &e.register(oracle_registry::WASM, ()))
}

pub fn create_pool_router_contract<'a>(e: &Env) -> PoolRouterClient<'a> {
    let router = PoolRouterClient::new(e, &e.register(crate::PoolRouter {}, ()));
    router
}

pub(crate) struct Setup<'a> {
    pub(crate) env: Env,

    // addresses
    pub(crate) users: vec::Vec<Address>,
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
    // pub(crate) tokens: [SorobanTokenClient<'a>; 4],
    // pub(crate) reward_token: SorobanTokenClient<'a>,

    // tokens
    pub(crate) token1: SorobanTokenClient<'a>,
    pub(crate) token1_admin_client: SorobanTokenAdminClient<'a>,
    pub(crate) token2: SorobanTokenClient<'a>,
    pub(crate) token2_admin_client: SorobanTokenAdminClient<'a>,

    // oracle
    pub(crate) oracle: Address,
    pub(crate) btc_asset_id: Symbol,
    pub(crate) xlm_asset_id: Symbol,
    // pub(crate) btc_addr: Address,
    // pub(crate) btc_asset: Symbol,
    // pub(crate) btc_asset: MockAsset,
    // pub(crate) eth_addr: Address,
    // pub(crate) eth_asset: Symbol,
    // pub(crate) eth_asset: MockAsset,
}

impl Default for Setup<'_> {
    fn default() -> Self {
        Self::setup()
    }
    // Create setup from default config and mint tokens for all users & set rewards config
}

impl Setup<'_> {
    pub(crate) fn setup() -> Self {
        let e = Env::default();
        // e.mock_all_auths();
        e.mock_all_auths_allowing_non_root_auth();
        e.cost_estimate().budget().reset_unlimited();

        assert_eq!(e.auths(), []);

        let start_time = 1753288157;
        jump(&e, start_time);

        let users = Self::generate_random_users(&e, 3);
        let admin = Address::generate(&e);
        let asset = Address::generate(&e);

        let mut token1 = create_token_contract(&e, &admin);
        let mut token2 = create_token_contract(&e, &admin);
        let tokens = Vec::from_array(&e, [token1.address.clone(), token2.address.clone()]);
        // let reward_token = create_token_contract(&e, &rewards_admin);

        let token1_admin_client = get_token_admin_client(&e, &token1.address.clone());
        let token2_admin_client = get_token_admin_client(&e, &token2.address.clone());
        // let token_reward_admin_client = get_token_admin_client(&e, &reward_token.address.clone());

        let mut tokens = std::vec![
            create_token_contract(&e, &admin).address,
            create_token_contract(&e, &admin).address,
            create_token_contract(&e, &admin).address,
            create_token_contract(&e, &admin).address
        ];
        tokens.sort();
        // let tokens = [
        //     // test_token::Client::new(&e, &tokens[0]),
        //     // test_token::Client::new(&e, &tokens[1]),
        //     // test_token::Client::new(&e, &tokens[2]),
        //     // test_token::Client::new(&e, &tokens[3]),
        // ];

        let reward_admin = Address::generate(&e);
        let admin = Address::generate(&e);

        let reward_token = create_token_contract(&e, &reward_admin);

        // Pool Router
        let pool_hash = install_liq_pool_hash(&e);
        let lp_token_hash = install_lp_token_wasm(&e);
        let synthetic_token_hash = install_token_wasm(&e);
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
        router.set_lp_token_hash(&admin, &lp_token_hash);
        router.set_synthetic_token_hash(&admin, &synthetic_token_hash);
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

        // Setup oracles
        let btc_asset_id = Symbol::new(&e, "BTC");
        let xlm_asset_id = Symbol::new(&e, "XLM");

        let btc_asset = MockAsset::Other(btc_asset_id.clone());
        let xlm_asset = MockAsset::Other(xlm_asset_id.clone());

        let base = MockAsset::Other(Symbol::new(&e, "USD"));

        let (oracle_id, oracle_client) = setup_price_feed_oracle(
            &e,
            &admin,
            &base,
            &Vec::from_array(&e, [btc_asset.clone(), xlm_asset.clone()]),
            14,
            300
        );

        // ===

        // let prices_1: Vec<i128> = vec![&e, 94_234_1234567, 1_1021304];
        let prices_1: Vec<i128> = Vec::from_array(&e, [118_115_40000000000000, 0_42190000000000]);
        oracle_client.set_price(&prices_1, &start_time);

        // verify price data can be fetched
        let result_1 = oracle_client.lastprice(&btc_asset).unwrap();
        assert_eq!(result_1.price, prices_1.get_unchecked(0));
        // assert_eq!(result_1.timestamp, start_time);

        let result_2 = oracle_client.lastprice(&xlm_asset).unwrap();
        assert_eq!(result_2.price, prices_1.get_unchecked(1));
        // assert_eq!(result_2.timestamp, start_time);

        // let last_timestamp = oracle_client.last_timestamp();
        // assert_eq!(last_timestamp, start_time);

        // pass time
        // jump(&e, 325);
        // env.ledger().set(LedgerInfo {
        //     timestamp: start_time + 325,
        //     protocol_version: 22,
        //     sequence_number: start_block + 325 / 5,
        //     network_id: Default::default(),
        //     base_reserve: 10,
        //     min_temp_entry_ttl: 16,
        //     min_persistent_entry_ttl: 4096,
        //     max_entry_ttl: 6312000,
        // });

        // verify price data can still be fetched and timestamp does not change
        // let result_1 = oracle_client.lastprice(&asset_1).unwrap();
        // assert_eq!(result_1.price, prices_1.get_unchecked(0));
        // assert_eq!(result_1.timestamp, start_time);

        // let result_2 = oracle_client.lastprice(&asset_2).unwrap();
        // assert_eq!(result_2.price, prices_1.get_unchecked(1));
        // assert_eq!(result_2.timestamp, start_time);

        // let last_timestamp = oracle_client.last_timestamp();
        // assert_eq!(last_timestamp, start_time);

        // set another round of prices
        // let prices_2: Vec<i128> = Vec::from_array(&e, [10923722794294087742, 10923722794294087742]);
        // oracle_client.set_price(&prices_2, &(start_time + 300));

        // // verify most recent prices are fetched
        // let result_1 = oracle_client.lastprice(&asset_1).unwrap();
        // assert_eq!(result_1.price, prices_2.get_unchecked(0));
        // assert_eq!(result_1.timestamp, start_time + 300);

        // let result_2 = oracle_client.lastprice(&asset_2).unwrap();
        // assert_eq!(result_2.price, prices_2.get_unchecked(1));
        // assert_eq!(result_2.timestamp, start_time + 300);

        // let last_timestamp = oracle_client.last_timestamp();
        // assert_eq!(last_timestamp, start_time + 300);

        // // verify old prices can be fetched
        // let result_1 = oracle_client.price(&asset_1, &start_time).unwrap();
        // assert_eq!(result_1.price, prices_1.get_unchecked(0));
        // assert_eq!(result_1.timestamp, start_time);

        // let result_2 = oracle_client.price(&asset_2, &start_time).unwrap();
        // assert_eq!(result_2.price, prices_1.get_unchecked(1));
        // assert_eq!(result_2.timestamp, start_time);

        // // verify timestamp is normalized to the most recent price
        // // older than the requested timestamp
        // let result_1 = oracle_client.price(&asset_1, &(100 + start_time)).unwrap();
        // assert_eq!(result_1.price, prices_1.get_unchecked(0));
        // assert_eq!(result_1.timestamp, start_time);

        // let result_2 = oracle_client.price(&asset_2, &(250 + start_time)).unwrap();
        // assert_eq!(result_2.price, prices_1.get_unchecked(1));
        // assert_eq!(result_2.timestamp, start_time);

        // // verify get prices can fetch both
        // let result_1_vec = oracle_client.prices(&asset_1, &2).unwrap();
        // assert_eq!(result_1_vec.len(), 2);
        // let result_1_0 = result_1_vec.get_unchecked(0);
        // assert_eq!(result_1_0.price, prices_2.get_unchecked(0));
        // assert_eq!(result_1_0.timestamp, start_time + 300);
        // let result_1_1 = result_1_vec.get_unchecked(1);
        // assert_eq!(result_1_1.price, prices_1.get_unchecked(0));
        // assert_eq!(result_1_1.timestamp, start_time);

        // let result_2_vec = oracle_client.prices(&asset_2, &2).unwrap();
        // assert_eq!(result_2_vec.len(), 2);
        // let result_2_0 = result_2_vec.get_unchecked(0);
        // assert_eq!(result_2_0.price, prices_2.get_unchecked(1));
        // assert_eq!(result_2_0.timestamp, start_time + 300);
        // let result_2_1 = result_2_vec.get_unchecked(1);
        // assert_eq!(result_2_1.price, prices_1.get_unchecked(1));
        // assert_eq!(result_2_1.timestamp, start_time);

        // // verify un-normalized timestamps get set to the most recent normalized timestamp
        // let prices_3: Vec<i128> = Vec::from_array(&e, [10923722794294087742, 10923722794294087742]);
        // oracle_client.set_price(&prices_3, &(start_time + 600 + 100));

        // let result_1 = oracle_client.lastprice(&asset_1).unwrap();
        // assert_eq!(result_1.price, prices_3.get_unchecked(0));
        // assert_eq!(result_1.timestamp, start_time + 600);

        // let result_2 = oracle_client.lastprice(&asset_2).unwrap();
        // assert_eq!(result_2.price, prices_3.get_unchecked(1));
        // assert_eq!(result_2.timestamp, start_time + 600);

        // let last_timestamp = oracle_client.last_timestamp();
        // assert_eq!(last_timestamp, start_time + 600);

        // ===

        // Setup Oracle Registry
        let registry = create_oracle_registry_contract(&e);
        registry.initialize(&admin, &emergency_admin);
        registry.set_oracle_guard_rails(
            &admin,
            &(OracleGuardRails {
                price_divergence: PriceDivergenceGuardRails {
                    oracle_twap_percent_divergence: 120_0000000,
                },
                validity: ValidityGuardRails {
                    seconds_before_stale_for_pool: 3000, // ~5 seconds
                    too_volatile_ratio: 120_0000000, // 5x or 80% down
                },
            })
        );

        router.set_oracle_registry(&admin, &registry.address);

        // Register XLM oracle
        registry.register_oracle(&admin, &btc_asset_id, &oracle_id, &14, &0);

        // Register BTC oralce
        registry.register_oracle(&admin, &xlm_asset_id, &oracle_id, &14, &0);

        Setup {
            env: e,
            admin,

            asset,
            users,
            // tokens,
            // reward_token,

            router,
            registry,

            emergency_admin,
            rewards_admin,
            operations_admin,
            pause_admin,
            emergency_pause_admin,

            // tokens
            token1,
            token1_admin_client,
            token2,
            token2_admin_client,

            // oracle
            oracle: oracle_id,
            btc_asset_id,
            xlm_asset_id,
            // btc_addr,
            // btc_asset,
            // btc_asset,
            // eth_addr,
            // eth_asset,
            // eth_asset,
        }
    }

    pub(crate) fn generate_random_users(e: &Env, users_count: u32) -> vec::Vec<Address> {
        let mut users = vec![];
        for _c in 0..users_count {
            users.push(Address::generate(e));
        }
        users
    }
}
