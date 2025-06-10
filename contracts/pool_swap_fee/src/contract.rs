use crate::errors::Error;
use crate::events::{ Events, ProviderFeeEvents };
use crate::interface::{ AdminInterface, PoolSwapFeeInterface };
use crate::incentives::get_incentives_manager;
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
use pool_tokens::{get_total_lp_tokens, get_user_balance_lp};
use utils::math::safe_math::SafeMath;
use crate::storage::{
    get_buffer,
    get_buffer_fraction,
    get_fee_destination,
    get_lp_revenue_fraction,
    get_max_swap_fee_fraction,
    get_router,
    set_buffer,
    set_buffer_fraction,
    set_fee_destination,
    set_max_swap_fee_fraction,
    set_router,
};
use soroban_sdk::auth::{ ContractContext, InvokerContractAuthEntry, SubContractInvocation };
use soroban_sdk::token::Client as SorobanTokenClient;
use soroban_sdk::{
    contract,
    contractimpl,
    panic_with_error,
    vec,
    Address,
    BytesN,
    Env,
    IntoVal,
    Symbol,
    Vec,
};
use upgrade::events::Events as UpgradeEvents;
use upgrade::interface::UpgradeableContract;
use upgrade::{ apply_upgrade, commit_upgrade, revert_upgrade };
use utils::constant::{ FEE_DENOMINATOR, PRICE_PRECISION };
use utils::token::transfer_token;

#[contract]
pub struct PoolSwapFeeCollector;

#[contractimpl]
impl PoolSwapFeeInterface for PoolSwapFeeCollector {
    // swap
    // Executes a token swap with fee deduction.
    //
    // Arguments:
    //   - e: The Soroban environment.
    //   - user: The user initiating the swap (must be authorized).
    //   - swap: The swap args.
    //   - token_in: The input token address.
    //   - in_amount: The amount of token_in provided by the user.
    //   - out_min: The minimum acceptable output token amount (after fee deduction).
    //   - fee_fraction: The provider fee fraction in basis points (bps).
    //
    // Returns:
    //   - A u128 value representing the net output tokens transferred to the user.
    fn swap(
        e: Env,
        user: Address,
        swap: (Vec<Address>, BytesN<32>, Address),
        token_in: Address,
        in_amount: u128,
        out_min: u128,
        fee_fraction: u32
    ) -> u128 {
        user.require_auth();

        if fee_fraction > get_max_swap_fee_fraction(&e) {
            panic_with_error!(&e, Error::FeeFractionTooHigh);
        }

        let (_, _, token_out) = swap.clone();

        transfer_token(&e, &token_in, &user, &e.current_contract_address(), &(in_amount as i128));

        let router = get_router(&e);
        e.authorize_as_current_contract(
            vec![
                &e,
                InvokerContractAuthEntry::Contract(SubContractInvocation {
                    context: ContractContext {
                        contract: token_in.clone(),
                        fn_name: Symbol::new(&e, "transfer"),
                        args: (
                            e.current_contract_address(),
                            router.clone(),
                            in_amount as i128,
                        ).into_val(&e),
                    },
                    sub_invocations: vec![&e],
                })
            ]
        );
        let amount_out: u128 = e.invoke_contract(
            &router,
            &Symbol::new(&e, "swap"),
            Vec::from_array(&e, [
                e.current_contract_address().to_val(),
                swap.into_val(&e),
                token_in.clone().to_val(),
                in_amount.into_val(&e),
                out_min.into_val(&e),
            ])
        );
        let fee_amount = (amount_out * (fee_fraction as u128)) / (FEE_DENOMINATOR as u128);
        let amount_out_w_fee = amount_out - fee_amount;
        if amount_out_w_fee < out_min {
            panic_with_error!(&e, Error::OutMinNotSatisfied);
        }
        transfer_token(
            &e,
            &token_out,
            &e.current_contract_address(),
            &user,
            &(amount_out_w_fee as i128)
        );
        Events::new(&e).charge_provider_fee(token_out.clone(), fee_amount);

        //
        let lp_revenue_fraction = get_lp_revenue_fraction(&e);
        let lp_fee_amount =
            (fee_amount * (lp_revenue_fraction as u128)) / (FEE_DENOMINATOR as u128);

        // Deposit portion of swap fee to the Buffer
        let protocol_fee_amount = fee_amount.safe_sub(&e, lp_fee_amount);

        let buffer_fraction = get_buffer_fraction(&e);
        let fee_amount_for_buffer = (protocol_fee_amount * (buffer_fraction as u128)) / 10_000_u128;
        let buffer = get_buffer(&e);

        e.authorize_as_current_contract(
            vec![
                &e,
                InvokerContractAuthEntry::Contract(SubContractInvocation {
                    context: ContractContext {
                        contract: token_out.clone(),
                        fn_name: Symbol::new(&e, "transfer"),
                        args: (
                            e.current_contract_address(),
                            buffer.clone(),
                            fee_amount_for_buffer as i128,
                        ).into_val(&e),
                    },
                    sub_invocations: vec![&e],
                })
            ]
        );
        let _: u128 = e.invoke_contract(
            &get_buffer(&e),
            &Symbol::new(&e, "deposit"),
            Vec::from_array(&e, [
                e.current_contract_address().to_val(),
                token_out.clone().to_val(),
                fee_amount_for_buffer.into_val(&e),
            ])
        );

        Events::new(&e).settle_revenue(token_out, fee_amount_for_buffer as u128);

        let remaining_fee = protocol_fee_amount.safe_sub(&e, fee_amount_for_buffer);

        // Update total incentives data and refresh/initialize user incentive
        let out_idx = 0;
        let incentives = get_incentives_manager(&e);
        let total_shares = get_total_lp_tokens(&e);
        let user_shares = get_user_balance_lp(&e, &user);
        let token_a_fee = if out_idx == 0 { lp_fee_amount } else { 0 };
        let token_b_fee = if out_idx == 0 { 0 } else { lp_fee_amount };
        incentives
            .manager()
            .checkpoint_user(&user, total_shares, user_shares, token_a_fee, token_b_fee);

        amount_out_w_fee
    }

