#![cfg(any(test, feature = "testutils"))]

// use sep_40_oracle::testutils::{ MockPriceOracleClient, MockPriceOracleWASM };
use soroban_sdk::testutils::{ Ledger, LedgerInfo };
use soroban_sdk::{ Address, BytesN, Env, String, Symbol, Vec, U256 };
use soroban_sdk::token::{
    StellarAssetClient as SorobanTokenAdminClient,
    TokenClient as SorobanTokenClient,
};

use crate::test_utils::pool_router::PoolTier;

pub fn assert_approx_eq_abs(a: u128, b: u128, delta: u128) {
    assert!(
        a > b - delta && a < b + delta,
        "assertion failed: `(left != right)` \
         (left: `{:?}`, right: `{:?}`, epsilon: `{:?}`)",
        a,
        b,
        delta
    );
}

pub fn assert_approx_eq_abs_u256(a: U256, b: U256, delta: U256) {
    assert!(
        a > b.sub(&delta) && a < b.add(&delta),
        "assertion failed: `(left != right)` \
         (left: `{:?}`, right: `{:?}`, epsilon: `{:?}`)",
        a,
        b,
        delta
    );
}

pub fn jump(e: &Env, time: u64) {
    e.ledger().set(LedgerInfo {
        timestamp: e.ledger().timestamp().saturating_add(time),
        protocol_version: e.ledger().protocol_version(),
        sequence_number: e.ledger().sequence(),
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 999999,
        min_persistent_entry_ttl: 999999,
        max_entry_ttl: u32::MAX,
    });
}

pub fn jump_sequence(e: &Env, sequence: u32) {
    e.ledger().set(LedgerInfo {
        timestamp: e.ledger().timestamp(),
        protocol_version: e.ledger().protocol_version(),
        sequence_number: e.ledger().sequence().saturating_add(sequence),
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 999999,
        min_persistent_entry_ttl: 999999,
        max_entry_ttl: u32::MAX,
    });
}

pub fn install_dummy_wasm<'a>(e: &Env) -> BytesN<32> {
    soroban_sdk::contractimport!(file = "../../wasm/dummy_contract.wasm");
    e.deployer().upload_contract_wasm(WASM)
}

//   ______    ______    _____  ___    ________  ___________   __      _____  ___  ___________  ________
//  /" _  "\  /    " \  (\"   \|"  \  /"       )("     _   ") /""\    (\"   \|"  \("     _   ")/"       )
// (: ( \___)// ____  \ |.\\   \    |(:   \___/  )__/  \\__/ /    \   |.\\   \    |)__/  \\__/(:   \___/
//  \/ \    /  /    ) :)|: \.   \\  | \___  \       \\_ /   /' /\  \  |: \.   \\  |   \\_ /    \___  \
//  //  \ _(: (____/ // |.  \    \. |  __/  \\      |.  |  //  __'  \ |.  \    \. |   |.  |     __/  \\
// (:   _) \\        /  |    \    \ | /" \   :)     \:  | /   /  \\  \|    \    \ |   \:  |    /" \   :)
//  \_______)\"_____/    \___|\____\)(_______/       \__|(___/    \___)\___|\____\)    \__|   (_______/

pub fn get_mock_oracle_registry_ids<'a>(e: &Env) -> (Symbol, Symbol) {
    (Symbol::new(e, "BTC"), Symbol::new(e, "XLM"))
}

pub fn get_mock_lp_token_info<'a>(e: &Env) -> (String, String) {
    (String::from_str(e, "Pool Share Token"), String::from_str(e, "Pool Share Token"))
}

//  ____  ____  ___________  __    ___        ________
// ("  _||_ " |("     _   ")|" \  |"  |      /"       )
// |   (  ) : | )__/  \\__/ ||  | ||  |     (:   \___/
// (:  |  | . )    \\_ /    |:  | |:  |      \___  \
//  \\ \__/ //     |.  |    |.  |  \  |___    __/  \\
//  /\\ __ //\     \:  |    /\  |\( \_|:  \  /" \   :)
// (__________)     \__|   (__\_|_)\_______)(_______/

// was pub(crate)
pub fn get_token_admin_client<'a>(e: &Env, address: &Address) -> SorobanTokenAdminClient<'a> {
    SorobanTokenAdminClient::new(e, address)
}

// init swap router with all it's complexity
pub fn setup_pool_router<'a>(e: &Env, admin: &Address) -> pool_router::Client<'a> {
    let pool_hash = install_liq_pool_hash(e);
    let token_hash = install_token_wasm(e);
    let router = deploy_pool_router_contract(e.clone());
    router.init_admin(admin);
    router.set_pool_hash(admin, &pool_hash);
    router.set_token_hash(admin, &token_hash);

    router
}

// create swap pool & deposit initial liquidity
pub fn setup_mock_pool<'a>(
    e: &Env,
    router: &pool_router::Client<'a>,
    admin: &Address,
    asset: &Address,
    tokens: &Vec<Address>,
    oracle_registry: &Address,
    token_client: &SorobanTokenAdminClient<'a>
) {
    let (_, pool_address) = router.init_pool(
        admin,
        &get_mock_oracle_registry_ids(&e),
        asset,
        tokens,
        &get_mock_lp_token_info(&e),
        &30,
        &PoolTier::A,
        &1_000_000u128,
        oracle_registry
    );
    let swap_pool = pool::Client::new(&e, &pool_address);
    token_client.mint(&admin, &1_000_000_000_0000000);
    swap_pool.deposit(&admin, &1_000_000_000_0000000);
}

pub fn setup_buffer<'a>(e: &Env, admin: &Address, router: &Address) -> buffer::Client<'a> {
    let buffer = deploy_buffer_contract(e);
    buffer.init_admin(admin);
    buffer.set_router(admin, router);
    // manually set fee_collector later once it's initialized...

    buffer
}

