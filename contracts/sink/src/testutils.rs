#![cfg(test)]
extern crate std;
use crate::InsuranceFundClient;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::token::{
    StellarAssetClient as SorobanTokenAdminClient, TokenClient as SorobanTokenClient,
};
use soroban_sdk::{Address, Env};
use std::vec;
use utils::constant::{PRICE_PRECISION, PRICE_PRECISION_I128, THIRTEEN_DAY};

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
    pub(crate) mint_to_user: i128,
    pub(crate) unstaking_period: u64,
    pub(crate) optimal_utilization: u32,
    pub(crate) base_rate: i32,
    pub(crate) rate_slopes: (u32, u32),
}

impl Default for TestConfig {
    fn default() -> Self {
        TestConfig {
            users_count: 3,
            mint_to_user: 1_000 * PRICE_PRECISION_I128,
            unstaking_period: THIRTEEN_DAY,
            optimal_utilization: 80_00000_u32,         // 80%
            base_rate: 2_00000_i32,                    // 2%
            rate_slopes: (10_00000_u32, 60_00000_u32), // 10% and 60%
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
    pub(crate) insurance_fund: InsuranceFundClient<'a>,

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
        setup.mint_tokens_for_users(config.mint_to_user);
        setup
    }

    pub(crate) fn setup(config: &TestConfig) -> Self {
        let e: Env = Env::default();
        e.mock_all_auths();
        e.cost_estimate().budget().reset_unlimited();

        let users = Self::generate_random_users(&e, config.users_count);
        let admin = users[0].clone();
        let emergency_admin = Address::generate(&e);

        let token_a = create_token_contract(&e, &admin);
        let token_a_admin_client = get_token_admin_client(&e, &token_a.address.clone());

        let insurance_fund = create_insurance_fund_contract(&e);
        insurance_fund.initialize(
            &admin,
            &emergency_admin,
            &token_a.address,
            &config.unstaking_period,
            &config.optimal_utilization,
            &config.base_rate,
            &config.rate_slopes,
        );
        insurance_fund.set_optimal_insurance(&admin, &(1_000_000 * PRICE_PRECISION));

        Self {
            env: e,
            admin,
            emergency_admin,
            users,
            insurance_fund,
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

    pub(crate) fn mint_tokens_for_users(&self, amount: i128) {
        for user in self.users.iter() {
            self.token_a_admin_client.mint(user, &amount);
            assert_eq!(self.token_a.balance(user), amount.clone());
        }
    }
}

mod insurance_fund {
    soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/insurance_fund.wasm");
}

pub fn create_insurance_fund_contract<'a>(e: &Env) -> InsuranceFundClient<'a> {
    let insurance_fund = InsuranceFundClient::new(e, &e.register(crate::InsuranceFund {}, ()));
    insurance_fund
}
