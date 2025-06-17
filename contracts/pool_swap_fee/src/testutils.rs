#![cfg(test)]
extern crate std;
use crate::PoolSwapFeeCollectorClient;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::token::{
    StellarAssetClient as SorobanTokenAdminClient,
    TokenClient as SorobanTokenClient,
};
use soroban_sdk::{ Address, Env, Vec };
use utils::test_utils::{
    buffer,
    create_token_contract,
    get_token_admin_client,
    pool_router,
    setup_fee_collector,
    setup_mock_pool,
    setup_oracle_registry,
    setup_pool_router,
};

pub(crate) struct TestConfig {
    pub(crate) max_provider_fee: u32,
}

impl Default for TestConfig {
    fn default() -> Self {
        TestConfig {
            max_provider_fee: 100, // 1% fee
        }
    }
}

pub(crate) struct Setup<'a> {
    pub(crate) env: Env,

    // addresses
    pub(crate) admin: Address,
    pub(crate) fee_destination: Address,

    // contracts
    pub(crate) fee_collector: PoolSwapFeeCollectorClient<'a>,
    pub(crate) router: pool_router::Client<'a>,
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

        let admin = Address::generate(&e);
        let fee_destination = Address::generate(&e);
        let asset = Address::generate(&e);

        let token_a = create_token_contract(&e, &admin);
        let token_b = create_token_contract(&e, &admin);

        let token_a_admin_client = get_token_admin_client(&e, &token_a.address.clone());
        let token_b_admin_client = get_token_admin_client(&e, &token_b.address.clone());

        // Setup auxilary contracts
        let plane = deploy_plane_contract(&e);

        let oracle_registry = setup_oracle_registry(&e, &admin, &asset);
        let router = setup_pool_router(&e, &admin);
        let fee_collector = setup_fee_collector(
            &e,
            &admin,
            &router.address,
            &buffer.address,
            &fee_destination
        );
        fee_collector.init_admin(admin);
        fee_collector.set_router(admin, router);
        fee_collector.set_buffer(admin, buffer);
        fee_collector.set_fee_destination(admin, fee_destination);

        // Finish setting up the Buffer
        buffer.set_fee_collector(&admin, &fee_collector.address);

        // create swap pool & deposit initial liquidity
        setup_mock_pool(
            &e,
            &router,
            &admin,
            &asset,
            &Vec::from_array(&e, [token_a.address.clone(), token_b.address.clone()]),
            &oracle_registry.address,
            &token_b_admin_client
        );

        Self {
            env: e,
            admin,
            fee_destination,
            fee_collector,
            router,
            buffer,
            token_a,
            token_a_admin_client,
            token_b,
            token_b_admin_client,
        }
    }
}

fn deploy_plane_contract<'a>(e: &Env) -> Address {
    soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/pool_plane.wasm");
    Client::new(e, &e.register(WASM, ())).address
}
