use crate::errors::OracleRegistryError;
use crate::events::{ Events, OracleRegistryEvents };
use crate::oracle::{ get_oracle_price, update_twap };
use crate::registry_interface::{ AdminInterface, OracleRegistryTrait };
use crate::storage::{
    get_historical_oracle_data,
    get_oracle,
    get_price_override_limit,
    put_historical_oracle_data,
    put_oracle_guard_rails,
    put_oracle,
    HistoricalOracleData,
};

use access_control::access::{ AccessControl, AccessControlTrait };
use access_control::emergency::{ get_emergency_mode, set_emergency_mode };
use access_control::errors::AccessControlError;
use access_control::events::Events as AccessControlEvents;
use access_control::interface::TransferableContract;
use access_control::management::{ MultipleAddressesManagementTrait, SingleAddressManagementTrait };
use access_control::role::Role;
use access_control::role::SymbolRepresentation;
use access_control::transfer::TransferOwnershipTrait;
use soroban_sdk::{ contract, contractimpl, panic_with_error, Address, BytesN, Env, Symbol, Vec };
use upgrade::events::Events as UpgradeEvents;
use upgrade::interface::UpgradeableContract;
use upgrade::{ apply_upgrade, commit_upgrade, revert_upgrade };
use utils::oracle::{ OracleGuardRails, OraclePriceData };
use utils::storage::{ AssetId, OracleInfo };

#[contract]
pub struct OracleRegistry;

// The `OracleRegistryTrait` trait provides the interface for interacting with a liquidity pool.
#[contractimpl]
impl OracleRegistryTrait for OracleRegistry {
    fn get_price(
        e: Env,
        user: Address,
        asset_id: AssetId,
        cached: bool,
        sanitize_clamp_denominator: Option<i64>
    ) -> OraclePriceData {
        user.require_auth();

        let now = e.ledger().timestamp();
        let oracle_info = get_oracle(&e, asset_id.clone());

        if cached || oracle_info.frozen {
            let historical_oracle_data = get_historical_oracle_data(&e, asset_id);
            return OraclePriceData {
                price: historical_oracle_data.last_oracle_price_twap,
                delay: historical_oracle_data.last_oracle_delay,
            };
        }

        // Fetch a new price
        let oracle_price_data = get_oracle_price(
            &e,
            &oracle_info.oracle_address,
            &oracle_info.asset,
            now
        );

        // Update data
        update_twap(&e, asset_id, &oracle_price_data, sanitize_clamp_denominator, now);

        oracle_price_data
    }
}

// The `UpgradeableContract` trait provides the interface for upgrading the contract.
#[contractimpl]
impl UpgradeableContract for OracleRegistry {
    // Returns the version of the contract.
    //
    // # Returns
    //
    // The version of the contract as a u32.
    fn version() -> u32 {
        150
    }

    // Commits a new wasm hash for a future upgrade.
    // The upgrade will be available through `apply_upgrade` after the standard upgrade delay
    // unless the system is in emergency mode.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    // * `new_wasm_hash` - The new wasm hash to commit.
    fn commit_upgrade(e: Env, admin: Address, new_wasm_hash: BytesN<32>) {
        admin.require_auth();
        AccessControl::new(&e).assert_address_has_role(&admin, &Role::Admin);
        commit_upgrade(&e, &new_wasm_hash);
        UpgradeEvents::new(&e).commit_upgrade(Vec::from_array(&e, [new_wasm_hash.clone()]));
    }

    // Applies the committed upgrade.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    fn apply_upgrade(e: Env, admin: Address) -> BytesN<32> {
        admin.require_auth();
        AccessControl::new(&e).assert_address_has_role(&admin, &Role::Admin);
        let new_wasm_hash = apply_upgrade(&e);
        UpgradeEvents::new(&e).apply_upgrade(Vec::from_array(&e, [new_wasm_hash.clone()]));
        new_wasm_hash
    }

    // Reverts the committed upgrade.
    // This can be used to cancel a previously committed upgrade.
    // The upgrade will be canceled only if it has not been applied yet.
    // If the upgrade has already been applied, it cannot be reverted.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    fn revert_upgrade(e: Env, admin: Address) {
        admin.require_auth();
        AccessControl::new(&e).assert_address_has_role(&admin, &Role::Admin);
        revert_upgrade(&e);
        UpgradeEvents::new(&e).revert_upgrade();
    }

    // Sets the emergency mode.
    // When the emergency mode is set to true, the contract will allow instant upgrades without the delay.
    // This is useful in case of critical issues that need to be fixed immediately.
    // When the emergency mode is set to false, the contract will require the standard upgrade delay.
    // The emergency mode can only be set by the emergency admin.
    //
    // # Arguments
    //
    // * `emergency_admin` - The address of the emergency admin.
    // * `value` - The value to set the emergency mode to.
    fn set_emergency_mode(e: Env, emergency_admin: Address, value: bool) {
        emergency_admin.require_auth();
        AccessControl::new(&e).assert_address_has_role(&emergency_admin, &Role::EmergencyAdmin);
        set_emergency_mode(&e, &value);
        AccessControlEvents::new(&e).set_emergency_mode(value);
    }

    // Returns the emergency mode flag value.
    fn get_emergency_mode(e: Env) -> bool {
        get_emergency_mode(&e)
    }
}

