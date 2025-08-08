#![cfg(test)]
extern crate std;
use crate::contracts;
// use crate::contracts::oracle_registry::PriceDivergenceGuardRails;
use crate::contracts::pool_router::PoolTier;
use crate::contracts::oracle_registry::{
    OracleGuardRails,
    PriceDivergenceGuardRails,
    ValidityGuardRails,
};
use sep_40_oracle::testutils::{ Asset as MockAsset, MockPriceOracleClient, MockPriceOracleWASM };
use soroban_sdk::testutils::{ Address as _, BytesN };
use soroban_sdk::token::{
    StellarAssetClient as SorobanTokenAdminClient,
    TokenClient as SorobanTokenClient,
};
use std::vec;
use soroban_sdk::{ Address, Env, String, Symbol, Vec };
use utils::constant::{ PERCENTAGE_PRECISION_U64, THIRTEEN_DAY };
use utils::test_utils::{ jump };

pub(crate) fn create_token_contract<'a>(e: &Env, admin: &Address) -> SorobanTokenClient<'a> {
    SorobanTokenClient::new(e, &e.register_stellar_asset_contract_v2(admin.clone()).address())
}

pub(crate) fn get_token_admin_client<'a>(
    e: &Env,
    address: &Address
) -> SorobanTokenAdminClient<'a> {
    SorobanTokenAdminClient::new(e, address)
}

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

    // addresses
    pub(crate) admin: Address,
    pub(crate) emergency_admin: Address,
    pub(crate) users: vec::Vec<Address>,

    pub(crate) fee_destination: Address,
    pub(crate) reward_token: Address,

    // contracts
    pub(crate) oracle_registry: contracts::oracle_registry::Client<'a>,
    pub(crate) router: contracts::pool_router::Client<'a>,
    // pub(crate) fee_collector: contracts::pool_swap_fee::Client<'a>,
    pub(crate) buffer: contracts::buffer::Client<'a>,
    pub(crate) insurance_fund: contracts::insurance_fund::Client<'a>,
    pub(crate) plane: contracts::pool_plane::Client<'a>,
    pub(crate) liquidity_calculator: contracts::liquidity_calculator::Client<'a>,

    // oracle
    pub(crate) oracle: Address,
    // pub(crate) oracle_client: MockPriceOracleClient<'a>,

    // pub(crate) btc_addr: Address,
    // pub(crate) xlm_addr: Address,

    // pub(crate) btc_asset: MockAsset,
    // pub(crate) xlm_asset: MockAsset,

    pub(crate) btc_asset_id: Symbol,
    pub(crate) xlm_asset_id: Symbol,

    // pub(crate) init_btc_price: i128,
    // pub(crate) init_xlm_price: i128,

    // insurance

    // pool
    pub(crate) pool_address: Address,

    // tokens
    pub(crate) token1: SorobanTokenClient<'a>,
    pub(crate) token1_admin_client: SorobanTokenAdminClient<'a>,
    pub(crate) token2: SorobanTokenClient<'a>,
    pub(crate) token2_admin_client: SorobanTokenAdminClient<'a>,
    // pub(crate) token_reward: SorobanTokenClient<'a>,
    pub(crate) token_reward_admin_client: SorobanTokenAdminClient<'a>,
    // pub(crate) token_share: PoolTokenClient<'a>,
}

impl Default for Setup<'_> {
    fn default() -> Self {
        Self::setup()
    }
}

