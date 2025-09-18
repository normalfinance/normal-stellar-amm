#![cfg(test)]
extern crate std;
use crate::InsuranceFundClient;
use soroban_sdk::testutils::{Address as _, Ledger};
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

    pub(crate) oracle_registry: Address,
    pub(crate) pool_router: Address,

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

    // Helper functions for insurance fund testing

    pub(crate) fn pay_premium(&self, amount: i128) {
        self.insurance_fund.pay_premium(&self.admin, &(amount as u128));
    }

    pub(crate) fn deposit_and_expect_success(&self, user: &Address, amount: i128) -> u128 {
        let initial_shares = self.get_user_shares(user);
        self.insurance_fund.deposit(user, &self.token_a.address, &(amount as u128));
        let final_shares = self.get_user_shares(user);
        final_shares - initial_shares
    }


    pub(crate) fn request_withdraw_and_expect_success(&self, user: &Address, shares: u128) {
        self.insurance_fund.request_withdraw(user, &self.token_a.address, &shares);
    }


    pub(crate) fn withdraw_and_expect_success(&self, user: &Address) -> u128 {
        let initial_balance = self.token_a.balance(user);
        self.insurance_fund.withdraw(user, &self.token_a.address);
        let final_balance = self.token_a.balance(user);
        (final_balance - initial_balance) as u128
    }

    pub(crate) fn assert_user_shares(&self, user: &Address, expected_shares: u128) {
        let actual_shares = self.get_user_shares(user);
        assert_eq!(actual_shares, expected_shares, "User shares mismatch for {:?}", user);
    }

    pub(crate) fn assert_vault_balance(&self, expected_balance: u128) {
        let actual_balance = self.get_vault_balance();
        assert_eq!(actual_balance, expected_balance, "Vault balance mismatch");
    }

    pub(crate) fn get_user_shares(&self, user: &Address) -> u128 {
        let stake = self.insurance_fund.get_stake(user, &self.token_a.address);
        stake.unchecked_shares()
    }

    pub(crate) fn get_vault_balance(&self) -> u128 {
        self.token_a.balance(&self.insurance_fund.address) as u128
    }

    pub(crate) fn get_reserve(&self) -> crate::reserve::InsuranceFundReserve {
        self.insurance_fund.get_reserve(&self.token_a.address)
    }

    pub(crate) fn setup_multiple_deposits(&self, amounts: &[i128]) -> std::vec::Vec<u128> {
        let mut shares_received = std::vec::Vec::new();
        for (i, &amount) in amounts.iter().enumerate() {
            let user = &self.users[i % self.users.len()];
            let shares = self.deposit_and_expect_success(user, amount);
            shares_received.push(shares);
        }
        shares_received
    }

    pub(crate) fn advance_time_past_unstaking(&self) {
        let unstaking_period = self.insurance_fund.get_unstaking_period();
        self.env.ledger().with_mut(|l| {
            l.timestamp = l.timestamp + unstaking_period + 1;
        });
    }

    pub(crate) fn advance_time(&self, seconds: u64) {
        self.env.ledger().with_mut(|l| {
            l.timestamp = l.timestamp + seconds;
        });
    }

    pub(crate) fn setup_withdraw_in_progress(&self, user: &Address, shares: u128) {
        self.request_withdraw_and_expect_success(user, shares);
    }

    pub(crate) fn get_optimal_insurance(&self) -> u128 {
        self.insurance_fund.get_optimal_insurance()
    }

    pub(crate) fn set_optimal_insurance(&self, amount: u128) {
        self.insurance_fund.set_optimal_insurance(&self.admin, &amount);
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
        let oracle_registry = Address::generate(&e);
        let pool_router = Address::generate(&e);
        insurance_fund.initialize(
            &admin,
            &emergency_admin,
            &oracle_registry,
            &pool_router,
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
            oracle_registry,
            pool_router,
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