// The `AdminInterface` trait provides the interface for administrative actions.
#[contractimpl]
impl AdminInterface for OracleRegistry {
    // Initializes the admin user.
    //
    // # Arguments
    //
    // * `account` - The address of the admin user.
    fn init_admin(e: Env, account: Address) {
        let access_control = AccessControl::new(&e);
        if access_control.get_role_safe(&Role::Admin).is_some() {
            panic_with_error!(&e, AccessControlError::AdminAlreadySet);
        }
        access_control.set_role_address(&Role::Admin, &account);
    }

    // Sets the oracle guard rails.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    // * `oracle_guard_rails` - The new oracle guard rails.
    fn set_oracle_guardrails(e: Env, admin: Address, oracle_guard_rails: OracleGuardRails) {
        admin.require_auth();
        AccessControl::new(&e).assert_address_has_role(&admin, &Role::Admin);

        put_oracle_guard_rails(&e, &oracle_guard_rails);
    }

    fn register_oracle(
        e: Env,
        admin: Address,
        asset_id: AssetId,
        oracle: Address,
        asset: Address,
        decimals: u32
    ) {
        admin.require_auth();

        // if has_oracle(&e, asset_id.clone()) {
        //     panic_with_error!(&e, OracleRegistryError::OracleExists);
        // }

        let oracle_info = OracleInfo {
            oracle_address: oracle,
            asset,
            decimals,
            frozen: false,
            last_updated: e.ledger().timestamp(),
        };
        put_oracle(&e, asset_id, &oracle_info);
    }

    // Sets the oracle guard rails.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    // * `oracle_guard_rails` - The address of the rewards admin.
    fn set_oracle_address(e: Env, admin: Address, asset_id: AssetId, address: Address) {
        admin.require_auth();

        let oracle_info = get_oracle(&e, asset_id.clone());

        put_oracle(
            &e,
            asset_id,
            &(OracleInfo {
                oracle_address: address,
                last_updated: e.ledger().timestamp(),
                ..oracle_info
            })
        );
    }

    // Sets the oracle guard rails.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    // * `oracle_guard_rails` - The address of the rewards admin.
    fn set_oracle_decimals(e: Env, admin: Address, asset_id: AssetId, decimals: u32) {
        admin.require_auth();

        let oracle_info = get_oracle(&e, asset_id.clone());

        put_oracle(
            &e,
            asset_id,
            &(OracleInfo {
                decimals,
                last_updated: e.ledger().timestamp(),
                ..oracle_info
            })
        );
    }

    // Sets the oracle guard rails.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    // * `asset_id` - The address of the rewards admin.
    fn sync_oracle_price(
        e: Env,
        admin: Address,
        asset_id: AssetId,
        sanitize_clamp_denominator: Option<i64>
    ) {
        admin.require_auth();

        let now = e.ledger().timestamp();
        let oracle_info = get_oracle(&e, asset_id.clone());

        // Pull latest price
        let oracle_price_data = get_oracle_price(
            &e,
            &oracle_info.oracle_address,
            &oracle_info.asset,
            now
        );

        update_twap(&e, asset_id, &oracle_price_data, sanitize_clamp_denominator, now);
    }

