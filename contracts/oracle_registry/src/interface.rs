use soroban_sdk::{ Address, Env, Symbol };
use utils::storage::OraclePriceData;

use crate::storage_types::OracleGuardRails;

pub trait OracleRegistryTrait {
    // Get the oracle price
    fn get_price(
        e: Env,
        sender: Address,
        asset_id: Symbol,
        cached: bool,
        sanitize_clamp_denominator: Option<i64>
    ) -> OraclePriceData;
}

pub trait AdminInterface {
    // Initialize admin user. Will panic if called twice
    fn init_admin(e: Env, account: Address);

    // Set oracle guardrails
    fn set_oracle_guardrails(e: Env, admin: Address, oracle_guard_rails: OracleGuardRails);

    // Set pric override limit
    fn set_price_override_limit(e: Env, admin: Address, limit: u128);

    // Create a new oracle
    fn register_oracle(
        e: Env,
        admin: Address,
        asset_id: Symbol,
        oracle: Address,
        asset: Address,
        decimals: u32
    );

    // Set oracle address
    fn set_oracle_address(e: Env, admin: Address, asset_id: Symbol, oracle: Address);

    // Set oracle decimals
    fn set_oracle_decimals(e: Env, admin: Address, asset_id: Symbol, decimals: u32);

    // Sync the oracle price
    fn sync_oracle_price(
        e: Env,
        admin: Address,
        asset_id: Symbol,
        sanitize_clamp_denominator: Option<i64>
    );

    // Admin failsafe to manually set the oracle price
    fn set_oracle_price(
        e: Env,
        admin: Address,
        asset_id: Symbol,
        oracle_price_twap: u128,
        price: u128
    );

    // Pause price updates
    fn freeze_oracle(e: Env, admin: Address, asset_id: Symbol);

    // Unpause price updates
    fn unfreeze_oracle(e: Env, admin: Address, asset_id: Symbol);
}