    // swap_strict_receive
    // Executes a swap ensuring a specific output amount by adjusting the input and fee.
    //
    // Arguments:
    //   - e: The Soroban environment.
    //   - user: The user initiating the swap (must be authorized).
    //   - swap: A vector defining the swap path.
    //   - token_in: The input token address.
    //   - out_amount: The exact target output amount.
    //   - in_max: The maximum amount of token_in the user is willing to spend.
    //   - fee_fraction: The provider fee fraction in basis points (bps).
    //
    // Returns:
    //   - A u128 value representing the total input amount (including fees) required.
    fn swap_strict_receive(
        e: Env,
        user: Address,
        swap: (Vec<Address>, BytesN<32>, Address),
        token_in: Address,
        out_amount: u128,
        in_max: u128,
        fee_fraction: u32
    ) -> u128 {
        user.require_auth();

        if fee_fraction > get_max_swap_fee_fraction(&e) {
            panic_with_error!(&e, Error::FeeFractionTooHigh);
        }

        let (_, _, token_out) = swap.clone();

        transfer_token(&e, &token_in, &user, &e.current_contract_address(), &(in_max as i128));
        let router = get_router(&e);
        e.authorize_as_current_contract(
            vec![
                &e,
                InvokerContractAuthEntry::Contract(SubContractInvocation {
                    context: ContractContext {
                        contract: token_in.clone(),
                        fn_name: Symbol::new(&e, "transfer"),
                        args: (
                            e.current_contract_address(),
                            router.clone(),
                            in_max as i128,
                        ).into_val(&e),
                    },
                    sub_invocations: vec![&e],
                })
            ]
        );
        let amount_in: u128 = e.invoke_contract(
            &router,
            &Symbol::new(&e, "swap_strict_receive"),
            Vec::from_array(&e, [
                e.current_contract_address().to_val(),
                swap.into_val(&e),
                token_in.clone().to_val(),
                out_amount.into_val(&e),
                in_max.into_val(&e),
            ])
        );
        transfer_token(&e, &token_out, &e.current_contract_address(), &user, &(out_amount as i128));
        let fee_amount = (amount_in * (fee_fraction as u128)) / (FEE_DENOMINATOR as u128);
        let amount_in_with_fee = amount_in + fee_amount;
        if amount_in_with_fee > in_max {
            panic_with_error!(&e, Error::InMaxNotSatisfied);
        }
        let surplus = in_max - amount_in_with_fee;
        if surplus > 0 {
            transfer_token(&e, &token_in, &e.current_contract_address(), &user, &(surplus as i128));
        }
        Events::new(&e).charge_provider_fee(token_in, fee_amount);

        // Deposit portion of swap fee to the Buffer
        let buffer_fraction = get_buffer_fraction(&e);
        let fee_amount_for_buffer = (fee_amount * (buffer_fraction as u128)) / 10_000_u128;
        let buffer = get_buffer(&e);

        e.authorize_as_current_contract(
            vec![
                &e,
                InvokerContractAuthEntry::Contract(SubContractInvocation {
                    context: ContractContext {
                        contract: token_out.clone(),
                        fn_name: Symbol::new(&e, "transfer"),
                        args: (
                            e.current_contract_address(),
                            buffer.clone(),
                            fee_amount_for_buffer as i128,
                        ).into_val(&e),
                    },
                    sub_invocations: vec![&e],
                })
            ]
        );
        let _: u128 = e.invoke_contract(
            &get_buffer(&e),
            &Symbol::new(&e, "deposit"),
            Vec::from_array(&e, [
                e.current_contract_address().to_val(),
                token_out.clone().to_val(),
                fee_amount_for_buffer.into_val(&e),
            ])
        );

        Events::new(&e).settle_revenue(token_out, fee_amount_for_buffer as u128);

        fee_amount.safe_sub(&e, fee_amount_for_buffer);

        amount_in_with_fee
    }
}

