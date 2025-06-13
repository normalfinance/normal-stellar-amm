#![cfg(test)]
extern crate std;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::token::{
    StellarAssetClient as SorobanTokenAdminClient,
    TokenClient as SorobanTokenClient,
};
use soroban_sdk::{ Address, Env, Vec };
use utils::test_utils::{
    buffer,
    create_token_contract,
    fee_collector,
    get_token_admin_client,
    oracle_registry,
    pool_router,
    setup_buffer,
};
use std::vec;

pub(crate) struct TestConfig {
    pub(crate) users_count: u32,
    pub(crate) min_time_between_payouts: u64,
}

impl Default for TestConfig {
    fn default() -> Self {
        TestConfig {
            users_count: 3,
            min_time_between_payouts: 30, // 30 seconds
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
    pub(crate) buffer: buffer::Client<'a>,

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

        let users = Self::generate_random_users(&e, config.users_count);
        let admin = users[0].clone();
        let emergency_admin = Address::generate(&e);
        let asset = Address::generate(&e);
        let fee_destination = Address::generate(&e);

        let token_a = create_token_contract(&e, &admin);
        let token_b = create_token_contract(&e, &admin);

        let token_a_admin_client = get_token_admin_client(&e, &token_a.address.clone());
        let token_b_admin_client = get_token_admin_client(&e, &token_b.address.clone());

        let router = Address::generate(&e);
        let fee_collector = Address::generate(&e);

        let buffer = setup_buffer(&e, &admin, &emergency_admin, &router);
        buffer.set_fee_collector(&admin, &fee_collector);

        Self {
            env: e,
            admin,
            emergency_admin,
            buffer,
            users,
            token_a,
            token_a_admin_client,
            token_b,
            token_b_admin_client,
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