impl Setup<'_> {
    pub(crate) fn setup() -> Self {
        let e: Env = Env::default();
        e.mock_all_auths();
        e.cost_estimate().budget().reset_unlimited();

        let start_time = 1753288157;
        jump(&e, start_time);

        // Addresses
        let users = Self::generate_random_users(&e, 3);
        let admin = Address::generate(&e);
        let emergency_admin = Address::generate(&e);
        let rewards_admin = soroban_sdk::Address::generate(&e);
        let operations_admin = soroban_sdk::Address::generate(&e);
        let pause_admin = soroban_sdk::Address::generate(&e);
        let emergency_pause_admin = soroban_sdk::Address::generate(&e);
        let fee_collector = Address::generate(&e);

        // Tokens
        let mut token1 = create_token_contract(&e, &admin);
        let mut token2 = create_token_contract(&e, &admin);
        let tokens = Vec::from_array(&e, [token1.address.clone(), token2.address.clone()]);
        let reward_token = create_token_contract(&e, &rewards_admin);

        let token1_admin_client = get_token_admin_client(&e, &token1.address.clone());
        let token2_admin_client = get_token_admin_client(&e, &token2.address.clone());
        let token_reward_admin_client = get_token_admin_client(&e, &reward_token.address.clone());

        // Contracts
        // Pool Router
        let pool_hash = install_liq_pool_hash(&e);
        let lp_token_hash = install_lp_token_wasm(&e);
        let synthetic_token_hash = install_synthetic_token_wasm(&e);
        let router = create_pool_router_contract(&e);
        router.init_admin(&admin);
        router.set_privileged_addrs(
            &admin,
            &rewards_admin,
            &operations_admin,
            &pause_admin,
            &Vec::from_array(&e, [emergency_pause_admin.clone()])
        );
        router.set_pool_hash(&admin, &pool_hash);
        router.set_lp_token_hash(&admin, &lp_token_hash);
        router.set_synthetic_token_hash(&admin, &synthetic_token_hash);
        router.set_reward_token(&admin, &reward_token.address);

        let emergency_admin = Address::generate(&e);
        router.commit_transfer_ownership(
            &admin,
            &Symbol::new(&e, "EmergencyAdmin"),
            &emergency_admin
        );
        router.apply_transfer_ownership(&admin, &Symbol::new(&e, "EmergencyAdmin"));

        // Pool Plane
        let plane = create_plane_contract(&e);
        router.set_pools_plane(&admin, &plane.address);

        // Liquidity Calculator
        let liquidity_calculator = create_liquidity_calculator_contract(&e);
        liquidity_calculator.init_admin(&admin);
        liquidity_calculator.set_pools_plane(&admin, &plane.address);
        router.set_liquidity_calculator(&admin, &liquidity_calculator.address);

        // Setup oracles
        let btc_asset_id = Symbol::new(&e, "BTC");
        let xlm_asset_id = Symbol::new(&e, "XLM");

        let btc_asset = MockAsset::Other(btc_asset_id.clone());
        let xlm_asset = MockAsset::Other(xlm_asset_id.clone());

        let base = MockAsset::Other(Symbol::new(&e, "USD"));

        let (oracle_id, oracle_client) = setup_price_feed_oracle(
            &e,
            &admin,
            &base,
            &Vec::from_array(&e, [btc_asset.clone(), xlm_asset.clone()]),
            7,
            300
        );

        // ===

        // let prices_1: Vec<i128> = vec![&e, 94_234_1234567, 1_1021304];
        let prices_1: Vec<i128> = Vec::from_array(&e, [10923722794294087742, 10923722794294087742]);
        oracle_client.set_price(&prices_1, &start_time);

        // verify price data can be fetched
        let result_1 = oracle_client.lastprice(&btc_asset).unwrap();
        assert_eq!(result_1.price, prices_1.get_unchecked(0));
        // assert_eq!(result_1.timestamp, start_time);

        let result_2 = oracle_client.lastprice(&xlm_asset).unwrap();
        assert_eq!(result_2.price, prices_1.get_unchecked(1));
        // assert_eq!(result_2.timestamp, start_time);

        // let last_timestamp = oracle_client.last_timestamp();
        // assert_eq!(last_timestamp, start_time);

        // pass time
        // jump(&e, 325);
        // env.ledger().set(LedgerInfo {
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
        // let result_1 = oracle_client.lastprice(&asset_1).unwrap();
        // assert_eq!(result_1.price, prices_1.get_unchecked(0));
        // assert_eq!(result_1.timestamp, start_time);

        // let result_2 = oracle_client.lastprice(&asset_2).unwrap();
        // assert_eq!(result_2.price, prices_1.get_unchecked(1));
        // assert_eq!(result_2.timestamp, start_time);

        // let last_timestamp = oracle_client.last_timestamp();
        // assert_eq!(last_timestamp, start_time);

        // set another round of prices
        // let prices_2: Vec<i128> = Vec::from_array(&e, [10923722794294087742, 10923722794294087742]);
        // oracle_client.set_price(&prices_2, &(start_time + 300));

        // // verify most recent prices are fetched
        // let result_1 = oracle_client.lastprice(&asset_1).unwrap();
        // assert_eq!(result_1.price, prices_2.get_unchecked(0));
        // assert_eq!(result_1.timestamp, start_time + 300);

        // let result_2 = oracle_client.lastprice(&asset_2).unwrap();
        // assert_eq!(result_2.price, prices_2.get_unchecked(1));
        // assert_eq!(result_2.timestamp, start_time + 300);

        // let last_timestamp = oracle_client.last_timestamp();
        // assert_eq!(last_timestamp, start_time + 300);

        // // verify old prices can be fetched
        // let result_1 = oracle_client.price(&asset_1, &start_time).unwrap();
        // assert_eq!(result_1.price, prices_1.get_unchecked(0));
        // assert_eq!(result_1.timestamp, start_time);

        // let result_2 = oracle_client.price(&asset_2, &start_time).unwrap();
        // assert_eq!(result_2.price, prices_1.get_unchecked(1));
        // assert_eq!(result_2.timestamp, start_time);

        // // verify timestamp is normalized to the most recent price
        // // older than the requested timestamp
        // let result_1 = oracle_client.price(&asset_1, &(100 + start_time)).unwrap();
        // assert_eq!(result_1.price, prices_1.get_unchecked(0));
        // assert_eq!(result_1.timestamp, start_time);

        // let result_2 = oracle_client.price(&asset_2, &(250 + start_time)).unwrap();
        // assert_eq!(result_2.price, prices_1.get_unchecked(1));
        // assert_eq!(result_2.timestamp, start_time);

        // // verify get prices can fetch both
        // let result_1_vec = oracle_client.prices(&asset_1, &2).unwrap();
        // assert_eq!(result_1_vec.len(), 2);
        // let result_1_0 = result_1_vec.get_unchecked(0);
        // assert_eq!(result_1_0.price, prices_2.get_unchecked(0));
        // assert_eq!(result_1_0.timestamp, start_time + 300);
        // let result_1_1 = result_1_vec.get_unchecked(1);
        // assert_eq!(result_1_1.price, prices_1.get_unchecked(0));
        // assert_eq!(result_1_1.timestamp, start_time);

        // let result_2_vec = oracle_client.prices(&asset_2, &2).unwrap();
        // assert_eq!(result_2_vec.len(), 2);
        // let result_2_0 = result_2_vec.get_unchecked(0);
        // assert_eq!(result_2_0.price, prices_2.get_unchecked(1));
        // assert_eq!(result_2_0.timestamp, start_time + 300);
        // let result_2_1 = result_2_vec.get_unchecked(1);
        // assert_eq!(result_2_1.price, prices_1.get_unchecked(1));
        // assert_eq!(result_2_1.timestamp, start_time);

        // // verify un-normalized timestamps get set to the most recent normalized timestamp
        // let prices_3: Vec<i128> = Vec::from_array(&e, [10923722794294087742, 10923722794294087742]);
        // oracle_client.set_price(&prices_3, &(start_time + 600 + 100));

        // let result_1 = oracle_client.lastprice(&asset_1).unwrap();
        // assert_eq!(result_1.price, prices_3.get_unchecked(0));
        // assert_eq!(result_1.timestamp, start_time + 600);

        // let result_2 = oracle_client.lastprice(&asset_2).unwrap();
        // assert_eq!(result_2.price, prices_3.get_unchecked(1));
        // assert_eq!(result_2.timestamp, start_time + 600);

        // let last_timestamp = oracle_client.last_timestamp();
        // assert_eq!(last_timestamp, start_time + 600);

        // ===

        // Setup Oracle Registry
        let registry = create_oracle_registry_contract(&e);
        registry.initialize(&admin, &emergency_admin);
        registry.set_oracle_guard_rails(
            &admin,
            &(OracleGuardRails {
                price_divergence: PriceDivergenceGuardRails {
                    oracle_twap_percent_divergence: 120_0000000,
                },
                validity: ValidityGuardRails {
                    seconds_before_stale_for_pool: 3000, // ~5 seconds
                    too_volatile_ratio: 120_0000000, // 5x or 80% down
                },
            })
        );

        router.set_oracle_registry(&admin, &registry.address);

        // Register XLM oracle
        registry.register_oracle(&admin, &btc_asset_id, &oracle_id, &14, &0);

        // Register BTC oralce
        registry.register_oracle(&admin, &xlm_asset_id, &oracle_id, &14, &0);

        // Buffer
        let buffer = create_buffer_contract(&e);
        buffer.initialize(&admin, &emergency_admin, &1000, &1000);
        // buffer.set_fee_collector(&admin, &fee_collector);

        // Insurance Fund
        let insurance_fund = create_insurance_fund_contract(&e);
        insurance_fund.initialize(
            &admin,
            &emergency_admin,
            &token2.address,
            &THIRTEEN_DAY,
            &80_00000_u32, // 80%
            &2_00000_i32, // 2%
            &(10_00000_u32, 60_00000_u32) // 10% and 60%
        );

        // Deploy a Pool
        let pool_address = router.init_pool(
            &admin,
            &(),
            &token2.address.clone(),
            &(),
            &(),
            &30,
            &PoolTier::A,
            &1_000_000_0000000u128
        );

        Self {
            env: e,
            users,
            admin,
            emergency_admin,
            fee_destination: fee_collector,

            reward_token: reward_token.address,

            // contracts
            oracle_registry: registry,
            // fee_collector: '',
            router,
            buffer,
            insurance_fund,
            plane,
            liquidity_calculator,

            // oracle
            oracle: oracle_id,
            btc_asset_id,
            xlm_asset_id,

            // pool
            pool_address,

            // tokens
            token1,
            token1_admin_client,
            token2,
            token2_admin_client,
            // token_reward: reward_token.clone(),
            token_reward_admin_client,
            // token_share,
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
}

//  ________    _______    _______   ___        ______  ___  ___
// |"      "\  /"     "|  |   __ "\ |"  |      /    " \|"  \/"  |
// (.  ___  :)(: ______)  (. |__) :)||  |     // ____  \\   \  /
// |: \   ) || \/    |    |:  ____/ |:  |    /  /    ) :)\\  \/
// (| (___\ || // ___)_   (|  /      \  |___(: (____/ // /   /
// |:       :)(:      "| /|__/ \    ( \_|:  \\        / /   /
// (________/  \_______)(_______)    \_______)\"_____/ |___/

fn create_pool_router_contract<'a>(e: &Env) -> contracts::pool_router::Client<'a> {
    contracts::pool_router::Client::new(e, &e.register(contracts::pool_router::WASM, ()))
}

fn create_oracle_registry_contract<'a>(e: &Env) -> contracts::oracle_registry::Client<'a> {
    contracts::oracle_registry::Client::new(e, &e.register(contracts::oracle_registry::WASM, ()))
}

fn create_liquidity_calculator_contract<'a>(
    e: &Env
) -> contracts::liquidity_calculator::Client<'a> {
    contracts::liquidity_calculator::Client::new(
        e,
        &e.register(contracts::liquidity_calculator::WASM, ())
    )
}

