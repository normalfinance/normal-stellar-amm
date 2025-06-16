#![cfg(test)]
extern crate std;
use crate::contracts;
use sep_40_oracle::testutils::{ Asset, MockPriceOracleWASM };
use soroban_sdk::testutils::Address as _;
use soroban_sdk::token::{
    StellarAssetClient as SorobanTokenAdminClient,
    TokenClient as SorobanTokenClient,
};
use soroban_sdk::{ Address, BytesN, Env, String, Symbol, Vec };
use utils::storage::{ PoolTier };

pub(crate) struct Setup<'a> {
    pub(crate) env: Env,
    pub(crate) admin: Address,
    pub(crate) operator: Address,
    pub(crate) emergency_admin: Address,
    pub(crate) fee_collector: contracts::swap_fee::Client<'a>,
    pub(crate) router: contracts::router::Client<'a>,
    pub(crate) oracle_registry: contracts::oracle_registry::Client<'a>,
    pub(crate) fee_destination: Address,
    pub(crate) reward_token: Address,
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

        let admin = Address::generate(&e);
        let operator = Address::generate(&e);
        let emergency_admin = Address::generate(&e);
        let fee_destination = Address::generate(&e);

        let reward_token = create_token_contract(&e, &admin);

        // init swap router
        let pool_hash = e.deployer().upload_contract_wasm(contracts::constant_product_pool::WASM);
        let token_hash = e.deployer().upload_contract_wasm(contracts::lp_token::WASM);

        let router = deploy_pool_router_contract(e.clone());
        router.init_admin(&admin);
        router.set_pool_hash(&admin, &pool_hash);
        router.set_token_hash(&admin, &token_hash);
        router.set_reward_token(&admin, &reward_token.address);

        let oracle_registry = deploy_oracle_registry_contract(e.clone());

        let fee_collector = deploy_provider_swap_fee_contract(
            &e,
            &admin,
            &emergency_admin,
            &router.address
        );

        Self {
            env: e,
            admin,
            operator,
            emergency_admin,
            fee_destination,
            oracle_registry,
            fee_collector,
            router,
            reward_token: reward_token.address,
        }
    }

    pub(crate) fn deploy_pool(
        &self,
        token_a: &Address,
        token_b: &Address,
        fee_fraction: u32
    ) -> (contracts::constant_product_pool::Client, BytesN<32>) {
        get_token_admin_client(&self.env, &self.reward_token).mint(&self.admin, &10_0000000);

        let (pool_hash, pool_address) = self.router.init_pool(
            &self.admin,
            &("", ""),
            &Asset::Other(Symbol::new(&self.env, "SOL")),
            &Vec::from_array(&self.env, [token_a.clone(), token_b.clone()]),
            &(
                String::from_str(&self.env, "Pool Share Token"),
                &String::from_str(&self.env, "Pool Share Token"),
            ),
            &fee_fraction,
            &PoolTier::A,
            &1_000_000_u128,
            &self.oracle_registry.address
        );
        (contracts::constant_product_pool::Client::new(&self.env, &pool_address), pool_hash)
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

pub fn deploy_provider_swap_fee_contract<'a>(
    e: &Env,
    admin: &Address,
    emergency_admin: &Address,
    router: &Address
) -> contracts::swap_fee::Client<'a> {
    contracts::swap_fee::Client::new(
        e,
        &e.register(contracts::swap_fee::WASM, (admin, emergency_admin, router))
    )
}

fn deploy_pool_router_contract<'a>(e: Env) -> contracts::router::Client<'a> {
    contracts::router::Client::new(&e, &e.register(contracts::router::WASM, ()))
}

fn deploy_oracle_registry_contract<'a>(e: Env) -> contracts::oracle_registry::Client<'a> {
    contracts::oracle_registry::Client::new(&e, &e.register(contracts::oracle_registry::WASM, ()))
}
