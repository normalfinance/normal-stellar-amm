#![cfg(test)]
extern crate std;
use crate::contracts;
use crate::testutils::oracle_registry::{
    OracleGuardRails, PriceDivergenceGuardRails, ValidityGuardRails,
};
use sep_40_oracle::testutils::{Asset as MockAsset, MockPriceOracleClient, MockPriceOracleWASM};
use sep_40_oracle::testutils::{Asset, MockPriceOracleWASM};
use soroban_sdk::testutils::{Address as _, BytesN};
use soroban_sdk::token::{
    StellarAssetClient as SorobanTokenAdminClient, TokenClient as SorobanTokenClient,
};
use soroban_sdk::{Address, BytesN, Env, String, Symbol, Vec};
use utils::constant::PERCENTAGE_PRECISION_U64;
use utils::state::oracle_registry::PoolTier;
use utils::test_utils::{
    create_token_contract, get_mock_lp_token_info, get_mock_oracle_registry_ids,
    get_token_admin_client, install_liq_pool_hash, install_token_wasm,
};

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
    pub(crate) fee_collector: contracts::pool_swap_fee::Client<'a>,
    pub(crate) buffer: contracts::buffer::Client<'a>,
    pub(crate) insurance_fund: contracts::insurance_fund::Client<'a>,
    pub(crate) plane: contracts::pool_plane::Client<'a>,
    pub(crate) liquidity_calculator: contracts::liquidity_calculator::Client<'a>,

    // oracle
    pub(crate) oracle: Address,
    pub(crate) oracle_client: MockPriceOracleClient<'a>,

    pub(crate) btc_addr: Address,
    pub(crate) xlm_addr: Address,

    pub(crate) btc_asset: MockAsset,
    pub(crate) xlm_asset: MockAsset,

    pub(crate) btc_asset_id: Symbol,
    pub(crate) xlm_asset_id: Symbol,

    pub(crate) init_btc_price: i128,
    pub(crate) init_xlm_price: i128,

    // insurance

    // pool
    pub(crate) pool_hash: BytesN<32>,
    pub(crate) pool_address: Address,

    // tokens
    pub(crate) token1: SorobanTokenClient<'a>,
    pub(crate) token1_admin_client: SorobanTokenAdminClient<'a>,
    pub(crate) token2: SorobanTokenClient<'a>,
    pub(crate) token2_admin_client: SorobanTokenAdminClient<'a>,
    pub(crate) token_reward: SorobanTokenClient<'a>,
    pub(crate) token_reward_admin_client: SorobanTokenAdminClient<'a>,
    pub(crate) token_share: PoolTokenClient<'a>,
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

        // Addresses
        let users = Self::generate_random_users(&e, config.users_count);
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
        let reward_token = create_token_contract(&e, &reward_admin);

        let token1_admin_client = get_token_admin_client(&e, &token1.address.clone());
        let token2_admin_client = get_token_admin_client(&e, &token2.address.clone());
        let token_reward_admin_client = get_token_admin_client(&e, &reward_token.address.clone());

        // Contracts
        // Pool Router
        let pool_hash = install_liq_pool_hash(&e);
        let token_hash = install_token_wasm(&e);
        let router = create_pool_router_contract(&e);
        router.init_admin(&admin);
        router.set_privileged_addrs(
            &admin,
            &rewards_admin,
            &operations_admin,
            &pause_admin,
            &Vec::from_array(&e, [emergency_pause_admin.clone()]),
        );
        router.set_pool_hash(&admin, &pool_hash);
        router.set_token_hash(&admin, &token_hash);
        router.set_reward_token(&admin, &reward_token.address);

        let emergency_admin = Address::generate(&e);
        router.commit_transfer_ownership(
            &admin,
            &Symbol::new(&e, "EmergencyAdmin"),
            &emergency_admin,
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

        // Oracle Registry
        let btc_addr = Address::generate(&e);
        let xlm_addr = Address::generate(&e);

        let btc_asset_id = Symbol::new(&e, "BTC");
        let xlm_asset_id = Symbol::new(&e, "XLM");

        let btc_asset = MockAsset::Stellar(btc_addr.clone());
        let xlm_asset = MockAsset::Stellar(xlm_addr.clone());

        let (oracle_id, oracle_client) = setup_price_feed_oracle(
            &e,
            &admin,
            &MockAsset::Other(Symbol::new(&e, "USD")),
            &Vec::from_array(&e, [btc_asset.clone(), xlm_asset.clone()]),
            7,
            300,
        );

        // prices
        let init_btc_price = 50000_0000000_i128; // $50,000
        let init_xlm_price = 0_5000000_i128; // $0.50
        let prices: Vec<i128> = Vec::from_array(&e, [init_btc_price, init_xlm_price]);
        oracle_client.set_price(&prices, &e.ledger().timestamp());

        let registry = create_oracle_registry_contract(&e);
        registry.initialize(&admin, &emergency_admin);
        registry.set_oracle_guardrails(
            &admin,
            &(OracleGuardRails {
                price_divergence: PriceDivergenceGuardRails {
                    oracle_twap_percent_divergence: PERCENTAGE_PRECISION_U64 / 2,
                },
                validity: ValidityGuardRails {
                    seconds_before_stale_for_pool: 10,    // ~5 seconds
                    confidence_interval_max_size: 20_000, // 2% of price
                    too_volatile_ratio: 5,                // 5x or 80% down
                },
            }),
        );
        registry.register_oracle(&admin, &btc_asset_id, &oracle_id, &btc_addr, &7, &0);

        // Buffer
        let buffer = create_buffer_contract(&e);
        buffer.initialize(&admin, &emergency_admin, &router.address);
        buffer.set_fee_collector(&admin, &fee_collector);

        // Insurance Fund
        let insurance_fund = create_insurance_fund_contract(&e);
        insurance_fund.initialize(
            &admin,
            &emergency_admin,
            &token2.address,
            &0,
            &80_00000_u32,                 // 80%
            &2_00000_i32,                  // 2%
            &(10_00000_i32, 60_00000_i32), // 10% and 60%
        );

        // Deploy a Pool
        let (pool_hash, pool_address) = router.init_pool(
            &admin,
            &get_mock_oracle_registry_ids(&e),
            &btc_addr,
            &tokens,
            &get_mock_lp_token_info(&e),
            &30,
            &PoolTier::A,
            &1_000_000_u128,
        );

        Self {
            env: e,
            users,
            admin,
            emergency_admin,
            fee_destination,

            reward_token: reward_token.address,

            // contracts
            oracle_registry: registry,
            fee_collector,
            router,
            buffer,
            insurance_fund,
            plane,
            liquidity_calculator,

            // oracle

            // pool
            pool_hash,
            pool_address,

            // tokens
            token1,
            token1_admin_client,
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
}

//  ________    _______    _______   ___        ______  ___  ___
// |"      "\  /"     "|  |   __ "\ |"  |      /    " \|"  \/"  |
// (.  ___  :)(: ______)  (. |__) :)||  |     // ____  \\   \  /
// |: \   ) || \/    |    |:  ____/ |:  |    /  /    ) :)\\  \/
// (| (___\ || // ___)_   (|  /      \  |___(: (____/ // /   /
// |:       :)(:      "| /|__/ \    ( \_|:  \\        / /   /
// (________/  \_______)(_______)    \_______)\"_____/ |___/

fn create_pool_router_contract<'a>(e: Env) -> contracts::pool_router::Client<'a> {
    contracts::pool_router::Client::new(&e, &e.register(contracts::pool_router::WASM, ()))
}

fn create_oracle_registry_contract<'a>(e: Env) -> contracts::oracle_registry::Client<'a> {
    contracts::oracle_registry::Client::new(&e, &e.register(contracts::oracle_registry::WASM, ()))
}

fn create_liquidity_calculator_contract<'a>(
    e: &Env,
) -> contracts::liquidity_calculator::Client<'a> {
    contracts::liquidity_calculator::Client::new(
        e,
        &e.register(contracts::liquidity_calculator::WASM, ()),
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

fn create_insurance_fund_contract<'a>(e: &Env) -> contracts::insurnace_fund::Client<'a> {
    contracts::insurnace_fund::Client::new(e, &e.register(contracts::insurnace_fund::WASM, ()))
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
    resolution: u32,
) -> (Address, MockPriceOracleClient<'a>) {
    let oracle_id = env.register(MockPriceOracleWASM, ());
    let oracle_client = MockPriceOracleClient::new(env, &oracle_id);
    oracle_client.set_data(admin, base, assets, &decimals, &resolution);
    (oracle_id, oracle_client)
}
