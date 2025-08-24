#![cfg(test)]
extern crate std;
use crate::PoolSwapFeeCollectorClient;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::token::{
    StellarAssetClient as SorobanTokenAdminClient, TokenClient as SorobanTokenClient,
};
use soroban_sdk::{Address, Env};
use std::vec;

pub(crate) fn create_token_contract<'a>(e: &Env, admin: &Address) -> SorobanTokenClient<'a> {
    SorobanTokenClient::new(
        e,
        &e.register_stellar_asset_contract_v2(admin.clone())
            .address(),
    )
}

pub(crate) fn get_token_admin_client<'a>(
    e: &Env,
    address: &Address,
) -> SorobanTokenAdminClient<'a> {
    SorobanTokenAdminClient::new(e, address)
}

pub(crate) struct TestConfig {
    pub(crate) users_count: u32,
}

impl Default for TestConfig {
    fn default() -> Self {
        TestConfig { users_count: 3 }
    }
}

pub(crate) struct Setup<'a> {
    pub(crate) env: Env,

    // addresses
    pub(crate) admin: Address,
    pub(crate) emergency_admin: Address,
    pub(crate) users: vec::Vec<Address>,
    pub(crate) fee_destination: Address,

    // contracts
    pub(crate) fee_collector: PoolSwapFeeCollectorClient<'a>,

    // tokens
    pub(crate) token_a: SorobanTokenClient<'a>,
    // pub(crate) token_a_admin_client: SorobanTokenAdminClient<'a>,
    pub(crate) token_b: SorobanTokenClient<'a>,
    pub(crate) token_b_admin_client: SorobanTokenAdminClient<'a>,
}

impl Default for Setup<'_> {
    // Create setup from default config
    fn default() -> Self {
        let default_config = TestConfig::default();
        Self::setup(&default_config)
    }
}

impl Setup<'_> {
    pub(crate) fn setup(config: &TestConfig) -> Self {
        let e: Env = Env::default();
        e.mock_all_auths();
        e.cost_estimate().budget().reset_unlimited();

        // addresses
        let users = Self::generate_random_users(&e, config.users_count);
        let admin = users[0].clone();
        let emergency_admin = Address::generate(&e);
        let fee_destination = Address::generate(&e);

        // contracts

        let router = Address::generate(&e);
        let insurance_fund = Address::generate(&e);

        // tokens
        let token_a = create_token_contract(&e, &admin);
        let token_b = create_token_contract(&e, &admin);

        let token_a_admin_client = get_token_admin_client(&e, &token_a.address.clone());
        let token_b_admin_client = get_token_admin_client(&e, &token_b.address.clone());

        let fee_collector = create_pool_swap_fee_contract(&e);
        fee_collector.init_admin(&admin, &emergency_admin);
        fee_collector.set_router(&admin, &router);
        fee_collector.set_insurance_fund(&admin, &insurance_fund);
        fee_collector.set_fee_destination(&admin, &fee_destination);

        Self {
            env: e,

            // addresses
            users,
            admin,
            emergency_admin,
            fee_destination,

            // contracts
            fee_collector,

            // tokens
            token_a,
            // token_a_admin_client,
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

mod pool_swap_fee {
    soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/pool_swap_fee.wasm");
}

pub fn create_pool_swap_fee_contract<'a>(e: &Env) -> PoolSwapFeeCollectorClient<'a> {
    PoolSwapFeeCollectorClient::new(e, &e.register(crate::PoolSwapFeeCollector, ()))
}
