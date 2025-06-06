use soroban_sdk::{ Address, Env };
use utils::{ oracle::{ OracleGuardRails, OraclePriceData }, storage::AssetId };

pub trait OracleRegistryTrait {
    //
    fn get_price(
        e: Env,
        user: Address,
        asset_id: AssetId,
        cached: bool,
        sanitize_clamp_denominator: Option<i64>
    ) -> OraclePriceData;
}

pub trait AdminInterface {
    // Initialize admin user. Will panic if called twice
    fn init_admin(e: Env, account: Address);

    // Set oracle guardrails
    fn set_oracle_guardrails(e: Env, admin: Address, oracle_guard_rails: OracleGuardRails);

    // Create a new oracle
    fn register_oracle(
        e: Env,
        admin: Address,
        asset_id: AssetId,
        oracle: Address,
        asset: Address,
        decimals: u32
    );

    // Set oracle address
    fn set_oracle_address(e: Env, admin: Address, asset_id: AssetId, oracle: Address);

    // Set oracle decimals
    fn set_oracle_decimals(e: Env, admin: Address, asset_id: AssetId, decimals: u32);

    // Sync the oracle price
    fn sync_oracle_price(
        e: Env,
        admin: Address,
        asset_id: AssetId,
        sanitize_clamp_denominator: Option<i64>
    );

    // Admin failsafe to manually set the oracle price
    fn set_oracle_price(
        e: Env,
        admin: Address,
        asset_id: AssetId,
        oracle_price_twap: u128,
        price: u128
    );

    // Pause price updates
    fn freeze_oracle(e: Env, admin: Address, asset_id: AssetId);

    // Unpause price updates
    fn unfreeze_oracle(e: Env, admin: Address, asset_id: AssetId);
}
