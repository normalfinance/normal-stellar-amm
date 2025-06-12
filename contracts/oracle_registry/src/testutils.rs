#![cfg(test)]
extern crate std;
use crate::storage_types::OracleGuardRails;
use crate::OracleRegistryClient;
use sep_40_oracle::testutils::{ Asset as MockAsset, MockPriceOracleClient, MockPriceOracleWASM };
use soroban_sdk::testutils::Address as _;
use soroban_sdk::token::{
    StellarAssetClient as SorobanTokenAdminClient,
    TokenClient as SorobanTokenClient,
};
use soroban_sdk::{ Address, BytesN, Env, String, Symbol, Vec };
use utils::storage::{ OraclePair };
use utils::test_utils::{ create_token_contract, get_token_admin_client, setup_oracle_registry };

pub(crate) struct TestConfig {
    pub(crate) default_oracle_guardrails: OracleGuardRails,
}

impl Default for TestConfig {
    fn default() -> Self {
        TestConfig {
            default_oracle_guardrails: OracleGuardRails {
                price_divergence: PriceDivergenceGuardRails {
                    oracle_twap_percent_divergence: PERCENTAGE_PRECISION_U64 / 2,
                },
                validity: ValidityGuardRails {
                    slots_before_stale_for_pool: 10, // ~5 seconds
                    confidence_interval_max_size: 20_000, // 2% of price
                    too_volatile_ratio: 5, // 5x or 80% down
                },
            },
        }
    }
}

pub(crate) struct Setup<'a> {
    pub(crate) env: Env,

    // addresses
    pub(crate) admin: Address,
    pub(crate) user: Address,

    // contracts
    pub(crate) oracle_registry: OracleRegistryClient<'a>, // oracle_registry::Client<'a>,

    // oracles
    pub(crate) asset_id: Symbol,
    pub(crate) unregistered_asset_id: Symbol,
    pub(crate) oracle_client: MockPriceOracleClient<'a>,

    // state
    pub(crate) oracle_guardrails: OracleGuardRails,
    pub(crate) initial_oracle_price: u128,
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
        let user = Address::generate(&e);

        let token = create_token_contract(&e, &admin);

        let oracle_registry = setup_oracle_registry(&e, &admin, &Address::generate(&e));

        // register oracle
        let oracle_client = MockPriceOracleClient::new(&e, &Address::generate(&e));
        let initial_oracle_price = 100_u128;
        oracle_client.set_data(
            &admin,
            &MockAsset::Stellar(token),
            &Vec::from_array(&e, [MockAsset::Other(Symbol::new(&e, "BTC"))]),
            &7,
            &(5 * 60 * 60)
        );
        oracle_client.set_price(
            &Vec::from_array(&e, [initial_oracle_price as i128]),
            &e.ledger().timestamp()
        );

        Self {
            env: e,
            admin,
            user,
            oracle_registry,
            asset_id: Symbol::new(&e, "BTC"),
            unregistered_asset_id: Symbol::new(&e, "ETH"),
            oracle_client,
            oracle_guardrails: config.default_oracle_guardrails,
            initial_oracle_price,
        }
    }
}
