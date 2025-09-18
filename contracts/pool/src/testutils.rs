#![cfg(test)]
extern crate std;
use crate::plane::{pool_plane, PoolPlaneClient};

use crate::testutils::oracle_registry::{
    OracleGuardRails, PriceDivergenceGuardRails, ValidityGuardRails,
};
use crate::PoolClient;
use access_control::constants::ADMIN_ACTIONS_DELAY;
use sep_40_oracle::testutils::{Asset as MockAsset, MockPriceOracleClient, MockPriceOracleWASM};
use soroban_sdk::testutils::StellarAssetContract;
use soroban_sdk::token::{
    self, StellarAssetClient as SorobanTokenAdminClient, TokenClient as SorobanTokenClient,
};
use soroban_sdk::String;
use soroban_sdk::{testutils::Address as _, Address, BytesN, Env, Symbol, Vec};
use utils::constant::{PRICE_PRECISION, THIRTEEN_DAY};
use utils::state::pool::PoolConfig;
use utils::state::{
    access::PrivilegedAddresses,
    pool::{InitializeAllParams, PoolTier, RewardConfig},
    token::TokenInitInfo,
};

use std::vec;
use token_share::token_contract::{Client as ShareTokenClient, WASM};
use utils::test_utils::jump;

pub(crate) struct TestConfig {
    pub(crate) users_count: u32,
    pub(crate) mint_to_user: i128,
    pub(crate) rewards_count: i128,
    pub(crate) liq_pool_fee: u32,
    pub(crate) reward_tps: u128,
    pub(crate) reward_token_in_pool: bool,
}

impl Default for TestConfig {
    fn default() -> Self {
        TestConfig {
            users_count: 2,
            mint_to_user: 10000_0000000,
            rewards_count: 1_000_000_0000000,
            liq_pool_fee: 30,
            reward_tps: 10_5000000_u128,
            reward_token_in_pool: false,
        }
    }
}

