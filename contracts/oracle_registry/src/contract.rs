use crate::errors::{ OracleRegistryError };
use crate::events::{ OracleRegistryEvents, Events as OracleRegistryEvents };
use crate::oracle::get_oracle_price;
use crate::registry_interface::{ AdminInterface, IndexOracleTrait, OracleRegistryTrait };
use crate::storage::{
    get_historical_oracle_data,
    get_oracle,
    get_price_override_limit,
    has_oracle,
    put_oracle_guard_rails,
    put_oracles_set,
    set_oracle,
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
use access_control::utils::{
    require_pause_admin_or_owner,
    require_pause_or_emergency_pause_admin_or_owner,
};
use soroban_sdk::token::TokenClient as SorobanTokenClient;
use soroban_sdk::{
    contract,
    contractimpl,
    log,
    panic_with_error,
    Address,
    BytesN,
    Env,
    Map,
    Symbol,
    Vec,
};
use upgrade::events::Events as UpgradeEvents;
use upgrade::interface::UpgradeableContract;
use upgrade::{ apply_upgrade, commit_upgrade, revert_upgrade };
use utils::oracle::{ OracleGuardRails, OraclePriceData, OracleSource };
use utils::storage::{ AssetId, OracleInfo };

#[contract]
pub struct OracleRegistry;

// The `OracleRegistryTrait` trait provides the interface for interacting with a liquidity pool.
#[contractimpl]
impl OracleRegistryTrait for OracleRegistry {
    fn get_oracle_price(e: Env, user: Address, asset_id: AssetId, cached: bool) -> OraclePriceData {
        user.require_auth();

        let now = e.ledger().timestamp();
        let oracle_info = get_oracle(&e, &asset_id);

        if oracle_info.frozen {
            panic_with_error!(&e, OracleRegistryError::OracleFrozen);
        }

        let oracle_price_data: OraclePriceData;

        if cached {
            oracle_price_data = get_historical_oracle_data(&e, &asset_id);
        } else {
            // Fetch a new price
            oracle_price_data = get_oracle_price(
                &e,
                oracle_info.source,
                oracle_info.oracle_address,
                asset,
                now
            );

            // update data
            // ...
        }

        oracle_price_data
    }
}

// The `IndexOracleTrait` trait provides the interface for interacting with an index token.
#[contractimpl]
impl IndexOracleTrait for OracleRegistry {
    fn create_index(e: Env, user: Address) {
        user.require_auth();

        // ...
    }

    fn update_index(e: Env, user: Address, asset_id: AssetId) {
        user.require_auth();

        // ...
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
        oracle_address: Address,
        source: OracleSource
    ) {
        admin.require_auth();

        if has_oracle(&e, asset_id) {
            panic_with_error!(&e, OracleRegistryError::OracleExists);
        }

        let oracle_info = OracleInfo {
            oracle_address,
            source,
            decimals: 7,
            frozen: false,
            last_updated: e.ledger().timestamp(),
        };
        set_oracle(&e, asset_id, &oracle_info);
    }

    // Sets the oracle guard rails.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    // * `oracle_guard_rails` - The address of the rewards admin.
    fn update_oracle(
        e: Env,
        admin: Address,
        asset_id: AssetId,
        oracle_address: Address,
        source: OracleSource
    ) {
        admin.require_auth();

        let oracle_info = get_oracle(&e, &asset_id);

        oracle_info.oracle_address = oracle_address;
        oracle_info.source = source;

        put_oracle_data(&e, data);
    }

    fn unregister_oracle(e: Env, admin: Address, asset_id: AssetId) {}

    // Sets the oracle guard rails.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    // * `oracle_guard_rails` - The address of the rewards admin.
    fn pull_oracle_price(e: Env, admin: Address, asset_id: AssetId) {
        admin.require_auth();

        let now = e.ledger().timestamp();
        let oracle_info = get_oracle(&e, &asset_id);

        // Pull latest price
        let oracle_price_data = get_oracle_price(
            &e,
            oracle_info.source,
            oracle_info.oracle_address,
            asset,
            now
        );

        put_historical_oracle_data(&e, asset_id, data);

        set_oracle(&e, asset_id, OracleInfo);
    }

    // Sets the oracle price manually.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    // * `asset_id` - The address of the rewards admin.
    // * `price` - The address of the rewards admin.
    fn set_oracle_price(e: Env, admin: Address, asset_id: AssetId, price: u128) {
        admin.require_auth();

        // let oracle_info = get_oracle(&e, &asset_id);
        let historical_oracle_data = get_historical_oracle_data(&e, &asset_id);

        if historical_oracle_data.last_oracle_price_twap / price > get_price_override_limit(&e) {
            panic_with_error!(&e, OracleRegistryError::PriceOverrideLimitExceeded);
        }

        // TODO: update price
    }

    // Sets the oracle status to frozen.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    // * `asset_id` - The oracle to freeze.
    fn freeze_oracle(e: Env, admin: Address, asset_id: AssetId) {
        admin.require_auth();

        let oracle_info = get_oracle(&e, &asset_id);

        oracle_info.frozen = true;

        put_oracle_data(&e, oracle_info);
    }

    // Sets the oracle status to unfrozen.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    // * `asset_id` - The oracle to unfreeze.
    fn unfreeze_oracle(e: Env, admin: Address, asset_id: AssetId) {
        admin.require_auth();

        let oracle_info = get_oracle(&e, &asset_id);

        oracle_info.frozen = false;

        put_oracle_data(&e, oracle_info);
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
