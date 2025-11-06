#![allow(dead_code)]
#![cfg(test)]
extern crate std;
use crate::contracts;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::token::{
    StellarAssetClient as SorobanTokenAdminClient, TokenClient as SorobanTokenClient,
};
use soroban_sdk::{Address, BytesN, Env, Vec};

pub(crate) struct Setup<'a> {
    pub(crate) env: Env,
    pub(crate) admin: Address,
    pub(crate) operator: Address,
    pub(crate) emergency_admin: Address,
    pub(crate) router: contracts::router::Client<'a>,
    pub(crate) fee_destination: Address,
    pub(crate) reward_token: Address,
    pub(crate) locked_token: Address,
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
        let locked_token = create_token_contract(&e, &admin);
        let locked_token_admin = get_token_admin_client(&e, &locked_token.address);

        // init swap router
        let pool_hash = e
            .deployer()
            .upload_contract_wasm(contracts::constant_product_pool::WASM);
        let token_hash = e
            .deployer()
            .upload_contract_wasm(contracts::token_share::WASM);
        let plane = deploy_plane_contract(&e);

        let router = deploy_liqpool_router_contract(e.clone());
        router.init_admin(&admin);
        router.init_config_storage(
            &admin,
            &deploy_config_storage(&e, &admin, &emergency_admin).address,
        );
        router.set_rewards_gauge_hash(
            &admin,
            &e.deployer()
                .upload_contract_wasm(contracts::rewards_gauge::WASM),
        );
        router.set_pool_hash(&admin, &pool_hash);
        router.set_el(
            &admin,
            &e.deployer()
                .upload_contract_wasm(contracts::elastic_pool::WASM),
        );
        router.set_token_hash(&admin, &token_hash);
        router.set_reward_token(&admin, &reward_token.address);
        router.set_pools_plane(&admin, &plane.address);
        router.set_protocol_fee_fraction(&admin, &5000);

        let fee_collector_factory =
            deploy_provider_swap_fee_factory(&e, &admin, &emergency_admin, &router.address);

        Self {
            env: e,
            admin,
            operator,
            emergency_admin,
            fee_destination,
            router,
            reward_token: reward_token.address,
            locked_token: locked_token.address,
        }
    }

    pub(crate) fn deploy_standard_pool(
        &self,
        token_a: &Address,
        token_b: &Address,
        fee_fraction: u32,
    ) -> (contracts::constant_product_pool::Client, BytesN<32>) {
        get_token_admin_client(&self.env, &self.reward_token).mint(&self.admin, &10_0000000);
        let (pool_hash, pool_address) = self.router.init_standard_pool(
            &self.admin,
            &Vec::from_array(&self.env, [token_a.clone(), token_b.clone()]),
            &fee_fraction,
        );
        (
            contracts::constant_product_pool::Client::new(&self.env, &pool_address),
            pool_hash,
        )
    }

    pub(crate) fn deploy_elastic_pool(
        &self,
        token_a: &Address,
        token_b: &Address,
        fee_fraction: u32,
    ) -> (contracts::elastic_pool::Client, BytesN<32>) {
        get_token_admin_client(&self.env, &self.reward_token).mint(&self.admin, &1_0000000);
        let (pool_hash, pool_address) = self.router.init_elastic_pool(
            &self.admin,
            &Vec::from_array(&self.env, [token_a.clone(), token_b.clone()]),
            &fee_fraction,
        );
        (
            contracts::elastic_pool::Client::new(&self.env, &pool_address),
            pool_hash,
        )
    }
}

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

fn deploy_liqpool_router_contract<'a>(e: Env) -> contracts::router::Client<'a> {
    contracts::router::Client::new(&e, &e.register(contracts::router::WASM, ()))
}

fn deploy_plane_contract<'a>(e: &Env) -> contracts::pool_plane::Client {
    contracts::pool_plane::Client::new(e, &e.register(contracts::pool_plane::WASM, ()))
}

fn deploy_config_storage<'a>(
    e: &Env,
    admin: &Address,
    emergency_admin: &Address,
) -> contracts::config_storage::Client<'a> {
    contracts::config_storage::Client::new(
        e,
        &e.register(
            contracts::config_storage::WASM,
            contracts::config_storage::Args::__constructor(admin, emergency_admin),
        ),
    )
}