pub(crate) struct Setup<'a> {
    pub(crate) env: Env,

    // Addresses
    pub(crate) admin: Address,
    pub(crate) users: vec::Vec<Address>,
    pub(crate) emergency_admin: Address,
    pub(crate) rewards_admin: Address,
    pub(crate) operations_admin: Address,
    pub(crate) pause_admin: Address,
    pub(crate) emergency_pause_admin: Address,

    // Contracts
    pub(crate) liq_pool: PoolClient<'a>,
    pub(crate) plane: pool_plane::Client<'a>,
    pub(crate) registry: oracle_registry::Client<'a>,
    pub(crate) liquidity_calculator: liquidity_calculator::Client<'a>,
    pub(crate) router: Address,
    pub(crate) insurance_fund: insurance_fund::Client<'a>,
    pub(crate) sac_address: Address,

    // Oracle
    pub(crate) oracle_address: Address,
    // pub(crate) oracle_client: MockPriceOracleClient<'a>,
    pub(crate) sol_symbol: Symbol,
    pub(crate) xlm_symbol: Symbol,
    pub(crate) sol_asset: MockAsset,
    pub(crate) xlm_asset: MockAsset,
    // pub(crate) init_btc_price: i128,
    // pub(crate) init_eth_price: i128,
    // pub(crate) init_xlm_price: i128,

    // Tokens
    pub(crate) token1: SorobanTokenClient<'a>,
    pub(crate) token2: SorobanTokenClient<'a>,
    pub(crate) token2_admin_client: SorobanTokenAdminClient<'a>,
    pub(crate) token_reward: SorobanTokenClient<'a>,
    pub(crate) token_reward_admin_client: SorobanTokenAdminClient<'a>,
    pub(crate) token_share: ShareTokenClient<'a>,
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

        // let start_time = e.ledger().timestamp();
        let start_time = 1758191744;
        jump(&e, start_time);

        let users = Self::generate_random_users(&e, config.users_count);
        let admin = users[0].clone();
        let emergency_admin = Address::generate(&e);
        let rewards_admin = Address::generate(&e);
        let operations_admin = Address::generate(&e);
        let pause_admin = Address::generate(&e);
        let emergency_pause_admin = Address::generate(&e);

        // Deploy Stellar Asset Contract (sac)
        let asset_sac = e.register_stellar_asset_contract_v2(admin.clone());
        asset_sac
            .issuer()
            .set_flag(soroban_sdk::xdr::AccountFlags::ClawbackEnabledFlag);
        let asset_address = asset_sac.address();
        let token1 = token::Client::new(&e, &asset_address);

        let token2 = create_token_contract(&e, &admin);
        let reward_token = if config.reward_token_in_pool {
            SorobanTokenClient::new(&e, &asset_address.clone())
        } else {
            create_token_contract(&e, &admin)
        };

        let plane = create_plane_contract(&e);

        let token2_admin_client = get_token_admin_client(&e, &token2.address.clone());
        let token_reward_admin_client = get_token_admin_client(&e, &reward_token.address.clone());

        let router = Address::generate(&e);
        let insurance_fund = Address::generate(&e);

        // Liquidity Calculator
        let liquidity_calculator = create_liquidity_calculator_contract(&e);
        liquidity_calculator.init_admin(&admin);
        liquidity_calculator.set_pools_plane(&admin, &plane.address);

        // Oracle Registry
        let sol_symbol = Symbol::new(&e, "SOL");
        let xlm_symbol = Symbol::new(&e, "XLM");
        let usd_symbol = Symbol::new(&e, "USD");

        let sol_asset = MockAsset::Other(sol_symbol.clone());
        let xlm_asset = MockAsset::Other(xlm_symbol.clone());
        let usd_asset = MockAsset::Other(usd_symbol);

        let (oracle_address, oracle_client) = setup_price_feed_oracle(
            &e,
            &admin,
            &usd_asset,
            &Vec::from_array(&e, [sol_asset.clone(), xlm_asset.clone()]),
            14,
            300,
        );

        let prices: Vec<i128> = Vec::from_array(&e, [200_00000000000000, 0_50000000000000]);
        oracle_client.set_price(&prices, &start_time);

        let registry = create_oracle_registry_contract(&e);
        registry.initialize(&admin, &emergency_admin);
        registry.set_oracle_guard_rails(
            &admin,
            &(OracleGuardRails {
                price_divergence: PriceDivergenceGuardRails {
                    oracle_twap_percent_divergence: 1000000,
                },
                validity: ValidityGuardRails {
                    seconds_before_stale_for_pool: 5000,
                    too_volatile_ratio: 2000000, // allow ±20%
                },
            }),
        );

        // Register oracles
        registry.register_oracle(&admin, &sol_symbol, &oracle_address, &14, &1);
        registry.register_oracle(&admin, &xlm_symbol, &oracle_address, &14, &1);

        // Insurance Fund
        let insurance_fund = create_insurance_fund_contract(&e);
        insurance_fund.initialize(
            &admin,
            &emergency_admin,
            &registry.address,
            &router,
            &token2.address,
            &THIRTEEN_DAY,
            &80_00000_u32,
            &2_00000_i32,
            &(10_00000_u32, 60_00000_u32),
        );
        insurance_fund.set_optimal_insurance(&admin, &(1_000_000 * PRICE_PRECISION));

        // Pool
        let liq_pool = create_pool_contract(
            &e,
            &admin,
            &plane.address,
            &router,
            &registry.address,
            &insurance_fund.address,
            &(sol_symbol.clone(), xlm_symbol.clone()),
            &asset_address,
            &(
                install_token_wasm(&e),
                String::from_str(&e, "Pool Share Token"),
                String::from_str(&e, "Pool Share Token"),
            ),
            &token2.address.clone(),
            &reward_token.address,
            (30_u32, 5000_u32),
            &PoolTier::A,
            1_000_000_0000000u128,
        );
        token::StellarAssetClient::new(&e, &asset_address).set_admin(&liq_pool.address);

        token_reward_admin_client.mint(&liq_pool.address, &config.rewards_count);

        liq_pool.set_privileged_addrs(
            &admin,
            &rewards_admin,
            &operations_admin,
            &pause_admin,
            &Vec::from_array(&e, [emergency_pause_admin.clone()]),
        );

        let emergency_admin = Address::generate(&e);
        // liq_pool.commit_transfer_ownership(
        //     &admin,
        //     &Symbol::new(&e, "EmergencyAdmin"),
        //     &emergency_admin,
        // );
        // jump(&e, ADMIN_ACTIONS_DELAY + 1); // delay is mandatory since emergency admin was set during initialization
        // liq_pool.apply_transfer_ownership(&admin, &Symbol::new(&e, "EmergencyAdmin"));

        // liq_pool.set_protocol_fee_fraction(&admin, &5000);

        let token_share = ShareTokenClient::new(&e, &liq_pool.share_id());

        insurance_fund.set_premium_payer_status(&admin, &liq_pool.address, &true);

        Self {
            env: e,

            // Addresses
            users,
            admin,
            emergency_admin,
            rewards_admin,
            operations_admin,
            pause_admin,
            emergency_pause_admin,

            // Contracts
            plane,
            liquidity_calculator,
            registry,
            liq_pool,
            router,
            insurance_fund,
            sac_address: asset_address,

            // Oracle
            oracle_address,
            sol_symbol,
            xlm_symbol,
            sol_asset,
            xlm_asset,

            // init_btc_price,
            // init_eth_price,
            // init_xlm_price,

            // Tokens
            token1,
            token2,
            token2_admin_client,
            token_reward: reward_token,
            token_reward_admin_client,
            token_share,
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
            self.token2_admin_client.mint(user, &amount);
            assert_eq!(self.token2.balance(user), amount.clone());
        }
    }

    pub(crate) fn set_rewards_config(&self, reward_tps: u128) {
        if reward_tps > 0 {
            self.liq_pool.set_rewards_config(
                &self.users[0],
                &self.env.ledger().timestamp().saturating_add(60),
                &reward_tps,
            );
        }
    }
}

