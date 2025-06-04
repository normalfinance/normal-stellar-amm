#![cfg(test)]
extern crate std;
use crate::ProviderSwapFeeCollectorClient;
use sep_40_oracle::testutils::MockPriceOracleWASM;
use sep_40_oracle::Asset;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::token::{
    StellarAssetClient as SorobanTokenAdminClient,
    TokenClient as SorobanTokenClient,
};
use soroban_sdk::{ Address, BytesN, Env, String, Symbol, Vec };
use utils::storage::{ OraclePair };

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
    pub(crate) admin: Address,
    pub(crate) contract: ProviderSwapFeeCollectorClient<'a>,
    pub(crate) router: swap_router::Client<'a>,
    pub(crate) operator: Address,
    pub(crate) buffer: buffer::Client<'a>,
    pub(crate) fee_destination: Address,
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
        let operator = Address::generate(&e);
        let fee_destination = Address::generate(&e);

        let token_a = create_token_contract(&e, &admin);
        let token_b = create_token_contract(&e, &admin);

        let token_a_admin_client = get_token_admin_client(&e, &token_a.address.clone());
        let token_b_admin_client = get_token_admin_client(&e, &token_b.address.clone());

        // init swap router with all it's complexity
        let pool_hash = install_liq_pool_hash(&e);
        let token_hash = install_token_wasm(&e);
        let router = deploy_liqpool_router_contract(e.clone());
        router.init_admin(&admin);
        router.set_pool_hash(&admin, &pool_hash);
        router.set_token_hash(&admin, &token_hash);

        let buffer = deploy_buffer_contract(e.clone());

        let oracles = OraclePair {
            base_oracle:  e.register(MockPriceOracleWASM, ()),
            quote_oracle:  e.register(MockPriceOracleWASM, ()),
        };

        // create swap pool & deposit initial liquidity
        let (_, pool_address) = router.init_standard_pool(
            &admin,
            &oracles,
            &Asset::Other(Symbol::new(&e, "SOL")),
            &Vec::from_array(&e, [token_a.address.clone(), token_b.address.clone()]),
            &String::from_str(&e, "Pool Share Token"),
            &String::from_str(&e, "Pool Share Token"),
            &30
        );
        let swap_pool = liquidity_pool::Client::new(&e, &pool_address);
        token_b_admin_client.mint(&admin, &1_000_000_000_0000000);
        swap_pool.deposit(&admin, &1_000_000_000_0000000);

        let contract = create_contract(
            &e,
            &router.address,
            &operator,
            &fee_destination,
            config.max_provider_fee
        );

        Self {
            env: e,
            admin,
            operator,
            fee_destination,
            contract,
            router,
            buffer,
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

pub mod liquidity_pool {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/soroban_liquidity_pool_contract.wasm"
    );
}

pub(crate) fn get_token_admin_client<'a>(
    e: &Env,
    address: &Address
) -> SorobanTokenAdminClient<'a> {
    SorobanTokenAdminClient::new(e, address)
}

pub fn create_contract<'a>(
    e: &Env,
    router: &Address,
    operator: &Address,
    fee_destination: &Address,
    swap_fee: u32
) -> ProviderSwapFeeCollectorClient<'a> {
    let contract = ProviderSwapFeeCollectorClient::new(
        e,
        &e.register(crate::ProviderSwapFeeCollector, (router, operator, fee_destination, swap_fee))
    );
    contract
}

pub mod swap_router {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/soroban_liquidity_pool_router_contract.wasm"
    );
}

fn deploy_liqpool_router_contract<'a>(e: Env) -> swap_router::Client<'a> {
    swap_router::Client::new(&e, &e.register(swap_router::WASM, ()))
}

pub mod buffer {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/soroban_buffer_contract.wasm"
    );
}

fn deploy_buffer_contract<'a>(e: Env) -> buffer::Client<'a> {
    buffer::Client::new(&e, &e.register(buffer::WASM, ()))
}

fn install_token_wasm(e: &Env) -> BytesN<32> {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/soroban_token_contract.wasm"
    );
    e.deployer().upload_contract_wasm(WASM)
}

fn install_liq_pool_hash(e: &Env) -> BytesN<32> {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/soroban_liquidity_pool_contract.wasm"
    );
    e.deployer().upload_contract_wasm(WASM)
}