pub fn setup_oracle_registry<'a>(
    e: &Env,
    admin: &Address,
    asset: &Address
) -> oracle_registry::Client<'a> {
    let oracle_registry = deploy_oracle_registry_contract(e);
    oracle_registry.init_admin(admin);
    oracle_registry.register_oracle(
        admin,
        &Symbol::new(e, "BTC"),
        &e.register(pool_router::WASM, ()), // MockPriceOracleWASM
        asset,
        &7
    );

    oracle_registry
}

pub fn setup_fee_collector<'a>(
    e: &Env,
    admin: &Address,
    router: &Address,
    buffer: &Address,
    fee_destination: &Address
) -> fee_collector::Client<'a> {
    let fee_collector = deploy_fee_collector_contract(e);
    fee_collector.init_admin(admin);
    fee_collector.set_router(admin, router);
    fee_collector.set_buffer(admin, buffer);
    fee_collector.set_fee_destination(admin, fee_destination);

    fee_collector
}

pub fn setup_oracles<'a>(e: &Env, address: &Address) {
    // let base_oracle_client = MockPriceOracleClient::new(e, address);
    // let quote_oracle_client = MockPriceOracleClient::new(e, address);

    // Setup base oracle
    // base_oracle_client.set_data(
    //     &admin,
    //     &MockAsset::Stellar(usdc.clone()),
    //     &Vec::from_array(e, [asset_mock.clone()]),
    //     &7,
    //     &(5 * 60 * 60)
    // );
    // base_oracle_client.set_price(
    //     &Vec::from_array(e, [base_oracle_price]),
    //     e.ledger().timestamp()
    // );

    // // Setup quote oracle
    // quote_oracle_client.set_data(
    //     &admin,
    //     &MockAsset::Stellar(usdc),
    //     &Vec::from_array(e, [quote_asset_mock.clone()]),
    //     &7,
    //     &(5 * 60 * 60)
    // );
    // quote_oracle_client.set_price(
    //     &Vec::from_array(e, [quote_oracle_price]),
    //     e.ledger().timestamp()
    // );
}

//   ______    ______    _____  ___  ___________  _______        __       ______  ___________  ________
//  /" _  "\  /    " \  (\"   \|"  \("     _   ")/"      \      /""\     /" _  "\("     _   ")/"       )
// (: ( \___)// ____  \ |.\\   \    |)__/  \\__/|:        |    /    \   (: ( \___))__/  \\__/(:   \___/
//  \/ \    /  /    ) :)|: \.   \\  |   \\_ /   |_____/   )   /' /\  \   \/ \        \\_ /    \___  \
//  //  \ _(: (____/ // |.  \    \. |   |.  |    //      /   //  __'  \  //  \ _     |.  |     __/  \\
// (:   _) \\        /  |    \    \ |   \:  |   |:  __   \  /   /  \\  \(:   _) \    \:  |    /" \   :)
//  \_______)\"_____/    \___|\____\)    \__|   |__|  \___)(___/    \___)\_______)    \__|   (_______/

// was pub(crate)
pub fn create_token_contract<'a>(e: &Env, admin: &Address) -> SorobanTokenClient<'a> {
    SorobanTokenClient::new(e, &e.register_stellar_asset_contract_v2(admin.clone()).address())
}
pub mod pool {
    soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/pool.wasm");
}

pub mod pool_router {
    soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/pool_router.wasm");
}

pub fn deploy_pool_router_contract<'a>(e: Env) -> pool_router::Client<'a> {
    pool_router::Client::new(&e, &e.register(pool_router::WASM, ()))
}

pub mod fee_collector {
    soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/pool_swap_fee.wasm");
}

pub fn deploy_fee_collector_contract<'a>(e: &Env) -> fee_collector::Client<'a> {
    fee_collector::Client::new(e, &e.register(fee_collector::WASM, ()))
}

pub mod oracle_registry {
    soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/oracle_registry.wasm");
}

pub fn deploy_oracle_registry_contract<'a>(e: &Env) -> oracle_registry::Client<'a> {
    oracle_registry::Client::new(e, &e.register(oracle_registry::WASM, ()))
}

pub mod buffer {
    soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/buffer.wasm");
}

pub fn deploy_buffer_contract<'a>(e: &Env) -> buffer::Client<'a> {
    buffer::Client::new(e, &e.register(buffer::WASM, ()))
}

pub mod insurance_fund {
    soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/insurance_fund.wasm");
}

pub fn deploy_insurance_fund_contract<'a>(e: &Env) -> insurance_fund::Client<'a> {
    insurance_fund::Client::new(e, &e.register(insurance_fund::WASM, ()))
}

//   __    __       __        ________  __    __    _______   ________
//  /" |  | "\     /""\      /"       )/" |  | "\  /"     "| /"       )
// (:  (__)  :)   /    \    (:   \___/(:  (__)  :)(: ______)(:   \___/
//  \/      \/   /' /\  \    \___  \   \/      \/  \/    |   \___  \
//  //  __  \\  //  __'  \    __/  \\  //  __  \\  // ___)_   __/  \\
// (:  (  )  :)/   /  \\  \  /" \   :)(:  (  )  :)(:      "| /" \   :)
//  \__|  |__/(___/    \___)(_______/  \__|  |__/  \_______)(_______/

pub fn install_token_wasm(e: &Env) -> BytesN<32> {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/soroban_token_contract.wasm"
    );
    e.deployer().upload_contract_wasm(WASM)
}

pub fn install_liq_pool_hash(e: &Env) -> BytesN<32> {
    soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/pool.wasm");
    e.deployer().upload_contract_wasm(WASM)
}
