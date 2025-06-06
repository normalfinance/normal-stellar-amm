#![cfg(test)]
extern crate std;
use crate::PoolClient;
use access_control::constants::ADMIN_ACTIONS_DELAY;
use sep_40_oracle::testutils::{ Asset as MockAsset, MockPriceOracleClient, MockPriceOracleWASM };
use sep_40_oracle::Asset;
use soroban_sdk::token::{
    StellarAssetClient as SorobanTokenAdminClient,
    TokenClient as SorobanTokenClient,
};
use soroban_sdk::String;
use soroban_sdk::{ testutils::Address as _, Address, BytesN, Env, Symbol, Vec };
use utils::constant::PERCENTAGE_PRECISION_U64;
use utils::oracle::{ OracleGuardRails, PriceDivergenceGuardRails, ValidityGuardRails };
use utils::storage::{
    InitializeAllParams,
    InitializeParams,
    OraclePair,
    PrivilegedAddresses,
    RewardConfig,
    TokenInitInfo,
};

use std::vec;
use token_share::token_contract::{ Client as ShareTokenClient, WASM };
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
            mint_to_user: 1000,
            rewards_count: 1_000_000_0000000,
            liq_pool_fee: 30,
            reward_tps: 10_5000000_u128,
            reward_token_in_pool: false,
        }
    }
}

pub(crate) struct Setup<'a> {
    pub(crate) env: Env,
    pub(crate) router: Address,
    pub(crate) oracles: OraclePair,
    pub(crate) asset: Address,
    pub(crate) base_oracle_price: i128,
    pub(crate) base_oracle_client: MockPriceOracleClient<'a>,
    pub(crate) quote_oracle_price: i128,
    pub(crate) quote_oracle_client: MockPriceOracleClient<'a>,
    pub(crate) oracle_guard_rails: OracleGuardRails,
    pub(crate) users: vec::Vec<Address>,
    pub(crate) token1: SorobanTokenClient<'a>,
    pub(crate) token1_admin_client: SorobanTokenAdminClient<'a>,
    pub(crate) token2: SorobanTokenClient<'a>,
    pub(crate) token2_admin_client: SorobanTokenAdminClient<'a>,
    pub(crate) token_reward: SorobanTokenClient<'a>,
    pub(crate) token_reward_admin_client: SorobanTokenAdminClient<'a>,
    pub(crate) token_share: ShareTokenClient<'a>,
    pub(crate) liq_pool: PoolClient<'a>,

    pub(crate) admin: Address,
    pub(crate) emergency_admin: Address,
    pub(crate) rewards_admin: Address,
    pub(crate) operations_admin: Address,
    pub(crate) pause_admin: Address,
    pub(crate) emergency_pause_admin: Address,
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

        let router = Address::generate(&e);
        // let xlm = Address::generate(&e);
        let usdc = Address::generate(&e);

        let asset = Asset::Other(Symbol::new(&e, "SOL"));
        let asset_mock = MockAsset::Other(Symbol::new(&e, "SOL"));
        // let quote_asset = Asset::Other(Symbol::new(&e, "XLM"));
        let quote_asset_mock = MockAsset::Other(Symbol::new(&e, "XLM"));

        let base_oracle_price = 2_0000000; // $2.00
        let quote_oracle_price = 0_5000000; // $0.50

        let oracles = OraclePair {
            base_oracle: e.register(MockPriceOracleWASM, ()),
            quote_oracle: e.register(MockPriceOracleWASM, ()),
        };

        let base_oracle_client = MockPriceOracleClient::new(&e, &oracles.base_oracle.address);
        let quote_oracle_client = MockPriceOracleClient::new(&e, &oracles.quote_oracle.address);

        // Setup base oracle
        base_oracle_client.set_data(
            &admin,
            &MockAsset::Stellar(usdc.clone()),
            &Vec::from_array(&e, [asset_mock.clone()]),
            &7,
            &(5 * 60 * 60)
        );
        base_oracle_client.set_price(
            &Vec::from_array(&e, [base_oracle_price]),
            &e.ledger().timestamp()
        );

        // Setup quote oracle
        quote_oracle_client.set_data(
            &admin,
            &MockAsset::Stellar(usdc),
            &Vec::from_array(&e, [quote_asset_mock.clone()]),
            &7,
            &(5 * 60 * 60)
        );
        quote_oracle_client.set_price(
            &Vec::from_array(&e, [quote_oracle_price]),
            &e.ledger().timestamp()
        );

        let oracle_guard_rails = OracleGuardRails {
            price_divergence: PriceDivergenceGuardRails {
                oracle_twap_percent_divergence: PERCENTAGE_PRECISION_U64 / 2,
            },
            validity: ValidityGuardRails {
                slots_before_stale_for_pool: 10, // ~5 seconds
                confidence_interval_max_size: 20_000, // 2% of price
                too_volatile_ratio: 5, // 5x or 80% down
            },
        };

        let liq_pool = create_pool_contract(
            &e,
            &admin,
            &router,
            &oracles,
            &asset,
            &install_token_wasm(&e),
            &String::from_str(&e, "nSOL / XLM Pool Token"),
            &String::from_str(&e, "nSOL-LP"),
            &Vec::from_array(&e, [token1.address.clone(), token2.address.clone()]),
            &reward_token.address,
            config.liq_pool_fee
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

        let token_share = ShareTokenClient::new(&e, &liq_pool.share_id());

        // Set token1 admin to liquidity pool so it can mint/burn token1 on user calls
        token1_admin_client.set_admin(&liq_pool.address);

        Self {
            env: e,
            router,
            oracles,
            asset,
            base_oracle_price,
            base_oracle_client,
            quote_oracle_price,
            quote_oracle_client,
            oracle_guard_rails,
            users,
            token1,
            token1_admin_client,
            token2,
            token2_admin_client,
            token_reward: reward_token,
            token_reward_admin_client,
            token_share,
            liq_pool: liq_pool,
            admin,
            emergency_admin,
            rewards_admin,
            operations_admin,
            pause_admin,
            emergency_pause_admin,
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
                &reward_tps
            );
        }
    }
}

pub(crate) fn create_token_contract<'a>(e: &Env, admin: &Address) -> SorobanTokenClient<'a> {
    SorobanTokenClient::new(e, &e.register_stellar_asset_contract_v2(admin.clone()).address())
}

pub(crate) fn get_token_admin_client<'a>(
    e: &Env,
    address: &Address
) -> SorobanTokenAdminClient<'a> {
    SorobanTokenAdminClient::new(e, address)
}

pub fn create_pool_contract<'a>(
    e: &Env,
    admin: &Address,
    router: &Address,
    oracles: &OraclePair,
    asset: &Asset,
    token_wasm_hash: &BytesN<32>,
    lp_token_name: &String,
    lp_token_symbol: &String,
    tokens: &Vec<Address>,
    reward_token: &Address,
    fee_fraction: u32
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
            oracles: oracles.clone(),
            asset: asset.clone(),
            tokens: tokens.clone(),
            lp_token_info: TokenInitInfo {
                token_wasm_hash: token_wasm_hash.clone(),
                name: lp_token_name.clone(),
                symbol: lp_token_symbol.clone(),
            },
            fee_fraction,
        },
        reward_config: RewardConfig {
            reward_token: reward_token.clone(),
        },
    };
    pool.initialize_all(&params);
    pool
}

pub fn install_token_wasm(e: &Env) -> BytesN<32> {
    e.deployer().upload_contract_wasm(WASM)
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