    // Sets the oracle price manually.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    // * `asset_id` - The address of the rewards admin.
    // * `price` - The address of the rewards admin.
    fn set_oracle_price(
        e: Env,
        admin: Address,
        asset_id: AssetId,
        oracle_price_twap: u128,
        price: u128
    ) {
        admin.require_auth();

        let now = e.ledger().timestamp();

        // let oracle_info = get_oracle(&e, &asset_id);
        let historical_oracle_data = get_historical_oracle_data(&e, asset_id.clone());

        if historical_oracle_data.last_oracle_price_twap / price > get_price_override_limit(&e) {
            panic_with_error!(&e, OracleRegistryError::PriceOverrideLimitExceeded);
        }

        let new_historical_oracle_data = HistoricalOracleData {
            last_oracle_price_twap: oracle_price_twap,
            last_oracle_price: price,
            last_oracle_delay: 0,
            last_oracle_price_twap_ts: now,
        };
        put_historical_oracle_data(&e, asset_id, &new_historical_oracle_data);
    }

    // Sets the oracle status to frozen.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    // * `asset_id` - The oracle to freeze.
    fn freeze_oracle(e: Env, admin: Address, asset_id: AssetId) {
        admin.require_auth();

        let oracle_info = get_oracle(&e, asset_id.clone());

        put_oracle(
            &e,
            asset_id,
            &(OracleInfo {
                frozen: true,
                last_updated: e.ledger().timestamp(),
                ..oracle_info
            })
        );
    }

    // Sets the oracle status to unfrozen.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    // * `asset_id` - The oracle to unfreeze.
    fn unfreeze_oracle(e: Env, admin: Address, asset_id: AssetId) {
        admin.require_auth();

        let oracle_info = get_oracle(&e, asset_id.clone());

        put_oracle(
            &e,
            asset_id,
            &(OracleInfo {
                frozen: false,
                last_updated: e.ledger().timestamp(),
                ..oracle_info
            })
        );
    }
}

// The `TransferableContract` trait provides the interface for transferring ownership of the contract.
#[contractimpl]
impl TransferableContract for OracleRegistry {
    // Commits an ownership transfer.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    // * `role_name` - The name of the role to transfer ownership of. The role must be one of the following:
    //     * `Admin`
    //     * `EmergencyAdmin`
    // * `new_address` - New address for the role
    fn commit_transfer_ownership(e: Env, admin: Address, role_name: Symbol, new_address: Address) {
        admin.require_auth();
        let access_control = AccessControl::new(&e);
        access_control.assert_address_has_role(&admin, &Role::Admin);

        let role = Role::from_symbol(&e, role_name);
        access_control.commit_transfer_ownership(&role, &new_address);
        AccessControlEvents::new(&e).commit_transfer_ownership(role, new_address);
    }

    // Applies the committed ownership transfer.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    // * `role_name` - The name of the role to transfer ownership of. The role must be one of the following:
    //     * `Admin`
    //     * `EmergencyAdmin`
    fn apply_transfer_ownership(e: Env, admin: Address, role_name: Symbol) {
        admin.require_auth();
        let access_control = AccessControl::new(&e);
        access_control.assert_address_has_role(&admin, &Role::Admin);

        let role = Role::from_symbol(&e, role_name);
        let new_address = access_control.apply_transfer_ownership(&role);
        AccessControlEvents::new(&e).apply_transfer_ownership(role, new_address);
    }

    // Reverts the committed ownership transfer.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    // * `role_name` - The name of the role to transfer ownership of. The role must be one of the following:
    //     * `Admin`
    //     * `EmergencyAdmin`
    fn revert_transfer_ownership(e: Env, admin: Address, role_name: Symbol) {
        admin.require_auth();
        let access_control = AccessControl::new(&e);
        access_control.assert_address_has_role(&admin, &Role::Admin);

        let role = Role::from_symbol(&e, role_name);
        access_control.revert_transfer_ownership(&role);
        AccessControlEvents::new(&e).revert_transfer_ownership(role);
    }

    // Returns the future address for the role.
    // The future address is the address that the ownership of the role will be transferred to.
    // The future address is set using the `commit_transfer_ownership` function.
    // The address will be defaulted to the current address if the transfer is not committed.
    //
    // # Arguments
    //
    // * `role_name` - The name of the role to get the future address for. The role must be one of the following:
    //    * `Admin`
    //    * `EmergencyAdmin`
    fn get_future_address(e: Env, role_name: Symbol) -> Address {
        let access_control = AccessControl::new(&e);
        let role = Role::from_symbol(&e, role_name);
        match access_control.get_transfer_ownership_deadline(&role) {
            0 =>
                match access_control.get_role_safe(&role) {
                    Some(address) => address,
                    None => panic_with_error!(&e, AccessControlError::RoleNotFound),
                }
            _ => access_control.get_future_address(&role),
        }
    }
}
