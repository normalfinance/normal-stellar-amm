use crate::errors::{ BufferError };
use crate::events::{ Events, BufferEvents };
use crate::interface::{ AdminInterface, BufferTrait };
use crate::reserve::Reserve;
use crate::storage::{
    get_buffer_reserve_amount,
    get_is_killed_deposit,
    get_is_killed_request_payout,
    get_last_payout_timestamp,
    get_min_reserve_ratio,
    get_min_time_between_payouts,
    get_reserve,
    get_router,
    put_reserve,
    set_fee_collector,
    set_is_killed_deposit,
    set_is_killed_request_payout,
    set_last_payout_timestamp,
    set_min_time_between_payouts,
    set_router,
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
use utils::math::safe_math::SafeMath;
use utils::token::{ transfer_token, validate_tokens_contracts };

#[contract]
pub struct Buffer;

// The `BufferTrait` trait provides the interface for interacting with the buffer.
#[contractimpl]
impl BufferTrait for Buffer {
    fn deposit(e: Env, sender: Address, token: Address, amount: u128) {
        sender.require_auth();

        let router = get_router(&e);
        if sender != router {
            panic_with_error!(&e, BufferError::NotAuthorized);
        }

        validate_tokens_contracts(&e, &Vec::from_array(&e, [token.clone()]));

        let reserve = get_reserve(&e, &token);

        if reserve.balance + amount > reserve.max_balance {
            panic_with_error!(&e, BufferError::MaxBalanceHit);
        }

        // Transfer token to the Buffer
        transfer_token(&e, &token, &sender, &e.current_contract_address(), &(amount as i128));

        // Update the Buffer
        put_reserve(&e, &token, &reserve.deposit(&e, amount));

        Events::new(&e).deposit(token, sender, amount);
    }

    fn request_payout(e: Env, sender: Address, token: Address, amount: u128) {
        sender.require_auth();

        let router = get_router(&e);
        if sender != router {
            panic_with_error!(&e, BufferError::NotAuthorized);
        }

        let now = e.ledger().timestamp();

        // Error if too soon since last payout
        if now - get_last_payout_timestamp(&e) <= get_min_time_between_payouts(&e) {
            panic_with_error!(&e, BufferError::AdminNotSet);
        }

        let mut reserve = get_reserve(&e, &token);

        // Error if insuffient balance
        if amount > balance {
            panic_with_error!(&e, BufferError::AdminNotSet);
        }

        // Transfer tokens to Pool
        transfer_token(&e, &token, &e.current_contract_address(), &sender, &(amount as i128));

        // Update the Buffer
        put_reserve(&e, &token, &reserve.payout(&e, amount, now));
        set_last_payout_timestamp(&e, &now);

        Events::new(&e).request_payout(token, sender, amount);
    }

    // Returns the pool's reserves.
    //
    // # Returns
    //
    // A vector of the pool's reserves.
    fn get_reserve(e: Env, token: Address) -> Reserve {
        get_reserve(&e, &token)
    }

    fn get_min_reserve_ratio(e: Env) -> u128 {
        get_min_reserve_ratio(&e)
    }

    fn get_last_payout_timestamp(e: Env) -> u64 {
        get_last_payout_timestamp(&e)
    }
}

// The `AdminInterface` trait provides the interface for administrative actions.
#[contractimpl]
impl AdminInterface for Buffer {
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

    // Sets the router address.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    // * `router` - The address of the router contract.
    fn set_router(e: Env, admin: Address, router: Address) {
        admin.require_auth();
        let access_control = AccessControl::new(&e);
        access_control.assert_address_has_role(&admin, &Role::Admin);

        set_router(&e, &router);
    }

    // Sets the fee collector address.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    // * `fee_collector` - The address of the fee collector.
    fn set_fee_collector(e: Env, admin: Address, fee_collector: Address) {
        admin.require_auth();
        let access_control = AccessControl::new(&e);
        access_control.assert_address_has_role(&admin, &Role::Admin);

        set_fee_collector(&e, &fee_collector);
    }

    // Sets the minimum time between payouts.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    // * `min_time` - The new minimum time between payouts.
    fn set_min_time_between_payouts(e: Env, admin: Address, min_time: u64) {
        admin.require_auth();
        let access_control = AccessControl::new(&e);
        access_control.assert_address_has_role(&admin, &Role::Admin);

        set_min_time_between_payouts(&e, &min_time);
    }

    // Sets the max reserve balance.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    // * `token` - The address of the token in reserve.
    // * `max_balance` - The new reserve max balance.
    fn set_reserve_max_balance(e: Env, admin: Address, token: Address, max_balance: u128) {
        admin.require_auth();
        let access_control = AccessControl::new(&e);
        access_control.assert_address_has_role(&admin, &Role::Admin);

        let mut reserve = get_reserve(&e, &token);
        put_reserve(&e, &token, &reserve.update_max_balance(max_balance));
    }

    // Withdraws surplus reservess.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    // * `token` - The address of the token in reserve to withdraw.
    // * `amount` - The amount to withdraw.
    fn withdraw_surplus(e: Env, admin: Address, token: Address, amount: u128) {
        admin.require_auth();
        let access_control = AccessControl::new(&e);
        access_control.assert_address_has_role(&admin, &Role::Admin);

        validate_tokens_contracts(&e, &Vec::from_array(&e, [token.clone()]));

        let reserve = get_reserve(&e, &token);

        if amount > reserve.balance {
            panic!("insufficient reserve");
        }

        // must leave minimum buffer
        let min_reserve_ratio = get_min_reserve_ratio(&e);
        let min_reserve = (reserve.balance * (min_reserve_ratio as u128)) / 10_000;
        if reserve.balance - amount < min_reserve {
            panic!("withdrawal violates minimum reserve policy");
        }

        put_reserve(&e, token.clone(), &reserve.withdraw(&e, amount));

        transfer_token(&e, &token, &e.current_contract_address(), &admin, &(amount as i128));

        Events::new(&e).withdraw_surplus(token, admin, amount);
    }

    // Sync token balances with reserves.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    // * `token` - The address of the token to sync.
    fn sync(e: Env, admin: Address, token: Address) {
        admin.require_auth();
        let access_control = AccessControl::new(&e);
        access_control.assert_address_has_role(&admin, &Role::Admin);

        let reserve = get_reserve(&e, &token);
        let balance = get_buffer_reserve_amount(&e, &token);
        put_reserve(&e, &token, &reserve.update_balance(balance));
    }

    // Skim excess token balances.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    // * `token` - The address of the token to skim.
    fn skim(e: Env, admin: Address, token: Address) -> u128 {
        admin.require_auth();
        let access_control = AccessControl::new(&e);
        access_control.assert_address_has_role(&admin, &Role::Admin);

        let reserve = get_reserve(&e, &token);
        let balance = get_buffer_reserve_amount(&e, &token);
        let reserve_balance = reserve.balance;
        put_reserve(&e, &token, &reserve.update_balance(balance));

        let balance_delta = balance.safe_sub(&e, reserve_balance);
        transfer_token(&e, &token, &e.current_contract_address(), &admin, &(balance_delta as i128));

        balance_delta
    }

    // Stops the buffer deposits instantly.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    fn kill_deposit(e: Env, admin: Address) {
        admin.require_auth();
        require_pause_or_emergency_pause_admin_or_owner(&e, &admin);

        set_is_killed_deposit(&e, &true);
        Events::new(&e).kill_deposit();
    }

    // Stops the buffer payouts instantly.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    fn kill_request_payout(e: Env, admin: Address) {
        admin.require_auth();
        require_pause_or_emergency_pause_admin_or_owner(&e, &admin);

        set_is_killed_request_payout(&e, &true);
        Events::new(&e).kill_request_payout();
    }

    // Resumes the buffer deposits.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    fn unkill_deposit(e: Env, admin: Address) {
        admin.require_auth();
        require_pause_admin_or_owner(&e, &admin);

        set_is_killed_deposit(&e, &false);
        Events::new(&e).unkill_deposit();
    }

    // Resumes the buffer payouts.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    fn unkill_request_payout(e: Env, admin: Address) {
        admin.require_auth();
        require_pause_admin_or_owner(&e, &admin);

        set_is_killed_request_payout(&e, &false);
        Events::new(&e).unkill_request_payout();
    }

    // Get deposit killswitch status.
    fn get_is_killed_deposit(e: Env) -> bool {
        get_is_killed_deposit(&e)
    }

    // Get payout killswitch status.
    fn get_is_killed_request_payout(e: Env) -> bool {
        get_is_killed_request_payout(&e)
    }
}

// The `TransferableContract` trait provides the interface for transferring ownership of the contract.
#[contractimpl]
impl TransferableContract for Buffer {
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

// The `UpgradeableContract` trait provides the interface for upgrading the contract.
#[contractimpl]
impl UpgradeableContract for Buffer {
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