// The `AdminInterface` trait provides the interface for administrative actions.
#[contractimpl]
impl AdminInterface for PoolSwapFeeCollector {
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

    // Sets the buffer address.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    // * `buffer` - The address of the Buffer contract.
    fn set_buffer(e: Env, admin: Address, buffer: Address) {
        admin.require_auth();
        let access_control = AccessControl::new(&e);
        access_control.assert_address_has_role(&admin, &Role::Admin);

        set_buffer(&e, &buffer);
    }

    // Sets the fee destination address.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    // * `fee_destination` - The address of the fee destination.
    fn set_fee_destination(e: Env, admin: Address, fee_destination: Address) {
        admin.require_auth();
        let access_control = AccessControl::new(&e);
        access_control.assert_address_has_role(&admin, &Role::Admin);

        set_fee_destination(&e, &fee_destination);
    }

    // Set the buffer fraction
    fn set_buffer_fraction(e: Env, admin: Address, buffer_fraction: u32) {
        admin.require_auth();
        let access_control = AccessControl::new(&e);
        access_control.assert_address_has_role(&admin, &Role::Admin);

        set_buffer_fraction(&e, &buffer_fraction);
    }

    // Set the max swap fee fraction
    fn set_max_swap_fee_fraction(e: Env, admin: Address, max_swap_fee_fraction: u32) {
        admin.require_auth();
        let access_control = AccessControl::new(&e);
        access_control.assert_address_has_role(&admin, &Role::Admin);

        set_max_swap_fee_fraction(&e, &max_swap_fee_fraction);
    }

    // get_router
    // Returns the address of the router contract used for swaps.
    //
    // Arguments:
    //   - e: The Soroban environment.
    //
    // Returns:
    //   - An Address representing the router.
    fn get_router(e: Env) -> Address {
        get_router(&e)
    }

    // get_buffer
    // Returns the address of the buffer contract used for fee deposits.
    //
    // Arguments:
    //   - e: The Soroban environment.
    //
    // Returns:
    //   - An Address representing the buffer.
    fn get_buffer(e: Env) -> Address {
        get_buffer(&e)
    }

    // get_fee_destination
    // Returns the address where fees are sent.
    //
    // Arguments:
    //   - e: The Soroban environment.
    //
    // Returns:
    //   - An Address representing the fee destination.
    fn get_fee_destination(e: Env) -> Address {
        get_fee_destination(&e)
    }

