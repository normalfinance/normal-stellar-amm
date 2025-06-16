#![cfg(test)]
extern crate std;
use crate::storage_types::{ OracleGuardRails, PriceDivergenceGuardRails, ValidityGuardRails };
use crate::OracleRegistryClient;
use sep_40_oracle::testutils::{ Asset as MockAsset, MockPriceOracleClient, MockPriceOracleWASM };
use soroban_sdk::testutils::Address as _;

use soroban_sdk::{ Address, Env, Symbol, Vec };
use utils::constant::PERCENTAGE_PRECISION_U64;
use std::vec;

pub(crate) struct TestConfig {
    pub(crate) users_count: u32,
    pub(crate) oracle_guard_rails: OracleGuardRails,
    pub(crate) price_override_limit: u128,
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
                    slots_before_stale_for_pool: 10, // ~5 seconds
                    confidence_interval_max_size: 20_000, // 2% of price
                    too_volatile_ratio: 5, // 5x or 80% down
                },
            },
            price_override_limit: 100,
        }
    }
}

pub(crate) struct Setup<'a> {
    pub(crate) env: Env,

    // addresses
    pub(crate) admin: Address,
    pub(crate) emergency_admin: Address,
    pub(crate) users: vec::Vec<Address>,
    pub(crate) USDC: Address,

    // contracts
    pub(crate) registry: OracleRegistryClient<'a>,

    // oracles
    pub(crate) asset_id: Symbol,
    pub(crate) unregistered_asset_id: Symbol,
    pub(crate) oracle_client: MockPriceOracleClient<'a>,

    // state
    pub(crate) oracle_guardrails: OracleGuardRails,
    pub(crate) initial_oracle_price: u128,
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

        let users = Self::generate_random_users(&e, config.users_count);
        let admin = users[0].clone();
        let emergency_admin = Address::generate(&e);

        // tokens
        let asset_id = Symbol::new(&e, "BTC");
        let unregistered_asset_id = Symbol::new(&e, "ETH");
        let asset_addr = Address::generate(&e);
        let USDC = Address::generate(&e);

        let registry = create_oracle_registry_contract(&e);
        registry.initialize(&admin, &emergency_admin);
        registry.register_oracle(
            &admin,
            &asset_id,
            &e.register(MockPriceOracleWASM, ()),
            &asset_addr,
            &7,
            &0
        );

        // register oracle
        let oracle_client = MockPriceOracleClient::new(&e, &Address::generate(&e));
        let initial_oracle_price = 100_u128;
        oracle_client.set_data(
            &admin,
            &MockAsset::Stellar(USDC.clone()),
            &Vec::from_array(&e, [MockAsset::Other(asset_id.clone())]),
            &7,
            &(5 * 60 * 60)
        );
        oracle_client.set_price(
            &Vec::from_array(&e, [initial_oracle_price as i128]),
            &e.ledger().timestamp()
        );

        Self {
            env: e,
            admin,
            emergency_admin,
            users,
            USDC,
            registry,
            asset_id,
            unregistered_asset_id,
            oracle_client,
            oracle_guardrails: config.oracle_guard_rails,
            initial_oracle_price,
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

pub(crate) fn update_oracle_price(setup: &Setup, oracle: &Address, new_price: u128, now: &u64) {
    let client = MockPriceOracleClient::new(&setup.env, oracle);
    client.set_data(
        &setup.admin,
        &MockAsset::Stellar(setup.USDC.clone()),
        &Vec::from_array(&setup.env, [MockAsset::Other(setup.asset_id.clone())]),
        &7,
        &(5 * 60 * 60)
    );
    client.set_price(&Vec::from_array(&setup.env, [new_price as i128]), now);
}

pub mod oracle_registry {
    soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/oracle_registry.wasm");
}

pub fn create_oracle_registry_contract<'a>(e: &Env) -> OracleRegistryClient<'a> {
    OracleRegistryClient::new(e, &e.register(crate::OracleRegistry, ()))
}
