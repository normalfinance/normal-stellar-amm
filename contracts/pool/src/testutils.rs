#![cfg(test)]
extern crate std;
use crate::testutils::oracle_registry::{
    OracleGuardRails,
    PriceDivergenceGuardRails,
    ValidityGuardRails,
};
use soroban_fixed_point_math::FixedPoint;
use crate::PoolClient;
use access_control::constants::ADMIN_ACTIONS_DELAY;
use sep_40_oracle::testutils::{ Asset as MockAsset, MockPriceOracleClient, MockPriceOracleWASM };
use soroban_sdk::token::{
    StellarAssetClient as SorobanTokenAdminClient,
    TokenClient as SorobanTokenClient,
};
use soroban_sdk::String;
use soroban_sdk::{ testutils::{Address as _, Ledger}, Address, BytesN, Env, Symbol, Vec };
use utils::constant::{ PERCENTAGE_PRECISION_U64, PRICE_PRECISION_I128 };
use utils::state::{
    pool::{ InitializeAllParams, InitializeParams, PoolTier, RewardConfig },
    access::PrivilegedAddresses,
    token::TokenInitInfo,
};

use pool_tokens::token_contract::{ Client as PoolTokenClient };
use std::vec;
use utils::test_utils::{
    create_token_contract,
    get_mock_lp_token_info,
    get_token_admin_client,
    install_liq_pool_hash,
    install_token_wasm,
    jump,
};

pub(crate) struct TestConfig {
    pub(crate) users_count: u32,
    pub(crate) mint_to_user: i128,
    pub(crate) rewards_count: i128,
    pub(crate) liq_pool_fee: u32,
    pub(crate) reward_tps: u128,
    pub(crate) reward_token_in_pool: bool,
    pub(crate) oracle_guard_rails: OracleGuardRails,
}

impl Default for TestConfig {
    fn default() -> Self {
        TestConfig {
            users_count: 2,
            mint_to_user: 1000,
            rewards_count: 1_000_000_0000000,
            liq_pool_fee: 30,
            reward_tps: 10_5000000_u128,
            reward_token_in_pool: false,
            oracle_guard_rails: OracleGuardRails {
                price_divergence: PriceDivergenceGuardRails {
                    oracle_twap_percent_divergence: PERCENTAGE_PRECISION_U64 / 10, // allows up to ±10%
                },
                validity: ValidityGuardRails {
                    seconds_before_stale_for_pool: 600,
                    too_volatile_ratio: 200, // allows up to ±100%
                },
            },
        }
    }
}

pub(crate) struct Setup<'a> {
    pub(crate) env: Env,

    // addresses
    pub(crate) admin: Address,
    pub(crate) users: vec::Vec<Address>,
    pub(crate) emergency_admin: Address,
    pub(crate) rewards_admin: Address,
    pub(crate) operations_admin: Address,
    pub(crate) pause_admin: Address,
    pub(crate) emergency_pause_admin: Address,

    // contracts
    pub(crate) liq_pool: PoolClient<'a>,
    pub(crate) router: pool_router::Client<'a>,
    pub(crate) plane: pool_plane::Client<'a>,
    pub(crate) registry: oracle_registry::Client<'a>,

    // oracle
    pub(crate) oracle: Address,
    pub(crate) oracle_client: MockPriceOracleClient<'a>,

    pub(crate) btc_addr: Address,
    pub(crate) eth_addr: Address,
    pub(crate) xlm_addr: Address,

    pub(crate) btc_asset: MockAsset,
    pub(crate) eth_asset: MockAsset,
    pub(crate) xlm_asset: MockAsset,

    pub(crate) btc_asset_id: Symbol,
    pub(crate) eth_asset_id: Symbol,
    pub(crate) xlm_asset_id: Symbol,

    pub(crate) init_btc_price: i128,
    pub(crate) init_eth_price: i128,
    pub(crate) init_xlm_price: i128,

    // tokens
    pub(crate) token1: SorobanTokenClient<'a>,
    pub(crate) token1_admin_client: SorobanTokenAdminClient<'a>,
    pub(crate) token2: SorobanTokenClient<'a>,
    pub(crate) token2_admin_client: SorobanTokenAdminClient<'a>,
    pub(crate) token_reward: SorobanTokenClient<'a>,
    pub(crate) token_reward_admin_client: SorobanTokenAdminClient<'a>,
    pub(crate) token_share: PoolTokenClient<'a>,
}

impl Default for Setup<'_> {
    // Create setup from default config and mint tokens for all users & set rewards config
    fn default() -> Self {
        let default_config = TestConfig::default();
        Self::new_with_config(&default_config)
    }
}

