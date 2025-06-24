#![cfg(test)]
extern crate std;
use crate::storage_types::{OracleGuardRails, PriceDivergenceGuardRails, ValidityGuardRails};
use crate::OracleRegistryClient;
use sep_40_oracle::testutils::{Asset as MockAsset, MockPriceOracleClient, MockPriceOracleWASM};
use soroban_sdk::testutils::Address as _;

use soroban_sdk::{Address, Env, Symbol, Vec};
use std::vec;
use utils::constant::PERCENTAGE_PRECISION_U64;
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
                    oracle_twap_percent_divergence: PERCENTAGE_PRECISION_U64 / 2,
                },
                validity: ValidityGuardRails {
                    seconds_before_stale_for_pool: 5,
                    too_volatile_ratio: 120, // allow ±20%
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
    pub(crate) oracle: Address,
    pub(crate) eth_addr: Address,

    // contracts
    pub(crate) registry: OracleRegistryClient<'a>,

    // oracles
    pub(crate) btc_asset_id: Symbol,
    pub(crate) eth_asset_id: Symbol,
    pub(crate) oracle_client: MockPriceOracleClient<'a>,

    // state
    pub(crate) oracle_guard_rails: OracleGuardRails,
    pub(crate) init_btc_price: i128,
    pub(crate) init_eth_price: i128,
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

        // addresses
        let users = Self::generate_random_users(&e, config.users_count);
        let admin = users[0].clone();
        let emergency_admin = Address::generate(&e);

        // assets
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
            300,
        );

        // prices
        let start_time = e.ledger().timestamp();
        let init_btc_price = 50000_0000000_i128; // $50,000
        let init_eth_price = 3000_0000000_i128; // $3,000
        let prices: Vec<i128> = Vec::from_array(&e, [init_btc_price, init_eth_price]);
        oracle_client.set_price(&prices, &start_time);

        // verify price data can be fetched
        let result_1 = oracle_client.lastprice(&btc_asset).unwrap();
        assert_eq!(result_1.price, prices.get_unchecked(0));
        assert_eq!(result_1.timestamp, start_time);

        let result_2 = oracle_client.lastprice(&eth_asset).unwrap();
        assert_eq!(result_2.price, prices.get_unchecked(1));
        assert_eq!(result_2.timestamp, start_time);

        let last_timestamp = oracle_client.last_timestamp();
        assert_eq!(last_timestamp, start_time);

        // pass time
        jump(&e, 325);
        // e.ledger().set(LedgerInfo {
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
        let result_1 = oracle_client.lastprice(&btc_asset).unwrap();
        assert_eq!(result_1.price, prices.get_unchecked(0));
        assert_eq!(result_1.timestamp, start_time);

        let result_2 = oracle_client.lastprice(&eth_asset).unwrap();
        assert_eq!(result_2.price, prices.get_unchecked(1));
        assert_eq!(result_2.timestamp, start_time);

        let last_timestamp = oracle_client.last_timestamp();
        assert_eq!(last_timestamp, start_time);

        // set another round of prices
        // let prices_2: Vec<i128> = vec![&e, 95_214_7654321, 1_1040921];
        let prices_2: Vec<i128> = Vec::from_array(&e, [50100_0000000_i128, 3050_0000000_i128]);
        //
        oracle_client.set_price(&prices_2, &(start_time + 300));

        // verify most recent prices are fetched
        let result_1 = oracle_client.lastprice(&btc_asset).unwrap();
        assert_eq!(result_1.price, prices_2.get_unchecked(0));
        assert_eq!(result_1.timestamp, start_time + 300);

        let result_2 = oracle_client.lastprice(&eth_asset).unwrap();
        assert_eq!(result_2.price, prices_2.get_unchecked(1));
        assert_eq!(result_2.timestamp, start_time + 300);

        let last_timestamp = oracle_client.last_timestamp();
        assert_eq!(last_timestamp, start_time + 300);

        // verify old prices can be fetched
        let result_1 = oracle_client.price(&btc_asset, &start_time).unwrap();
        assert_eq!(result_1.price, prices.get_unchecked(0));
        assert_eq!(result_1.timestamp, start_time);

        let result_2 = oracle_client.price(&eth_asset, &start_time).unwrap();
        assert_eq!(result_2.price, prices.get_unchecked(1));
        assert_eq!(result_2.timestamp, start_time);

        // verify timestamp is normalized to the most recent price
        // older than the requested timestamp
        let result_1 = oracle_client
            .price(&btc_asset, &(100 + start_time))
            .unwrap();
        assert_eq!(result_1.price, prices.get_unchecked(0));
        assert_eq!(result_1.timestamp, start_time);

        let result_2 = oracle_client
            .price(&eth_asset, &(250 + start_time))
            .unwrap();
        assert_eq!(result_2.price, prices.get_unchecked(1));
        assert_eq!(result_2.timestamp, start_time);

        // verify get prices can fetch both
        let result_1_vec = oracle_client.prices(&btc_asset, &2).unwrap();
        assert_eq!(result_1_vec.len(), 2);
        let result_1_0 = result_1_vec.get_unchecked(0);
        assert_eq!(result_1_0.price, prices_2.get_unchecked(0));
        assert_eq!(result_1_0.timestamp, start_time + 300);
        let result_1_1 = result_1_vec.get_unchecked(1);
        assert_eq!(result_1_1.price, prices.get_unchecked(0));
        assert_eq!(result_1_1.timestamp, start_time);

        let result_2_vec = oracle_client.prices(&eth_asset, &2).unwrap();
        assert_eq!(result_2_vec.len(), 2);
        let result_2_0 = result_2_vec.get_unchecked(0);
        assert_eq!(result_2_0.price, prices_2.get_unchecked(1));
        assert_eq!(result_2_0.timestamp, start_time + 300);
        let result_2_1 = result_2_vec.get_unchecked(1);
        assert_eq!(result_2_1.price, prices.get_unchecked(1));
        assert_eq!(result_2_1.timestamp, start_time);

        // verify un-normalized timestamps get set to the most recent normalized timestamp
        // let prices_3: Vec<i128> = vec![&e, 96_214_7654321, 1_0940921];
        let prices_3: Vec<i128> = Vec::from_array(&e, [49850_0000000_i128, 3037_0000000_i128]);
        oracle_client.set_price(&prices_3, &(start_time + 600 + 100));

        let result_1 = oracle_client.lastprice(&btc_asset).unwrap();
        assert_eq!(result_1.price, prices_3.get_unchecked(0));
        assert_eq!(result_1.timestamp, start_time + 600);

        let result_2 = oracle_client.lastprice(&eth_asset).unwrap();
        assert_eq!(result_2.price, prices_3.get_unchecked(1));
        assert_eq!(result_2.timestamp, start_time + 600);

        let last_timestamp = oracle_client.last_timestamp();
        assert_eq!(last_timestamp, start_time + 600);

        // Register the oracle with the Registry
        let registry = create_oracle_registry_contract(&e);
        registry.initialize(&admin, &emergency_admin);
        registry.set_oracle_guard_rails(&admin, &config.oracle_guard_rails);

        registry.register_oracle(&admin, &btc_asset_id, &oracle_id, &btc_addr, &7, &0);

        Self {
            env: e,
            admin,
            emergency_admin,
            users,
            oracle: oracle_id,
            eth_addr,
            registry,
            btc_asset_id,
            eth_asset_id,
            oracle_client,
            oracle_guard_rails: config.oracle_guard_rails,
            init_btc_price,
            init_eth_price,
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
    let oracle_id = env.register(MockPriceOracleWASM, ());
    let oracle_client = MockPriceOracleClient::new(env, &oracle_id);
    oracle_client.set_data(admin, base, assets, &decimals, &resolution);
    (oracle_id, oracle_client)
}

pub mod oracle_registry {
    soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/oracle_registry.wasm");
}

pub fn create_oracle_registry_contract<'a>(e: &Env) -> OracleRegistryClient<'a> {
    OracleRegistryClient::new(e, &e.register(crate::OracleRegistry, ()))
}
