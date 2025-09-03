use crate::errors::OracleRegistryError;
use crate::interface::{AdminInterface, OracleRegistryTrait};
use crate::oracle::{get_oracle_price, oracle_validity, update_twap};
use crate::storage::{
    get_historical_oracle_data, get_oracle, get_oracle_base, get_oracle_guard_rails, put_oracle,
    remove_oracle, set_oracle_guard_rails,
};
use access_control::access::{AccessControl, AccessControlTrait};
use access_control::emergency::{get_emergency_mode, set_emergency_mode};
use access_control::errors::AccessControlError;
use access_control::events::Events as AccessControlEvents;
use access_control::interface::TransferableContract;
use access_control::management::{MultipleAddressesManagementTrait, SingleAddressManagementTrait};
use access_control::role::Role;
use access_control::role::SymbolRepresentation;
use access_control::transfer::TransferOwnershipTrait;
use access_control::utils::require_admin;
use reentrancy_guard::{enter, exit};
use soroban_sdk::{contract, contractimpl, panic_with_error, Address, BytesN, Env, Symbol, Vec};
use upgrade::events::Events as UpgradeEvents;
use upgrade::interface::UpgradeableContract;
use upgrade::{apply_upgrade, commit_upgrade, revert_upgrade};
use utils::state::oracle_registry::{
    HistoricalOracleData, MutableOracleInfo, OracleGuardRails, OracleInfo, OraclePriceData,
    OracleValidity,
};
use utils::temporal::Delay;
use utils::validation::{ensure_non_zero_u128, validate_positive_denominator};

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

    //  ___      ___       __        __    _____  ___
    // |"  \    /"  |     /""\      |" \  (\"   \|"  \
    //  \   \  //   |    /    \     ||  | |.\\   \    |
    //  /\\  \/.    |   /' /\  \    |:  | |: \.   \\  |
    // |: \.        |  //  __'  \   |.  | |.  \    \. |
    // |.  \    /:  | /   /  \\  \  /\  |\|    \    \ |
    // |___|\__/|___|(___/    \___)(__\_|_)\___|\____\)

    // Retrieves the current oracle price for a given asset.
    //
    // If `cached` is true or the oracle is frozen, it returns the last cached TWAP price.
    // Otherwise, it fetches a fresh price from the oracle, validates it, updates the TWAP,
    // and returns the latest data. If the fetched data fails safety checks, the function
    // will panic with an error.
    //
    // # Arguments
    // * `e` - The Soroban environment.
    // * `asset` - The asset symbol (e.g., "BTC") whose price is being requested.
    //
    // # Returns
    // * `HistoricalOracleData` -  The historical oracle price information.
    // * `OracleValidity` - The oracle validity.
    //
    // # Panics
    // Panics with `OracleRegistryError::OracleInvalid` if the live price data fails validation.
    fn get_price(e: Env, asset: Symbol) -> (HistoricalOracleData, OracleValidity) {
        let now = e.ledger().timestamp();
        let oracle = get_oracle(&e, &asset);

        let historical_oracle_data = get_historical_oracle_data(&e, &asset);

        if oracle.frozen {
            return (historical_oracle_data, OracleValidity::Frozen);
        }

        let oracle_price_data = get_oracle_price(&e, &oracle.address, &asset, now);

        let oracle_validity = oracle_validity(
            &e,
            historical_oracle_data.last_oracle_price_twap,
            &oracle_price_data,
        );

        if oracle_validity != OracleValidity::Frozen {
            update_twap(
                &e,
                &asset,
                &historical_oracle_data,
                &oracle_price_data,
                oracle.sanitize_clamp_denominator,
                now,
            );
        }

        let new_historical_oracle_data = get_historical_oracle_data(&e, &asset);

        (new_historical_oracle_data, oracle_validity)
    }

    // Gets the last historical price of the oracle.
    //
    // # Arguments
    //
    // * `asset` - The symbol of the oracle.
    fn get_last_price(e: Env, asset: Symbol) -> HistoricalOracleData {
        get_historical_oracle_data(&e, &asset)
    }

    // Gets the info on a registered oracle.
    //
    // # Arguments
    //
    // * `asset` - The symbol of the oracle.
    fn get_oracle(e: Env, asset: Symbol) -> OracleInfo {
        get_oracle(&e, &asset)
    }

    //   _______    _______  ___________  ___________  _______   _______    ________
    //  /" _   "|  /"     "|("     _   ")("     _   ")/"     "| /"      \  /"       )
    // (: ( \___) (: ______) )__/  \\__/  )__/  \\__/(: ______)|:        |(:   \___/
    //  \/ \       \/    |      \\_ /        \\_ /    \/    |  |_____/   ) \___  \
    //  //  \ ___  // ___)_     |.  |        |.  |    // ___)_  //      /   __/  \\
    // (:   _(  _|(:      "|    \:  |        \:  |   (:      "||:  __   \  /" \   :)
    //  \_______)  \_______)     \__|         \__|    \_______)|__|  \___)(_______/

    fn get_oracle_guard_rails(e: Env) -> OracleGuardRails {
        get_oracle_guard_rails(&e)
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
        100
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
    //  ___      ___       __        __    _____  ___
    // |"  \    /"  |     /""\      |" \  (\"   \|"  \
    //  \   \  //   |    /    \     ||  | |.\\   \    |
    //  /\\  \/.    |   /' /\  \    |:  | |: \.   \\  |
    // |: \.        |  //  __'  \   |.  | |.  \    \. |
    // |.  \    /:  | /   /  \\  \  /\  |\|    \    \ |
    // |___|\__/|___|(___/    \___)(__\_|_)\___|\____\)

    // Registers a new oracle for a given asset symbol.
    //
    // This function allows an authorized admin to register an oracle source for an asset.
    // It verifies that the oracle isn't already registered, fetches a live price from the
    // provided oracle address, validates the price data, and stores the oracle configuration
    // if valid.
    //
    // # Arguments
    // * `e` - The Soroban environment.
    // * `admin` - The address of the authorized admin performing the registration.
    // * `asset` - The symbol for the asset being registered.
    // * `oracle_addr` - The address of the external oracle contract providing the price.
    // * `decimals` - Decimal precision of the asset prices returned by the oracle.
    // * `sanitize_clamp_denominator` - Clamp denominator used for sanitizing price updates.
    //
    // # Returns
    // * `OracleInfo` - The successfully registered oracle metadata.
    //
    // # Panics
    // * `OracleRegistryError::InvalidClampDenominator` if the sanitize_clamp_denominator is negative.
    // * `OracleRegistryError::OracleAlreadyRegistered` if the asset already has an oracle.
    // * `OracleRegistryError::OracleInvalid` if the provided oracle fails validation (e.g. non-positive, too stale, or volatile).
    fn register_oracle(
        e: Env,
        admin: Address,
        asset: Symbol,
        oracle_addr: Address,
        decimals: u32,
        sanitize_clamp_denominator: u64,
    ) -> OracleInfo {
        admin.require_auth();
        require_admin(&e, &admin);

        enter(&e);

        validate_positive_denominator(
            &e,
            sanitize_clamp_denominator,
            OracleRegistryError::InvalidClampDenominator,
        );

        if get_oracle_base(&e, &asset).is_some() {
            panic_with_error!(&e, OracleRegistryError::OracleAlreadyRegistered);
        }

        let now = e.ledger().timestamp();
        let oracle_price_data = get_oracle_price(&e, &oracle_addr, &asset, now);

        // Check oracle validity
        let oracle_is_valid = oracle_validity(&e, oracle_price_data.price, &oracle_price_data)
            == OracleValidity::Valid;

        if !oracle_is_valid {
            panic_with_error!(&e, OracleRegistryError::OracleInvalid);
        }

        update_twap(
            &e,
            &asset,
            &HistoricalOracleData::default_with_current_oracle(oracle_price_data),
            &oracle_price_data,
            sanitize_clamp_denominator,
            now,
        );

        let oracle = OracleInfo {
            address: oracle_addr,
            decimals,
            frozen: false,
            sanitize_clamp_denominator,
            last_updated: now,
        };
        put_oracle(&e, &asset, &oracle);

        exit(&e);

        oracle
    }

    // Updates the oracle configuration for a given asset.
    //
    // Allows the contract admin to modify fields of an existing oracle, such as its address,
    // decimal precision, clamping settings, or frozen status. Before updating, the function
    // validates any new address by fetching its price and ensuring it meets validity checks.
    // Decimal precision is also validated to be within safe bounds.
    //
    // # Arguments
    // * `e` - The Soroban environment.
    // * `admin` - The authorized admin address initiating the update.
    // * `asset` - The symbol representing the asset whose oracle should be updated.
    // * `params` - A set of optional update parameters (e.g., address, decimals, frozen status).
    //
    // # Returns
    // * `OracleInfo` - The newly updated oracle configuration.
    //
    // # Panics
    // * `OracleRegistryError::OracleInvalid` if the new address returns an invalid price.
    // * `OracleRegistryError::InvalidDecimals` if provided decimals exceed safe limits.
    // * `OracleRegistryError::InvalidClampDenominator` if the sanitize_clamp_denominator is negative.
    // * `OracleRegistryError::OracleNotRegistered` if the asset does not have a registered oracle.
    fn update_oracle(
        e: Env,
        admin: Address,
        asset: Symbol,
        params: MutableOracleInfo,
    ) -> OracleInfo {
        admin.require_auth();
        require_admin(&e, &admin);

        enter(&e);

        if let Some(oracle) = get_oracle_base(&e, &asset) {
            let now = e.ledger().timestamp();

            // Address validation
            if let Some(oracle_addr) = params.address.clone() {
                let oracle_price_data = get_oracle_price(&e, &oracle_addr, &asset, now);

                // Check oracle validity
                let historical_oracle_data = get_historical_oracle_data(&e, &asset);
                let oracle_is_valid = oracle_validity(
                    &e,
                    historical_oracle_data.last_oracle_price_twap,
                    &oracle_price_data,
                ) == OracleValidity::Valid;

                if !oracle_is_valid {
                    panic_with_error!(&e, OracleRegistryError::OracleInvalid);
                }
            }

            // Decimal validation
            if let Some(decimals) = params.decimals {
                if decimals > 18 {
                    panic_with_error!(&e, OracleRegistryError::InvalidDecimals);
                }
            }

            if let Some(sanitize_clamp_denominator) = params.sanitize_clamp_denominator {
                validate_positive_denominator(
                    &e,
                    sanitize_clamp_denominator,
                    OracleRegistryError::InvalidClampDenominator,
                );
            }

            let updated_oracle = OracleInfo {
                address: params.address.unwrap_or(oracle.address),
                decimals: params.decimals.unwrap_or(oracle.decimals),
                sanitize_clamp_denominator: params
                    .sanitize_clamp_denominator
                    .unwrap_or(oracle.sanitize_clamp_denominator),
                frozen: params.frozen.unwrap_or(oracle.frozen),
                last_updated: now,
                ..oracle
            };
            put_oracle(&e, &asset, &updated_oracle);

            exit(&e);

            updated_oracle
        } else {
            exit(&e);
            panic_with_error!(&e, OracleRegistryError::OracleNotRegistered);
        }
    }

    // Deletes an oracle for a given asset.
    //
    //
    // # Arguments
    // * `e` - The Soroban environment.
    // * `admin` - The authorized admin address initiating the deletion.
    // * `asset` - The symbol representing the asset whose oracle should be deleted.
    fn delete_oracle(e: Env, admin: Address, asset: Symbol) {
        admin.require_auth();
        require_admin(&e, &admin);

        enter(&e);

        let oracle = get_oracle_base(&e, &asset);

        match oracle {
            Some(oracle_info) => {
                remove_oracle(&e, &asset);
            }
            None => {
                panic_with_error!(&e, OracleRegistryError::OracleNotFound);
            }
        }

        exit(&e);
    }

    // Manually sets a new oracle price for a given asset.
    //
    // This function is intended for administrative overrides of oracle prices, with safeguards
    // to ensure integrity and temporal spacing between updates. It verifies that the price is valid
    // in the context of recent TWAP history and enforces a minimum interval since the last update
    // before allowing a new override.
    //
    // # Arguments
    // * `e` - The current Soroban environment.
    // * `admin` - The authorized admin address initiating the override.
    // * `asset` - The asset symbol whose oracle price is being set.
    // * `price` - The new price to set as the oracle value (must be valid and positive).
    //
    // # Behavior
    // * Validates the price against current TWAP to ensure it's not too volatile or stale.
    // * Enforces a cooldown period defined by `oracle_guard_rails.validity.seconds_before_stale_for_pool`.
    // * Updates TWAP with the new price and a `delay` of 0 to indicate immediate override.
    //
    // # Panics
    // * `OracleRegistryError::NotAuthorized` if the caller is not an admin.
    // * `OracleRegistryError::OracleInvalid` if the provided price fails volatility or freshness checks.
    // * `OracleRegistryError::PriceOverrideTooSoon` if the override is attempted too soon after the last one.
    //
    // # Notes
    // - This method does not track override timestamps separately; it uses `last_updated` for cooldown logic.
    fn set_oracle_price(e: Env, admin: Address, asset: Symbol, price: u128) {
        admin.require_auth();
        require_admin(&e, &admin);

        ensure_non_zero_u128(&e, price);

        enter(&e);

        let now = e.ledger().timestamp();
        let oracle = get_oracle(&e, &asset);
        let oracle_guard_rails = get_oracle_guard_rails(&e);
        let historical_oracle_data = get_historical_oracle_data(&e, &asset);

        let oracle_is_valid = oracle_validity(
            &e,
            historical_oracle_data.last_oracle_price_twap,
            &(OraclePriceData {
                price,
                delay: Delay::from_timestamp_diff_expect(
                    now,
                    historical_oracle_data.last_oracle_price_twap_ts,
                    "Historical TWAP timestamp cannot be in the future",
                ),
            }),
        ) == OracleValidity::Valid;

        if !oracle_is_valid {
            panic_with_error!(&e, OracleRegistryError::OracleInvalid);
        }

        // Rate limit
        if now - oracle.last_updated <= oracle_guard_rails.validity.seconds_before_stale_for_pool {
            panic_with_error!(&e, OracleRegistryError::PriceOverrideTooSoon);
        }

        update_twap(
            &e,
            &asset,
            &historical_oracle_data,
            &(OraclePriceData {
                price: price,
                delay: Delay::ZERO,
            }),
            oracle.sanitize_clamp_denominator,
            now,
        );

        // Update the oracle's last_updated timestamp to enforce cooldown
        let updated_oracle = OracleInfo {
            last_updated: now,
            ..oracle
        };
        put_oracle(&e, &asset, &updated_oracle);

        exit(&e);
    }

    //   ________  _______  ___________  ___________  _______   _______    ________
    //  /"       )/"     "|("     _   ")("     _   ")/"     "| /"      \  /"       )
    // (:   \___/(: ______) )__/  \\__/  )__/  \\__/(: ______)|:        |(:   \___/
    //  \___  \   \/    |      \\_ /        \\_ /    \/    |  |_____/   ) \___  \
    //   __/  \\  // ___)_     |.  |        |.  |    // ___)_  //      /   __/  \\
    //  /" \   :)(:      "|    \:  |        \:  |   (:      "||:  __   \  /" \   :)
    // (_______/  \_______)     \__|         \__|    \_______)|__|  \___)(_______/

    fn set_oracle_guard_rails(e: Env, admin: Address, oracle_guard_rails: OracleGuardRails) {
        admin.require_auth();
        require_admin(&e, &admin);

        set_oracle_guard_rails(&e, &oracle_guard_rails);
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
            0 => match access_control.get_role_safe(&role) {
                Some(address) => address,
                None => panic_with_error!(&e, AccessControlError::RoleNotFound),
            },
            _ => access_control.get_future_address(&role),
        }
    }
}
