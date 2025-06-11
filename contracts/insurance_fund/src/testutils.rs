#![cfg(test)]
extern crate std;
use crate::InsuranceFundClient;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::token::{
    StellarAssetClient as SorobanTokenAdminClient,
    TokenClient as SorobanTokenClient,
};
use soroban_sdk::{ Address, BytesN, Env, String, Symbol, Vec };
use utils::storage::{ OraclePair, PoolTier };

pub(crate) struct TestConfig {
    pub(crate) min_time_between_payouts: u64,
}

impl Default for TestConfig {
    fn default() -> Self {
        TestConfig {
            min_time_between_payouts: 30, // 30 seconds
        }
    }
}

pub(crate) struct Setup<'a> {
    pub(crate) env: Env,
    pub(crate) admin: Address,
    pub(crate) insurance_fund: InsuranceFundClient<'a>,
    pub(crate) router: pool_router::Client<'a>,
    pub(crate) oracle_registry: oracle::Client<'a>,
    pub(crate) fee_collector: fee_collector::Client<'a>,

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
        let asset = Address::generate(&e);
        let fee_collector = Address::generate(&e);

        let token_a = create_token_contract(&e, &admin);
        let token_b = create_token_contract(&e, &admin);

        let token_a_admin_client = get_token_admin_client(&e, &token_a.address.clone());
        let token_b_admin_client = get_token_admin_client(&e, &token_b.address.clone());

        // init swap router with all it's complexity
        let pool_hash = install_liq_pool_hash(&e);
        let token_hash = install_token_wasm(&e);
        let router = deploy_pool_router_contract(e.clone());
        router.init_admin(&admin);
        router.set_pool_hash(&admin, &pool_hash);
        router.set_token_hash(&admin, &token_hash);

        let oracle_registry = deploy_oracle_registry_contract(&e);

        // create swap pool & deposit initial liquidity
        let (_, pool_address) = router.init_pool(
            &admin,
            &("", ""),
            &asset,
            &Vec::from_array(&e, [token_a.address.clone(), token_b.address.clone()]),
            &(String::from_str(&e, "Pool Share Token"), String::from_str(&e, "Pool Share Token")),
            &30,
            &PoolTier::A,
            &1_000_000_u128,
            &oracle_registry.address
        );

        let swap_pool = pool::Client::new(&e, &pool_address);
        token_b_admin_client.mint(&admin, &1_000_000_000_0000000);
        swap_pool.deposit(&admin, &1_000_000_000_0000000);

        // init fee collector
        let fee_collector = deploy_fee_collector_contract(e.clone());

        let insurance_fund = create_insurance_fund(
            &e,
            &admin,
            &router.address,
            &fee_collector.address,
            config.min_time_between_payouts
        );

        Self {
            env: e,
            admin,
            insurance_fund,
            router,
            oracle_registry,
            fee_collector,
            token_a,
            token_a_admin_client,
            token_b,
            token_b_admin_client,
        }
    }
}

pub(crate) fn create_token_contract<'a>(e: &Env, admin: &Address) -> SorobanTokenClient<'a> {
    SorobanTokenClient::new(e, &e.register_stellar_asset_contract_v2(admin.clone()).address())
}

pub mod pool {
    soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/pool.wasm");
}

pub(crate) fn get_token_admin_client<'a>(
    e: &Env,
    address: &Address
) -> SorobanTokenAdminClient<'a> {
    SorobanTokenAdminClient::new(e, address)
}

pub fn create_contract<'a>(
    e: &Env,
    admin: &Address,
    router: &Address,
    fee_collector: &Address,
    min_time_between_payouts: u64
) -> InsuranceFundClient<'a> {
    let contract = InsuranceFundClient::new(
        e,
        &e.register(crate::InsuranceFund, (admin, router, fee_collector, min_time_between_payouts))
    );
    contract
}

pub mod pool_router {
    soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/pool_router.wasm");
}

fn deploy_pool_router_contract<'a>(e: Env) -> pool_router::Client<'a> {
    pool_router::Client::new(&e, &e.register(pool_router::WASM, ()))
}

pub mod oracle_registry {
    soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/oracle_registry.wasm");
}

fn deploy_oracle_registry_contract<'a>(e: Env) -> oracle_registry::Client<'a> {
    oracle_registry::Client::new(&e, &e.register(oracle_registry::WASM, ()))
}

fn install_token_wasm(e: &Env) -> BytesN<32> {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/soroban_token_contract.wasm"
    );
    e.deployer().upload_contract_wasm(WASM)
}

fn install_liq_pool_hash(e: &Env) -> BytesN<32> {
    soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/pool.wasm");
    e.deployer().upload_contract_wasm(WASM)
}
