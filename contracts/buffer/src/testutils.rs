#![cfg(test)]
extern crate std;
use crate::BufferClient;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::token::{
    StellarAssetClient as SorobanTokenAdminClient,
    TokenClient as SorobanTokenClient,
};
use soroban_sdk::{ Address, Env };
use utils::constant::ONE_HOUR;
use utils::test_utils::{ create_token_contract, get_token_admin_client };
use std::vec;

pub(crate) struct TestConfig {
    pub(crate) users_count: u32,
    pub(crate) min_time_between_payouts: u64,
    pub(crate) min_reserve_ratio: u32,
}

impl Default for TestConfig {
    fn default() -> Self {
        TestConfig {
            users_count: 3,
            min_time_between_payouts: ONE_HOUR,
            min_reserve_ratio: 1000, // 10%
        }
    }
}

pub(crate) struct Setup<'a> {
    pub(crate) env: Env,

    // addresses
    pub(crate) admin: Address,
    pub(crate) emergency_admin: Address,
    pub(crate) users: vec::Vec<Address>,
    pub(crate) pool_address: Address,

    // contracts
    pub(crate) buffer: BufferClient<'a>,

    // tokens
    pub(crate) token_a: SorobanTokenClient<'a>,
    pub(crate) token_a_admin_client: SorobanTokenAdminClient<'a>,
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
        let pool_address = Address::generate(&e);

        let token_a = create_token_contract(&e, &admin);
        let token_a_admin_client = get_token_admin_client(&e, &token_a.address.clone());

        let buffer = create_buffer_contract(&e);
        buffer.initialize(
            &admin,
            &emergency_admin,
            &config.min_time_between_payouts,
            &config.min_reserve_ratio
        );

        Self {
            env: e,
            admin,
            emergency_admin,
            pool_address,
            buffer,
            users,
            token_a,
            token_a_admin_client,
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

mod buffer {
    soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/buffer.wasm");
}

pub fn create_buffer_contract<'a>(e: &Env) -> BufferClient<'a> {
    let buffer = BufferClient::new(e, &e.register(crate::Buffer {}, ()));
    buffer
}
