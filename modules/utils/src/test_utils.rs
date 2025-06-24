#![cfg(any(test, feature = "testutils"))]

use soroban_sdk::testutils::{Ledger, LedgerInfo};
use soroban_sdk::token::{
    StellarAssetClient as SorobanTokenAdminClient, TokenClient as SorobanTokenClient,
};
use soroban_sdk::{Address, BytesN, Env, String, Symbol, U256};

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

pub fn get_mock_assets<'a>(e: &Env) -> (Symbol, Symbol) {
    (Symbol::new(e, "BTC"), Symbol::new(e, "XLM"))
}

pub fn get_mock_lp_token_info<'a>(e: &Env) -> (String, String) {
    (
        String::from_str(e, "Pool Share Token"),
        String::from_str(e, "Pool Share Token"),
    )
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

//   ______    ______    _____  ___  ___________  _______        __       ______  ___________  ________
//  /" _  "\  /    " \  (\"   \|"  \("     _   ")/"      \      /""\     /" _  "\("     _   ")/"       )
// (: ( \___)// ____  \ |.\\   \    |)__/  \\__/|:        |    /    \   (: ( \___))__/  \\__/(:   \___/
//  \/ \    /  /    ) :)|: \.   \\  |   \\_ /   |_____/   )   /' /\  \   \/ \        \\_ /    \___  \
//  //  \ _(: (____/ // |.  \    \. |   |.  |    //      /   //  __'  \  //  \ _     |.  |     __/  \\
// (:   _) \\        /  |    \    \ |   \:  |   |:  __   \  /   /  \\  \(:   _) \    \:  |    /" \   :)
//  \_______)\"_____/    \___|\____\)    \__|   |__|  \___)(___/    \___)\_______)    \__|   (_______/

// was pub(crate)
pub fn create_token_contract<'a>(e: &Env, admin: &Address) -> SorobanTokenClient<'a> {
    SorobanTokenClient::new(
        e,
        &e.register_stellar_asset_contract_v2(admin.clone())
            .address(),
    )
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
