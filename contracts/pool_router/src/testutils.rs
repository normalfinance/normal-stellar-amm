#![cfg(test)]
extern crate std;

use crate::PoolRouterClient;
use sep_40_oracle::testutils::MockPriceOracleWASM;
use sep_40_oracle::Asset;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, BytesN, Env, Symbol, Vec};
use utils::storage::OraclePair;

pub(crate) mod test_token {
    use soroban_sdk::contractimport;
    contractimport!(file = "../../target/wasm32v1-none/release/soroban_token_contract.wasm");
}

pub fn create_token_contract<'a>(e: &Env, admin: &Address) -> test_token::Client<'a> {
    test_token::Client::new(
        e,
        &e.register_stellar_asset_contract_v2(admin.clone())
            .address(),
    )
}

pub fn create_pool_router_contract<'a>(e: &Env) -> PoolRouterClient<'a> {
    let router = PoolRouterClient::new(e, &e.register(crate::PoolRouter {}, ()));
    router
}

pub fn install_token_wasm(e: &Env) -> BytesN<32> {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/soroban_token_contract.wasm"
    );
    e.deployer().upload_contract_wasm(WASM)
}

pub mod pool {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/soroban_pool_contract.wasm"
    );
}

pub fn install_liq_pool_hash(e: &Env) -> BytesN<32> {
    e.deployer().upload_contract_wasm(pool::WASM)
}

pub(crate) struct Setup<'a> {
    pub(crate) env: Env,

    pub(crate) admin: Address,

    pub(crate) oracles: OraclePair,
    pub(crate) asset: Address,

    pub(crate) tokens: [test_token::Client<'a>; 4],
    pub(crate) reward_token: test_token::Client<'a>,

    pub(crate) router: PoolRouterClient<'a>,

    pub(crate) emergency_admin: Address,
    pub(crate) rewards_admin: Address,
    pub(crate) operations_admin: Address,
    pub(crate) pause_admin: Address,
    pub(crate) emergency_pause_admin: Address,
}

impl Default for Setup<'_> {
    // Create setup from default config and mint tokens for all users & set rewards config
    fn default() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        env.cost_estimate().budget().reset_unlimited();

        let admin = Address::generate(&env);

        let mut tokens = std::vec![
            create_token_contract(&env, &admin).address,
            create_token_contract(&env, &admin).address,
            create_token_contract(&env, &admin).address,
            create_token_contract(&env, &admin).address
        ];
        tokens.sort();
        let tokens = [
            test_token::Client::new(&env, &tokens[0]),
            test_token::Client::new(&env, &tokens[1]),
            test_token::Client::new(&env, &tokens[2]),
            test_token::Client::new(&env, &tokens[3]),
        ];

        let oracles = OraclePair {
            base_oracle: env.register(MockPriceOracleWASM, ()),
            quote_oracle: env.register(MockPriceOracleWASM, ()),
        };

        let asset = Asset::Other(Symbol::new(&env, "SOL"));

        let reward_admin = Address::generate(&env);
        let admin = Address::generate(&env);
        let payment_for_creation_address = Address::generate(&env);

        let reward_token = create_token_contract(&env, &reward_admin);

        let pool_hash = install_liq_pool_hash(&env);
        let token_hash = install_token_wasm(&env);
        let router = create_pool_router_contract(&env);
        router.init_admin(&admin);
        let rewards_admin = soroban_sdk::Address::generate(&env);
        let operations_admin = soroban_sdk::Address::generate(&env);
        let pause_admin = soroban_sdk::Address::generate(&env);
        let emergency_pause_admin = soroban_sdk::Address::generate(&env);
        router.set_privileged_addrs(
            &admin,
            &rewards_admin,
            &operations_admin,
            &pause_admin,
            &Vec::from_array(&env, [emergency_pause_admin.clone()]),
        );
        router.set_pool_hash(&admin, &pool_hash);
        router.set_token_hash(&admin, &token_hash);
        router.set_reward_token(&admin, &reward_token.address);

        let emergency_admin = Address::generate(&env);
        router.commit_transfer_ownership(
            &admin,
            &Symbol::new(&env, "EmergencyAdmin"),
            &emergency_admin,
        );
        router.apply_transfer_ownership(&admin, &Symbol::new(&env, "EmergencyAdmin"));

        Setup {
            env,
            admin,
            oracles,
            asset,
            tokens,
            reward_token,
            router,
            emergency_admin,
            rewards_admin,
            operations_admin,
            pause_admin,
            emergency_pause_admin,
        }
    }
}