impl Setup<'_> {
    // Create setup from config and mint tokens for all users
    pub(crate) fn new_with_config(config: &TestConfig) -> Self {
        let setup = Self::setup(config);
        setup.mint_tokens_for_users(config.mint_to_user);
        setup.set_rewards_config(config.reward_tps);
        setup
    }

    // Create users, token1, token2, reward token, lp token
    //
    // Mint reward token (1_000_000_0000000) & approve for pool token
    pub(crate) fn setup(config: &TestConfig) -> Self {
        let e: Env = Env::default();
        e.mock_all_auths();
        e.cost_estimate().budget().reset_unlimited();
        
        // Set initial timestamp to avoid staleness during oracle validation
        e.ledger().with_mut(|li| {
            li.timestamp = 1200;
        });

        let users = Self::generate_random_users(&e, config.users_count);
        let admin = users[0].clone();
        let rewards_admin = Address::generate(&e);
        let operations_admin = Address::generate(&e);
        let pause_admin = Address::generate(&e);
        let emergency_pause_admin = Address::generate(&e);

        let mut token1 = create_token_contract(&e, &admin);
        let mut token2 = create_token_contract(&e, &admin);
        let reward_token = if config.reward_token_in_pool {
            SorobanTokenClient::new(&e, &token1.address.clone())
        } else {
            create_token_contract(&e, &admin)
        };

        if &token2.address < &token1.address {
            std::mem::swap(&mut token1, &mut token2);
        }
        let token1_admin_client = get_token_admin_client(&e, &token1.address.clone());
        let token2_admin_client = get_token_admin_client(&e, &token2.address.clone());
        let token_reward_admin_client = get_token_admin_client(&e, &reward_token.address.clone());

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
        let usdc_addr = Address::generate(&e);
        let btc_addr = Address::generate(&e);
        let eth_addr = Address::generate(&e);
        let xlm_addr = Address::generate(&e);

        let btc_asset_id = Symbol::new(&e, "BTC");
        let eth_asset_id = Symbol::new(&e, "ETH");
        let xlm_asset_id = Symbol::new(&e, "XLM");

        let btc_asset = MockAsset::Stellar(btc_addr.clone());
        let eth_asset = MockAsset::Stellar(eth_addr.clone());
        let xlm_asset = MockAsset::Stellar(xlm_addr.clone());

        let base = MockAsset::Other(Symbol::new(&e, "USD"));

        let (oracle_id, oracle_client) = setup_price_feed_oracle(
            &e,
            &admin,
            &base,
            &Vec::from_array(&e, [btc_asset.clone(), eth_asset.clone(), xlm_asset.clone()]),
            7,
            300
        );

        // prices
        let start_time = e.ledger().timestamp();
        let init_btc_price = 50000_0000000_i128; // $50,000
        let init_eth_price = 3000_0000000_i128; // $3,000
        let init_xlm_price = 0_5000000_i128; // $0.50
        let prices: Vec<i128> = Vec::from_array(&e, [
            init_btc_price,
            init_eth_price,
            init_xlm_price,
        ]);
        oracle_client.set_price(&prices, &start_time);
        // Seed a second point slightly later to reduce staleness/volatility edge cases
        let prices2: Vec<i128> = Vec::from_array(&e, [
            init_btc_price,
            init_eth_price,
            init_xlm_price,
        ]);
        oracle_client.set_price(&prices2, &(start_time + 60));

        let registry = create_oracle_registry_contract(&e);
        registry.initialize(&admin, &emergency_admin);
        registry.set_oracle_guard_rails(&admin, &config.oracle_guard_rails);

        registry.register_oracle(&admin, &btc_asset_id, &oracle_id, &btc_addr, &7, &1);

        // Pool
        let liq_pool = create_pool_contract(
            &e,
            &admin,
            &plane.address,
            &router.address,
            &(btc_asset_id.clone(), xlm_asset_id.clone()),
            &install_token_wasm(&e),
            &get_mock_lp_token_info(&e),
            &Vec::from_array(&e, [token1.address.clone(), token2.address.clone()]),
            &reward_token.address,
            config.liq_pool_fee,
            &PoolTier::A,
            1_000_000_u128
        );
        token_reward_admin_client.mint(&liq_pool.address, &config.rewards_count);

        liq_pool.set_privileged_addrs(
            &admin,
            &rewards_admin.clone(),
            &operations_admin.clone(),
            &pause_admin.clone(),
            &Vec::from_array(&e, [emergency_pause_admin.clone()])
        );

        let emergency_admin = Address::generate(&e);
        liq_pool.commit_transfer_ownership(
            &admin,
            &Symbol::new(&e, "EmergencyAdmin"),
            &emergency_admin
        );
        jump(&e, ADMIN_ACTIONS_DELAY + 1); // delay is mandatory since emergency admin was set during initialization
        liq_pool.apply_transfer_ownership(&admin, &Symbol::new(&e, "EmergencyAdmin"));

        let token_share = PoolTokenClient::new(&e, &liq_pool.share_id());

        // Set token1 admin to liquidity pool so it can mint/burn token1 on user calls
        token1_admin_client.set_admin(&liq_pool.address);

        Self {
            env: e,
            plane,
            registry,
            router,

            // oracle
            oracle: oracle_id,
            oracle_client,

            btc_addr,
            eth_addr,
            xlm_addr,

            btc_asset,
            eth_asset,
            xlm_asset,

            btc_asset_id,
            eth_asset_id,
            xlm_asset_id,

            init_btc_price,
            init_eth_price,
            init_xlm_price,

            // pool
            users,
            token1,
            token1_admin_client,
            token2,
            token2_admin_client,
            token_reward: reward_token,
            token_reward_admin_client,
            token_share,
            liq_pool,
            admin,
            emergency_admin,
            rewards_admin,
            operations_admin,
            pause_admin,
            emergency_pause_admin,
        }
    }

    pub(crate) fn target_price(setup: &Setup) -> i128 {
        let btc_price = setup.oracle_client.lastprice(&setup.btc_asset).unwrap();
        let xlm_price = setup.oracle_client.lastprice(&setup.xlm_asset).unwrap();

        xlm_price.price.fixed_div_floor(btc_price.price, PRICE_PRECISION_I128).unwrap()
    }

    pub(crate) fn generate_random_users(e: &Env, users_count: u32) -> vec::Vec<Address> {
        let mut users = vec![];
        for _c in 0..users_count {
            users.push(Address::generate(e));
        }
        users
    }

    pub(crate) fn mint_tokens_for_users(&self, amount: i128) {
        for user in self.users.iter() {
            self.token2_admin_client.mint(user, &amount);
            assert_eq!(self.token2.balance(user), amount.clone());
        }
    }

    pub(crate) fn set_rewards_config(&self, reward_tps: u128) {
        if reward_tps > 0 {
            self.liq_pool.set_incentives_config(
                &self.users[0],
                &self.env.ledger().timestamp().saturating_add(60),
                &reward_tps
            );
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

mod pool_router {
    soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/pool_router.wasm");
}

pub fn create_pool_router_contract<'a>(e: &Env) -> pool_router::Client<'a> {
    pool_router::Client::new(e, &e.register(pool_router::WASM, ()))
}

pub fn create_pool_contract<'a>(
    e: &Env,
    admin: &Address,
    plane: &Address,
    router: &Address,
    assets: &(Symbol, Symbol),
    token_wasm_hash: &BytesN<32>,
    lp_token_info: &(String, String),
    tokens: &Vec<Address>,
    reward_token: &Address,
    fee_fraction: u32,
    tier: &PoolTier,
    quote_max_insurance: u128
) -> PoolClient<'a> {
    let pool = PoolClient::new(e, &e.register(crate::Pool {}, ()));
    let params = InitializeAllParams {
        base: InitializeParams {
            admin: admin.clone(),
            privileged_addrs: PrivilegedAddresses {
                emergency_admin: admin.clone(),
                rewards_admin: admin.clone(),
                operations_admin: admin.clone(),
                pause_admin: admin.clone(),
                emergency_pause_admins: Vec::from_array(e, [admin.clone()]),
            },
            router: router.clone(),
            assets: assets.clone(),
            tokens: tokens.clone(),
            lp_token_info: TokenInitInfo {
                token_wasm_hash: token_wasm_hash.clone(),
                name: lp_token_info.0.clone(),
                symbol: lp_token_info.1.clone(),
            },
            fee_fraction,
            tier: tier.clone(),
            quote_max_insurance,
        },
        reward_config: RewardConfig {
            reward_token: reward_token.clone(),
        },
        plane: plane.clone(),
    };
    pool.initialize_all(&params);
    pool
}

// #[test]
// fn test() {
//     let config = TestConfig {
//         users_count: 2,
//         mint_to_user: 1000,
//         rewards_count: 1_000_000_0000000,
//         liq_pool_fee: 30,
//         reward_tps: 10_5000000_u128,
//         reward_token_in_pool: false,
//     };
//     let _setup = Setup::new_with_config(&config);
// }
