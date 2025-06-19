use crate::errors::OracleRegistryError;
use crate::events::{ Events, OracleRegistryEvents };
use crate::interface::{ AdminInterface, OracleRegistryTrait };
use crate::oracle::{ get_oracle_price, oracle_validity, update_twap };
use soroban_fixed_point_math::FixedPoint;
use crate::storage::{
    get_historical_oracle_data,
    get_oracle,
    get_oracle_base,
    get_oracle_guard_rails,
    get_price_override_limit,
    get_price_override_threshold,
    put_oracle,
    set_oracle_guard_rails,
    set_price_override_limit,
    set_price_override_threshold,
};
use crate::storage_types::{ HistoricalOracleData, OracleGuardRails, OracleValidity };

use access_control::access::{ AccessControl, AccessControlTrait };
use access_control::emergency::{ get_emergency_mode, set_emergency_mode };
use access_control::errors::AccessControlError;
use access_control::events::Events as AccessControlEvents;
use access_control::interface::TransferableContract;
use access_control::management::{ MultipleAddressesManagementTrait, SingleAddressManagementTrait };
use access_control::role::Role;
use access_control::role::SymbolRepresentation;
use access_control::transfer::TransferOwnershipTrait;
use access_control::utils::require_admin;
use soroban_sdk::{ contract, contractimpl, panic_with_error, Address, BytesN, Env, Symbol, Vec };
use upgrade::events::Events as UpgradeEvents;
use upgrade::interface::UpgradeableContract;
use upgrade::{ apply_upgrade, commit_upgrade, revert_upgrade };
use utils::constant::{ PRICE_PRECISION };
use utils::state::oracle_registry::{ MutableOracleInfo, OracleInfo, OraclePriceData };

#[contract]
pub struct OracleRegistry;

// The `OracleRegistryTrait` trait provides the interface for interacting with a liquidity pool.
#[contractimpl]
impl OracleRegistryTrait for OracleRegistry {
    fn initialize(e: Env, admin: Address, emergency_admin: Address) {
        admin.require_auth();

        let access_control = AccessControl::new(&e);
        if access_control.get_role_safe(&Role::Admin).is_some() {
            panic_with_error!(&e, AccessControlError::AdminAlreadySet);
        }
        access_control.set_role_address(&Role::Admin, &admin);
        access_control.set_role_address(&Role::EmergencyAdmin, &emergency_admin);
    }

    fn get_price(e: Env, asset_id: Symbol, cached: bool) -> OraclePriceData {
        let now = e.ledger().timestamp();
        let oracle = get_oracle(&e, &asset_id);
        let historical_oracle_data = get_historical_oracle_data(&e, &asset_id);

        if cached || oracle.frozen {
            return OraclePriceData {
                price: historical_oracle_data.last_oracle_price_twap,
                delay: historical_oracle_data.last_oracle_delay,
            };
        }

        // Fetch a new price
        let oracle_price_data = get_oracle_price(&e, &oracle.address, &oracle.asset, now);

        let oracle_is_valid =
            oracle_validity(
                &e,
                historical_oracle_data.last_oracle_price_twap,
                &oracle_price_data
            ) == OracleValidity::Valid;

        if !oracle_is_valid {
            panic_with_error!(&e, OracleRegistryError::OracleInvalid);
        }

        update_twap(
            &e,
            &asset_id,
            &historical_oracle_data,
            &oracle_price_data,
            oracle.sanitize_clamp_denominator,
            now,
            false
        );

        oracle_price_data
    }

    fn get_last_price(e: Env, asset_id: Symbol) -> HistoricalOracleData {
        get_historical_oracle_data(&e, &asset_id)
    }