    // get_max_swap_fee_fraction
    // Returns the maximum swap fee in basis points.
    //
    // Arguments:
    //   - e: The Soroban environment.
    //
    // Returns:
    //   - A u32 value representing the maximum fee in basis points.
    fn get_max_swap_fee_fraction(e: Env) -> u32 {
        get_max_swap_fee_fraction(&e)
    }

    // get_buffer_fraction
    // Returns the buffer revenue fee in basis points.
    //
    // Arguments:
    //   - e: The Soroban environment.
    //
    // Returns:
    //   - A u32 value representing the portion of revenue for the buffer in basis points.
    fn get_buffer_fraction(e: Env) -> u32 {
        get_buffer_fraction(&e)
    }

    // claim_fees
    // Claims all fees held by the contract and transfers them to the specified address.
    //
    // Arguments:
    //   - e: The Soroban environment.
    //   - admin: The address calling for fee claiming (must match the stored operator).
    //   - token: The token contract address for which fees are claimed.
    //
    // Returns:
    //   - A u128 value representing the claimed token amount.
    fn claim_fees(e: Env, admin: Address, token: Address) -> u128 {
        admin.require_auth();
        let access_control = AccessControl::new(&e);
        access_control.assert_address_has_role(&admin, &Role::Admin);

        let token_client = SorobanTokenClient::new(&e, &token);
        let amount = token_client.balance(&e.current_contract_address());
        transfer_token(
            &e,
            &token,
            &e.current_contract_address(),
            &get_fee_destination(&e),
            &amount
        );
        Events::new(&e).claim_fee(token.clone(), amount as u128, token, amount as u128);
        amount as u128
    }

    // claim_fees_and_swap
    // Claims fees and swaps them immediately using the router.
    //
    // Arguments:
    //   - e: The Soroban environment.
    //   - admin: The address calling for the fee claim and swap (must match the stored operator).
    //   - swap: A vector describing the swap path; each element is a tuple of (intermediate token addresses, function hash, output token address).
    //   - token: The token for which fees are claimed.
    //   - out_min: The minimum acceptable output amount from the swap.
    //   - to: The destination address for the swapped tokens.
    //
    // Returns:
    //   - A u128 value representing the output token amount received after the swap.
    fn claim_fees_and_swap(
        e: Env,
        admin: Address,
        swap: (Vec<Address>, BytesN<32>, Address),
        token: Address,
        out_min: u128
    ) -> u128 {
        admin.require_auth();
        let access_control = AccessControl::new(&e);
        access_control.assert_address_has_role(&admin, &Role::Admin);

        let (_, _, token_out) = swap.clone();
        let router = get_router(&e);
        let token_client = SorobanTokenClient::new(&e, &token);
        let amount = token_client.balance(&e.current_contract_address()) as u128;
        e.authorize_as_current_contract(
            vec![
                &e,
                InvokerContractAuthEntry::Contract(SubContractInvocation {
                    context: ContractContext {
                        contract: token.clone(),
                        fn_name: Symbol::new(&e, "transfer"),
                        args: (
                            e.current_contract_address(),
                            router.clone(),
                            amount as i128,
                        ).into_val(&e),
                    },
                    sub_invocations: vec![&e],
                })
            ]
        );
        let out_amount: u128 = e.invoke_contract(
            &get_router(&e),
            &Symbol::new(&e, "swap"),
            Vec::from_array(&e, [
                e.current_contract_address().to_val(),
                swap.into_val(&e),
                token.clone().to_val(),
                amount.into_val(&e),
                out_min.into_val(&e),
            ])
        );
        transfer_token(
            &e,
            &token_out,
            &e.current_contract_address(),
            &get_fee_destination(&e),
            &(out_amount as i128)
        );
        Events::new(&e).claim_fee(token, amount, token_out, out_amount);
        out_amount
    }
}

// The `UpgradeableContract` trait provides the interface for upgrading the contract.
#[contractimpl]
impl UpgradeableContract for PoolSwapFeeCollector {
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

// The `TransferableContract` trait provides the interface for transferring ownership of the contract.
#[contractimpl]
impl TransferableContract for PoolSwapFeeCollector {
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