fn create_plane_contract<'a>(e: &Env) -> contracts::pool_plane::Client<'a> {
    contracts::pool_plane::Client::new(e, &e.register(contracts::pool_plane::WASM, ()))
}

fn create_pool_swap_fee_contract<'a>(e: &Env) -> contracts::pool_swap_fee::Client<'a> {
    contracts::pool_swap_fee::Client::new(e, &e.register(contracts::pool_swap_fee::WASM, ()))
}

fn create_buffer_contract<'a>(e: &Env) -> contracts::buffer::Client<'a> {
    contracts::buffer::Client::new(e, &e.register(contracts::buffer::WASM, ()))
}

fn create_insurance_fund_contract<'a>(e: &Env) -> contracts::insurance_fund::Client<'a> {
    contracts::insurance_fund::Client::new(e, &e.register(contracts::insurance_fund::WASM, ()))
}

//  ____  ____  ___________  __    ___        ________
// ("  _||_ " |("     _   ")|" \  |"  |      /"       )
// |   (  ) : | )__/  \\__/ ||  | ||  |     (:   \___/
// (:  |  | . )    \\_ /    |:  | |:  |      \___  \
//  \\ \__/ //     |.  |    |.  |  \  |___    __/  \\
//  /\\ __ //\     \:  |    /\  |\( \_|:  \  /" \   :)
// (__________)     \__|   (__\_|_)\_______)(_______/

// (https://github.com/script3/sep-40-oracle/blob/d2d9a19079d95f79c16c3ff506416346d75b537f/mock-sep-40/src/test.rs)
fn setup_price_feed_oracle<'a>(
    env: &Env,
    admin: &Address,
    base: &MockAsset,
    assets: &Vec<MockAsset>,
    decimals: u32,
    resolution: u32
) -> (Address, MockPriceOracleClient<'a>) {
    let oracle_id = env.register(MockPriceOracleWASM, ());
    let oracle_client = MockPriceOracleClient::new(env, &oracle_id);
    oracle_client.set_data(admin, base, assets, &decimals, &resolution);
    (oracle_id, oracle_client)
}