    fn get_oracle(e: Env, asset_id: Symbol) -> OracleInfo {
        get_oracle(&e, &asset_id)
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
    // Sets the oracle guard rails.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    // * `oracle_guard_rails` - The new oracle guard rails.
    fn set_oracle_guardrails(e: Env, admin: Address, oracle_guard_rails: OracleGuardRails) {
        admin.require_auth();
        require_admin(&e, &admin);

        set_oracle_guard_rails(&e, &oracle_guard_rails);
    }

    // Sets the oracle price override limit.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    // * `limit` - The new price limit.
    fn set_price_override_limit(e: Env, admin: Address, limit: u32) {
        admin.require_auth();
        require_admin(&e, &admin);

        set_price_override_limit(&e, &limit);
    }

    // Sets the oracle price override threshold.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    // * `threshold` - The new price threshold.
    fn set_price_override_threshold(e: Env, admin: Address, threshold: u64) {
        admin.require_auth();
        require_admin(&e, &admin);

        set_price_override_threshold(&e, &threshold);
    }

    fn get_oracle_guardrails(e: Env) -> OracleGuardRails {
        get_oracle_guard_rails(&e)
    }

    fn get_price_override_limit(e: Env) -> u32 {
        get_price_override_limit(&e)
    }

    fn get_price_override_threshold(e: Env) -> u64 {
        get_price_override_threshold(&e)
    }

    fn register_oracle(
        e: Env,
        admin: Address,
        asset_id: Symbol,
        oracle_addr: Address,
        asset: Address,
        decimals: u32,
        sanitize_clamp_denominator: i64
    ) -> OracleInfo {
        admin.require_auth();
        require_admin(&e, &admin);

        if get_oracle_base(&e, &asset_id).is_some() {
            panic_with_error!(&e, OracleRegistryError::OracleAlreadyRegistered);
        }

        let now = e.ledger().timestamp();
        let oracle_price_data = get_oracle_price(&e, &oracle_addr, &asset, now);

        // Check oracle validity
        let oracle_is_valid =
            oracle_validity(&e, oracle_price_data.price, &oracle_price_data) ==
            OracleValidity::Valid;

        if !oracle_is_valid {
            panic_with_error!(&e, OracleRegistryError::OracleInvalid);
        }

        update_twap(
            &e,
            &asset_id,
            &get_historical_oracle_data(&e, &asset_id),
            &oracle_price_data,
            sanitize_clamp_denominator,
            now,
            true
        );

        let oracle = OracleInfo {
            address: oracle_addr,
            asset,
            decimals,
            frozen: false,
            sanitize_clamp_denominator,
            last_updated: now,
        };
        put_oracle(&e, &asset_id, &oracle);

        oracle
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
        asset_id: Symbol,
        params: MutableOracleInfo
    ) -> OracleInfo {
        admin.require_auth();
        require_admin(&e, &admin);

        if let Some(oracle) = get_oracle_base(&e, &asset_id) {
            let now = e.ledger().timestamp();

            // Address validation
            if let Some(oracle_addr) = params.address.clone() {
                let oracle_price_data = get_oracle_price(&e, &oracle_addr, &oracle.asset, now);

                // Check oracle validity
                let historical_oracle_data = get_historical_oracle_data(&e, &asset_id);
                let oracle_is_valid =
                    oracle_validity(
                        &e,
                        historical_oracle_data.last_oracle_price_twap,
                        &oracle_price_data
                    ) == OracleValidity::Valid;

                if !oracle_is_valid {
                    panic_with_error!(&e, OracleRegistryError::OracleInvalid);
                }
            }

            // Decimal validation
            if let Some(decimals) = params.decimals {
                if decimals > 18 {
                    panic_with_error!(&e, OracleRegistryError::AdminNotSet);
                }
            }

            let updated_oracle = OracleInfo {
                address: params.address.unwrap_or(oracle.address),
                decimals: params.decimals.unwrap_or(oracle.decimals),
                sanitize_clamp_denominator: params.sanitize_clamp_denominator.unwrap_or(
                    oracle.sanitize_clamp_denominator
                ),
                frozen: params.frozen.unwrap_or(oracle.frozen),
                last_updated: now,
                ..oracle
            };
            put_oracle(&e, &asset_id, &updated_oracle);

            updated_oracle
        } else {
            panic_with_error!(&e, OracleRegistryError::OracleNotRegistered);
        }
    }

    // Sets the oracle price manually.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    // * `asset_id` - The address of the rewards admin.
    // * `price` - The address of the rewards admin.
    fn set_oracle_price(e: Env, admin: Address, asset_id: Symbol, price: u128) {
        admin.require_auth();
        require_admin(&e, &admin);

        let now = e.ledger().timestamp();
        let oracle = get_oracle(&e, &asset_id);

        // Rate limit
        let override_threshold = get_price_override_threshold(&e);
        // @dev The timestamp of the last override is not tracked, meaning any
        // update to the oracle will reset this counter. May be changed in the future.
        if now - oracle.last_updated >= override_threshold {
            panic_with_error!(&e, OracleRegistryError::PriceOverrideTooSoon);
        }

        // Smooth price updates
        let override_limit = get_price_override_limit(&e);
        let historical_oracle_data = get_historical_oracle_data(&e, &asset_id);
        let price_delta = historical_oracle_data.last_oracle_price_twap
            .fixed_div_floor(price, PRICE_PRECISION)
            .unwrap() as i32;
        if (price_delta.abs() as u32) >= override_limit {
            panic_with_error!(&e, OracleRegistryError::PriceOverrideLimitExceeded);
        }

        // Update the price and twap
        update_twap(
            &e,
            &asset_id,
            &historical_oracle_data,
            &(OraclePriceData { price: price, delay: 0 }),
            oracle.sanitize_clamp_denominator,
            now,
            false
        );
    }

    // TODO: unregister oracle
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
