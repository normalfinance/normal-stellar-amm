#![cfg(test)]
extern crate std;

use crate::PoolRouterClient;
use sep_40_oracle::testutils::MockPriceOracleWASM;
use sep_40_oracle::Asset;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{ Address, BytesN, Env, Symbol, Vec };
use utils::storage::OraclePair;
use utils::test_utils::{ create_token_contract, setup_pool_router };

pub(crate) mod test_token {
    use soroban_sdk::contractimport;
    contractimport!(file = "../../target/wasm32v1-none/release/soroban_token_contract.wasm");
}

pub(crate) struct Setup<'a> {
    pub(crate) e: Env,

    // addresses
    pub(crate) admin: Address,
    pub(crate) asset: Address,
    pub(crate) emergency_admin: Address,
    pub(crate) rewards_admin: Address,
    pub(crate) operations_admin: Address,
    pub(crate) pause_admin: Address,
    pub(crate) emergency_pause_admin: Address,

    // contracts
    pub(crate) router: PoolRouterClient<'a>,

    // tokens
    pub(crate) tokens: [test_token::Client<'a>; 4],
    pub(crate) reward_token: test_token::Client<'a>,
}

impl Default for Setup<'_> {
    // Create setup from default config and mint tokens for all users & set rewards config
    fn default() -> Self {
        let e = Env::default();
        e.mock_all_auths();
        e.cost_estimate().budget().reset_unlimited();

        let admin = Address::generate(&e);
        let asset = Address::generate(&e);

        let mut tokens = std::vec![
            create_token_contract(&e, &admin).address,
            create_token_contract(&e, &admin).address,
            create_token_contract(&e, &admin).address,
            create_token_contract(&e, &admin).address
        ];
        tokens.sort();
        let tokens = [
            test_token::Client::new(&e, &tokens[0]),
            test_token::Client::new(&e, &tokens[1]),
            test_token::Client::new(&e, &tokens[2]),
            test_token::Client::new(&e, &tokens[3]),
        ];

        let reward_admin = Address::generate(&e);
        let admin = Address::generate(&e);
        let payment_for_creation_address = Address::generate(&e);

        let reward_token = create_token_contract(&e, &reward_admin);

        let router = setup_pool_router(&e, &admin);

        let rewards_admin = Address::generate(&e);
        let operations_admin = Address::generate(&e);
        let pause_admin = Address::generate(&e);
        let emergency_pause_admin = Address::generate(&e);
        router.set_privileged_addrs(
            &admin,
            &rewards_admin,
            &operations_admin,
            &pause_admin,
            &Vec::from_array(&e, [emergency_pause_admin.clone()])
        );

        // let emergency_admin = Address::generate(&e);
        // router.commit_transfer_ownership(
        //     &admin,
        //     &Symbol::new(&e, "EmergencyAdmin"),
        //     &emergency_admin
        // );
        // router.apply_transfer_ownership(&admin, &Symbol::new(&e, "EmergencyAdmin"));

        Setup {
            env: e,
            admin,

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