// Contracts

pub mod liquidity_calculator {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/liquidity_calculator.wasm"
    );
}

pub mod oracle_registry {
    soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/oracle_registry.wasm");
}

pub mod insurance_fund {
    soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/insurance_fund.wasm");
}

pub(crate) mod test_token {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/soroban_token_contract.wasm"
    );
}

// pub mod pool_router {
//     soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/pool_router.wasm");
// }

// Create Contracts

pub fn create_token_contract<'a>(e: &Env, admin: &Address) -> SorobanTokenClient<'a> {
    SorobanTokenClient::new(
        e,
        &e.register_stellar_asset_contract_v2(admin.clone())
            .address(),
    )
}

pub fn create_plane_contract<'a>(e: &Env) -> pool_plane::Client<'a> {
    pool_plane::Client::new(e, &e.register(pool_plane::WASM, ()))
}

pub fn create_liquidity_calculator_contract<'a>(e: &Env) -> liquidity_calculator::Client<'a> {
    liquidity_calculator::Client::new(e, &e.register(liquidity_calculator::WASM, ()))
}

pub fn create_oracle_registry_contract<'a>(e: &Env) -> oracle_registry::Client<'a> {
    oracle_registry::Client::new(e, &e.register(oracle_registry::WASM, ()))
}

pub fn create_insurance_fund_contract<'a>(e: &Env) -> insurance_fund::Client<'a> {
    insurance_fund::Client::new(e, &e.register(insurance_fund::WASM, ()))
}

// pub fn create_pool_router_contract<'a>(e: &Env) -> pool_router::Client<'a> {
//     let router = pool_router::Client::new(e, &e.register(pool_router::WASM, ()));
//     router
// }

// Utils

pub(crate) fn get_token_admin_client<'a>(
    e: &Env,
    address: &Address,
) -> SorobanTokenAdminClient<'a> {
    SorobanTokenAdminClient::new(e, address)
}

// https://github.com/script3/sep-40-oracle/blob/d2d9a19079d95f79c16c3ff506416346d75b537f/mock-sep-40/src/test.rs
pub fn setup_price_feed_oracle<'a>(
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

pub fn create_pool_contract<'a>(
    e: &Env,
    admin: &Address,
    plane: &Address,
    router: &Address,
    oracle_registry: &Address,
    insurance_fund: &Address,
    assets: &(Symbol, Symbol),
    sac: &Address,
    share_token_info: &(BytesN<32>, String, String),
    token_b: &Address,
    reward_token: &Address,
    fees_config: (u32, u32),
    tier: &PoolTier,
    max_insurance: u128,
) -> PoolClient<'a> {
    let pool = PoolClient::new(e, &e.register(crate::Pool {}, ()));
    let params = InitializeAllParams {
        config: PoolConfig {
            admin: admin.clone(),
            privileged_addrs: PrivilegedAddresses {
                emergency_admin: admin.clone(),
                rewards_admin: admin.clone(),
                operations_admin: admin.clone(),
                pause_admin: admin.clone(),
                emergency_pause_admins: Vec::from_array(e, [admin.clone()]),
            },
            router: router.clone(),
            oracle_registry: oracle_registry.clone(),
            insurance_fund: insurance_fund.clone(),
            assets: assets.clone(),
            token_b: token_b.clone(),
            token_a_sac_address: sac.clone(),
            share_token_info: TokenInitInfo {
                token_wasm_hash: share_token_info.0.clone(),
                name: share_token_info.1.clone(),
                symbol: share_token_info.2.clone(),
            },
            fee_fraction: fees_config.0,
            protocol_fee_fraction: fees_config.1,
            status: utils::state::pool::PoolStatus::Initialized,
            tier: tier.clone(),
            max_insurance,
        },
        reward_config: RewardConfig {
            reward_token: reward_token.clone(),
        },
        plane: plane.clone(),
    };
    pool.initialize_all(&params);
    pool
}

pub fn install_token_wasm(e: &Env) -> BytesN<32> {
    e.deployer().upload_contract_wasm(WASM)
}
