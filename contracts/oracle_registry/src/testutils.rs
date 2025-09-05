#![cfg(test)]
extern crate std;

use crate::OracleRegistryClient;
use sep_40_oracle::testutils::{Asset as MockAsset, MockPriceOracleClient, MockPriceOracleWASM};
use soroban_sdk::testutils::{Address as _, Ledger, LedgerInfo};
use utils::state::oracle_registry::{
    OracleGuardRails, PriceDivergenceGuardRails, ValidityGuardRails,
};

use soroban_sdk::{Address, Env, Symbol, Vec};
use std::vec;
use utils::test_utils::jump;

pub(crate) struct TestConfig {
    pub(crate) users_count: u32,
    pub(crate) oracle_guard_rails: OracleGuardRails,
}

impl Default for TestConfig {
    fn default() -> Self {
        TestConfig {
            users_count: 3,
            oracle_guard_rails: OracleGuardRails {
                price_divergence: PriceDivergenceGuardRails {
                    oracle_twap_percent_divergence: 1200000000, // allow ±20%
                },
                validity: ValidityGuardRails {
                    seconds_before_stale_for_pool: 300, // 5mins
                    too_volatile_ratio: 1200000000,     // allow ±20%
                },
            },
        }
    }
}

pub(crate) struct Setup<'a> {
    pub(crate) env: Env,

    // addresses
    pub(crate) admin: Address,
    pub(crate) emergency_admin: Address,
    pub(crate) users: vec::Vec<Address>,

    // contracts
    pub(crate) registry: OracleRegistryClient<'a>,

    // oracles
    pub(crate) btc_symbol: Symbol,
    pub(crate) eth_symbol: Symbol,
    pub(crate) btc_asset: MockAsset,
    pub(crate) eth_asset: MockAsset,
    pub(crate) oracle_client: MockPriceOracleClient<'a>,

    // state
    pub(crate) oracle_guard_rails: OracleGuardRails,

    // oracle
    pub(crate) oracle: Address,
    pub(crate) initial_btc_price: u128,
    pub(crate) initial_eth_price: u128,
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

        // Set the current time to avoid oracle timing issues
        let start_time = 1751513914;
        jump(&e, start_time);

        // Addresses
        let users = Self::generate_random_users(&e, config.users_count);
        let admin = users[0].clone();
        let emergency_admin = Address::generate(&e);

        // Assets
        let btc_symbol = Symbol::new(&e, "BTC");
        let eth_symbol = Symbol::new(&e, "ETH");
        let usd_symbol = Symbol::new(&e, "USD");

        let btc_asset = MockAsset::Other(btc_symbol.clone());
        let eth_asset = MockAsset::Other(eth_symbol.clone());
        let usd_asset = MockAsset::Other(usd_symbol);

        let (oracle_address, oracle_client) = setup_price_feed_oracle(
            &e,
            &admin,
            &usd_asset,
            &Vec::from_array(&e, [btc_asset.clone(), eth_asset.clone()]),
            14,
            300,
        );

        // Add prices to the mocked oracle (using 14 decimal precision according to Reflector oracle)
        let initial_btc_price = 109237_22794294087742_i128;
        let initial_eth_price = 3237_22794294087742_i128;
        let prices: Vec<i128> = Vec::from_array(&e, [initial_btc_price, initial_eth_price]);
        oracle_client.set_price(&prices, &start_time);

        // verify price data can be fetched
        let result_1 = oracle_client.lastprice(&btc_asset).unwrap();
        assert_eq!(result_1.price, prices.get_unchecked(0));

        let result_2 = oracle_client.lastprice(&eth_asset).unwrap();
        assert_eq!(result_2.price, prices.get_unchecked(1));

        // Setup the Oracle Registry
        let registry = create_oracle_registry_contract(&e);
        registry.initialize(&admin, &emergency_admin);
        registry.set_oracle_guard_rails(&admin, &config.oracle_guard_rails);

        // Register the BTC oracle with the Registry
        registry.register_oracle(&admin, &btc_symbol, &oracle_address, &14, &1);

        Self {
            env: e,
            // Addresses
            admin,
            emergency_admin,
            users,

            // Contracts
            registry,

            // Assets
            btc_symbol,
            eth_symbol,
            btc_asset,
            eth_asset,

            // Oracle
            oracle: oracle_address,
            oracle_client,
            oracle_guard_rails: config.oracle_guard_rails,
            initial_btc_price: initial_btc_price as u128,
            initial_eth_price: initial_eth_price as u128,
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

// (https://github.com/script3/sep-40-oracle/blob/d2d9a19079d95f79c16c3ff506416346d75b537f/mock-sep-40/src/test.rs)
fn setup_price_feed_oracle<'a>(
    env: &Env,
    admin: &Address,
    base: &MockAsset,
    assets: &Vec<MockAsset>,
    decimals: u32,
    resolution: u32,
) -> (Address, MockPriceOracleClient<'a>) {
    let oracle_address = env.register(MockPriceOracleWASM, ());
    let oracle_client = MockPriceOracleClient::new(env, &oracle_address);
    oracle_client.set_data(admin, base, assets, &decimals, &resolution);
    (oracle_address, oracle_client)
}

pub mod oracle_registry {
    soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/oracle_registry.wasm");
}

pub fn create_oracle_registry_contract<'a>(e: &Env) -> OracleRegistryClient<'a> {
    OracleRegistryClient::new(e, &e.register(crate::OracleRegistry, ()))
}
