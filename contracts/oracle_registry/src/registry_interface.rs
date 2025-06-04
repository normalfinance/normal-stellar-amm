use soroban_sdk::{ Address, Env };
use utils::{ oracle::{ OracleGuardRails, OraclePriceData, OracleSource }, storage::AssetId };

pub trait OracleRegistryTrait {
    //
    fn get_oracle_price(e: Env, user: Address, asset_id: AssetId, cached: bool) -> OraclePriceData;
}

pub trait IndexOracleTrait {
    //
    fn create_index(e: Env, user: Address);

    fn update_index(e: Env, user: Address, asset_id: AssetId);
}

pub trait AdminInterface {
    // Initialize admin user. Will panic if called twice
    fn init_admin(e: Env, account: Address);

    //
    fn set_oracle_guardrails(e: Env, admin: Address, oracle_guard_rails: OracleGuardRails);

    fn register_oracle(
        e: Env,
        admin: Address,
        asset_id: AssetId,
        oracle_address: Address,
        source: OracleSource
    );

    // Failsafe to update an oracle
    fn update_oracle(
        e: Env,
        admin: Address,
        asset_id: AssetId,
        oracle_address: Address,
        oracle_source: OracleSource
    );

    fn unregister_oracle(e: Env, admin: Address, asset_id: AssetId);

    //
    fn pull_oracle_price(e: Env, admin: Address, asset_id: AssetId);

    // Admin failsafe
    fn set_oracle_price(e: Env, admin: Address, asset_id: AssetId, price: u128);

    //
    fn freeze_oracle(e: Env, admin: Address, asset_id: AssetId);

    //
    fn unfreeze_oracle(e: Env, admin: Address, asset_id: AssetId);
}
