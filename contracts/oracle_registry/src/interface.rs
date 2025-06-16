use soroban_sdk::{ Address, Env, Symbol };
use utils::storage::{ MutableOracleInfo, OracleInfo, OraclePriceData };

use crate::storage_types::{ HistoricalOracleData, OracleGuardRails };

pub trait OracleRegistryTrait {
    // Setup the registry
    fn initialize(e: Env, admin: Address, emergency_admin: Address);

    // Get the oracle price
    fn get_price(e: Env, asset_id: Symbol, cached: bool) -> OraclePriceData;

    // Get the historical oracle info
    fn get_last_price(e: Env, asset_id: Symbol) -> HistoricalOracleData;

    // Get the registered oracle info
    fn get_oracle(e: Env, asset_id: Symbol) -> OracleInfo;
}

pub trait AdminInterface {
    // Set oracle guardrails
    fn set_oracle_guardrails(e: Env, admin: Address, oracle_guard_rails: OracleGuardRails);

    // Set price override limit
    fn set_price_override_limit(e: Env, admin: Address, limit: u32);

    // Set price override threshold
    fn set_price_override_threshold(e: Env, admin: Address, threshold: u64);

    // Create a new oracle
    fn register_oracle(
        e: Env,
        admin: Address,
        asset_id: Symbol,
        oracle: Address,
        asset: Address,
        decimals: u32,
        sanitize_clamp_denominator: Option<i64>
    ) -> OracleInfo;

    // Update oracle info
    fn update_oracle(
        e: Env,
        admin: Address,
        asset_id: Symbol,
        params: MutableOracleInfo
    ) -> OracleInfo;

    // Admin failsafe to manually set the oracle price
    fn set_oracle_price(
        e: Env,
        admin: Address,
        asset_id: Symbol,
        oracle_price_twap: u128,
        price: u128
    );

    //   _______    _______  ___________  ___________  _______   _______    ________
    //  /" _   "|  /"     "|("     _   ")("     _   ")/"     "| /"      \  /"       )
    // (: ( \___) (: ______) )__/  \\__/  )__/  \\__/(: ______)|:        |(:   \___/
    //  \/ \       \/    |      \\_ /        \\_ /    \/    |  |_____/   ) \___  \
    //  //  \ ___  // ___)_     |.  |        |.  |    // ___)_  //      /   __/  \\
    // (:   _(  _|(:      "|    \:  |        \:  |   (:      "||:  __   \  /" \   :)
    //  \_______)  \_______)     \__|         \__|    \_______)|__|  \___)(_______/

    fn get_oracle_guardrails(e: Env) -> OracleGuardRails;

    fn get_price_override_limit(e: Env) -> u32;

    fn get_price_override_threshold(e: Env) -> u64;
}
