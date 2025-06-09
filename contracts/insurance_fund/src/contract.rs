use crate::errors::InsuranceFundError;
use crate::events::Events as FundEvents;
use crate::events::InsuranceFundEvents;

use crate::interface::{AdminInterface, InsuranceFundTrait};
use crate::stake::{
    apply_rebase_to_insurance_fund, apply_rebase_to_stake, calculate_if_shares_lost, get_stake,
    if_shares_to_vault_amount, save_stake, vault_amount_to_if_shares, StakeAction,
};
use crate::storage::{
    get_insurance_vault_amount, get_is_killed_deposit, get_is_killed_request_withdraw,
    get_is_killed_withdraw, get_max_shares, get_shares_base, get_token, get_total_shares,
    get_unstaking_period, put_token, set_is_killed_deposit, set_is_killed_request_withdraw,
    set_is_killed_withdraw, set_max_shares, set_total_shares, set_unstaking_period,
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
use access_control::utils::{
    require_pause_admin_or_owner, require_pause_or_emergency_pause_admin_or_owner,
};
use soroban_sdk::auth::{ContractContext, InvokerContractAuthEntry, SubContractInvocation};
use soroban_sdk::{
    contract, contractimpl, log, panic_with_error, vec, Address, BytesN, Env, IntoVal, Symbol, Vec,
};
use upgrade::events::Events as UpgradeEvents;
use upgrade::interface::UpgradeableContract;
use upgrade::{apply_upgrade, commit_upgrade, revert_upgrade};
use utils::math::safe_math::SafeMath;
use utils::token::transfer_token;
use utils::validate;

#[contract]
pub struct InsuranceFund;

impl InsuranceFund {
    pub fn __constructor(
        e: Env,
        admin: Address,
        token: Address,
        unstaking_period: u64,
        max_shares: u128,
    ) {
        let access_control = AccessControl::new(&e);
        if access_control.get_role_safe(&Role::Admin).is_some() {
            panic_with_error!(&e, AccessControlError::AdminAlreadySet);
        }
        access_control.set_role_address(&Role::Admin, &admin);

        set_unstaking_period(&e, &unstaking_period);
        set_max_shares(&e, &max_shares);
        put_token(&e, &token)
    }
}

// The `InsuranceFundTrait` trait provides the interface for interacting with a liquidity pool.
#[contractimpl]
impl InsuranceFundTrait for InsuranceFund {
    fn deposit(e: Env, user: Address, amount: u128) {
        user.require_auth();

        if get_is_killed_deposit(&e) {
            panic_with_error!(e, InsuranceFundError::FundDepositKilled);
        }

        let mut stake = get_stake(&e, &user);

        validate!(
            &e,
            stake.last_withdraw_request_shares == 0 && stake.last_withdraw_request_value == 0,
            InsuranceFundError::IFWithdrawRequestInProgress,
            "withdraw request in progress"
        );

        let total_shares = get_total_shares(&e);

        let insurance_vault_amount = get_insurance_vault_amount(&e);

        validate!(
            !(insurance_vault_amount == 0 && total_shares != 0),
            InsuranceFundError::InvalidIFForNewStakes,
            "Insurance Fund balance should be non-zero for new stakers to enter"
        )?;

        apply_rebase_to_insurance_fund(&e, insurance_vault_amount);
        apply_rebase_to_stake(&e, &mut stake);

        let total_shares = get_total_shares(&e);
        let max_shares = get_max_shares(&e);

        let if_shares_before = stake.checked_if_shares(&e);
        let total_if_shares_before = total_shares;

        let n_shares = vault_amount_to_if_shares(&e, amount, total_shares, insurance_vault_amount);

        // Ensure amount will not put Insurance Fund over max_shares
        validate!(
            e,
            total_shares + n_shares <= max_shares,
            InsuranceFundError::TooMuchInsurance,
            ""
        );

        // reset cost basis if no shares
        stake.cost_basis = if if_shares_before == 0 {
            amount
        } else {
            stake.cost_basis.safe_add(&e, amount)
        };

        stake.increase_if_shares(&e, n_shares);

        set_total_shares(&e, &(total_shares + n_shares));

        let if_shares_after = stake.checked_if_shares(&e);

        let new_total_shares = get_total_shares(&e);

        let now = e.ledger().timestamp();
        FundEvents::new(&e).if_stake_record(
            now,
            user.clone(),
            StakeAction::Deposit,
            amount,
            insurance_vault_amount,
            if_shares_before,
            total_if_shares_before,
            if_shares_after,
            new_total_shares,
        );

        transfer_token(
            &e,
            &get_token(&e),
            &user,
            &e.current_contract_address(),
            &(amount as i128),
        );
    }

    fn request_withdraw(e: Env, user: Address, amount: u128) {
        user.require_auth();

        if get_is_killed_request_withdraw(&e) {
            panic_with_error!(e, InsuranceFundError::FundRequestWithdrawKilled);
        }

        let now = e.ledger().timestamp();
        let mut stake = get_stake(&e, &user);

        validate!(
            &e,
            stake.last_withdraw_request_shares == 0,
            InsuranceFundError::IFWithdrawRequestInProgress,
            "Withdraw request is already in progress"
        );

        let total_shares = get_total_shares(&e);
        let insurance_vault_amount = get_insurance_vault_amount(&e);

        let n_shares = vault_amount_to_if_shares(&e, amount, total_shares, insurance_vault_amount);

        validate!(
            &e,
            n_shares > 0,
            InsuranceFundError::IFWithdrawRequestTooSmall,
            "Requested lp_shares = 0"
        );

        let user_if_shares = stake.checked_if_shares(&e);
        validate!(
            &e,
            user_if_shares >= n_shares,
            InsuranceFundError::InsufficientIFShares
        );

        log!(&e, "n_shares {}", n_shares);

        stake.last_withdraw_request_shares = n_shares;

        apply_rebase_to_insurance_fund(&e, insurance_vault_amount);
        apply_rebase_to_stake(&e, &mut stake);

        let total_shares = get_total_shares(&e);
        let shares_base = get_shares_base(&e);

        let if_shares_before = stake.checked_if_shares(&e);
        let total_if_shares_before = total_shares;

        validate!(
            &e,
            stake.last_withdraw_request_shares <= stake.checked_if_shares(&e),
            InsuranceFundError::InvalidInsuranceUnstakeSize,
            "last_withdraw_request_shares exceeds if_shares {} > {}",
            stake.last_withdraw_request_shares,
            stake.checked_if_shares(&e)
        );

        validate!(
            &e,
            stake.if_base == shares_base,
            InsuranceFundError::InvalidIFRebase,
            "if stake base != base"
        );

        stake.last_withdraw_request_value = if_shares_to_vault_amount(
            &e,
            stake.last_withdraw_request_shares,
            total_shares,
            insurance_vault_amount,
        )
        .min(insurance_vault_amount.saturating_sub(1));

        validate!(
            &e,
            stake.last_withdraw_request_value == 0
                || stake.last_withdraw_request_value < insurance_vault_amount,
            InsuranceFundError::InvalidIFUnstakeSize,
            "Requested withdraw value is not below Insurance Fund balance"
        );

        let if_shares_after = stake.checked_if_shares(&e);

        FundEvents::new(&e).if_stake_record(
            now,
            user.clone(),
            StakeAction::WithdrawRequest,
            stake.last_withdraw_request_value,
            insurance_vault_amount,
            if_shares_before,
            total_if_shares_before,
            if_shares_after,
            total_shares,
        );

        stake.last_withdraw_request_ts = now;

        save_stake(&e, &user, &stake);
    }

    fn cancel_request_withdraw(e: Env, user: Address) {
        user.require_auth();

        let now = e.ledger().timestamp();
        let mut stake = get_stake(&e, &user);

        validate!(
            &e,
            stake.last_withdraw_request_shares != 0,
            InsuranceFundError::NoIFWithdrawRequestInProgress,
            "No withdraw request in progress"
        );

        let insurance_vault_amount = get_insurance_vault_amount(&e);

        apply_rebase_to_insurance_fund(&e, insurance_vault_amount);
        apply_rebase_to_stake(&e, &mut stake);

        let total_shares = get_total_shares(&e);
        let shares_base = get_shares_base(&e);

        let if_shares_before = stake.checked_if_shares(&e);
        let total_if_shares_before = total_shares;

        validate!(
            &e,
            stake.if_base == shares_base,
            InsuranceFundError::InvalidIFRebase,
            "if stake base != base"
        );

        validate!(
            &e,
            stake.last_withdraw_request_shares != 0,
            InsuranceFundError::InvalidIFUnstakeCancel,
            "No withdraw request in progress"
        );

        let if_shares_lost = calculate_if_shares_lost(&e, &stake, insurance_vault_amount);

        stake.decrease_if_shares(&e, if_shares_lost);

        set_total_shares(&e, &total_shares.safe_sub(&e, if_shares_lost));

        let if_shares_after = stake.checked_if_shares(&e);

        FundEvents::new(&e).if_stake_record(
            now,
            user.clone(),
            StakeAction::WithdrawCancelRequest,
            0,
            insurance_vault_amount,
            if_shares_before,
            total_if_shares_before,
            if_shares_after,
            total_shares,
        );

        stake.last_withdraw_request_shares = 0;
        stake.last_withdraw_request_value = 0;
        stake.last_withdraw_request_ts = now;
    }

    fn withdraw(e: Env, user: Address) {
        user.require_auth();

        if get_is_killed_withdraw(&e) {
            panic_with_error!(e, InsuranceFundError::FundWithdrawKilled);
        }

        // TODO: check if pools are healthy

        let now = e.ledger().timestamp();
        let mut stake = get_stake(&e, &user);

        let time_since_withdraw_request = now.safe_sub(&e, stake.last_withdraw_request_ts);

        let unstaking_period = get_unstaking_period(&e);
        validate!(
            &e,
            time_since_withdraw_request >= unstaking_period,
            InsuranceFundError::TryingToRemoveLiquidityTooFast,
            ""
        );

        let insurance_vault_amount = get_insurance_vault_amount(&e);

        apply_rebase_to_insurance_fund(&e, insurance_vault_amount);
        apply_rebase_to_stake(&e, &mut stake);

        let total_shares = get_total_shares(&e);

        let if_shares_before = stake.checked_if_shares(&e);
        let total_if_shares_before = total_shares;

        let n_shares = stake.last_withdraw_request_shares;

        validate!(
            &e,
            n_shares > 0,
            InsuranceFundError::InvalidIFUnstake,
            "Must submit withdraw request and wait the escrow period"
        );

        validate!(
            &e,
            if_shares_before >= n_shares,
            InsuranceFundError::InsufficientIFShares,
            ""
        );

        let amount = if_shares_to_vault_amount(&e, n_shares, total_shares, insurance_vault_amount);

        let _if_shares_lost = calculate_if_shares_lost(&e, &stake, insurance_vault_amount);

        let withdraw_amount = amount.min(stake.last_withdraw_request_value);

        stake.decrease_if_shares(&e, n_shares);

        stake.cost_basis = stake.cost_basis.safe_sub(&e, withdraw_amount);

        set_total_shares(&e, &total_shares.safe_sub(&e, n_shares));

        // reset stake withdraw request info
        stake.last_withdraw_request_shares = 0;
        stake.last_withdraw_request_value = 0;
        stake.last_withdraw_request_ts = now;

        let if_shares_after = stake.checked_if_shares(&e);

        FundEvents::new(&e).if_stake_record(
            now,
            user.clone(),
            StakeAction::Withdraw,
            withdraw_amount,
            insurance_vault_amount,
            if_shares_before,
            total_if_shares_before,
            if_shares_after,
            total_shares,
        );

        transfer_token(
            &e,
            &get_token(&e),
            &e.current_contract_address(),
            &user,
            &(withdraw_amount as i128),
        );

        let insurance_vault_amount = get_insurance_vault_amount(&e);
        validate!(
            &e,
            insurance_vault_amount > 0,
            InsuranceFundError::InvalidIFDetected,
            "insurance_fund_vault.amount must remain > 0"
        );
    }
}

// The `UpgradeableContract` trait provides the interface for upgrading the contract.
#[contractimpl]
impl UpgradeableContract for InsuranceFund {
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
impl AdminInterface for InsuranceFund {
    // Sets the unstaking period.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    // * `unstaking_period` - The new unstaking period.
    fn set_unstaking_period(e: Env, admin: Address, unstaking_period: u64) {
        admin.require_auth();
        let access_control = AccessControl::new(&e);
        access_control.assert_address_has_role(&admin, &Role::Admin);

        set_unstaking_period(&e, &unstaking_period);
    }

    // Sets the max shares the Insurance Fund can have.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    // * `max_shares` - The max number of shares.
    fn set_max_shares(e: Env, admin: Address, max_shares: u128) {
        admin.require_auth();
        let access_control = AccessControl::new(&e);
        access_control.assert_address_has_role(&admin, &Role::Admin);

        set_max_shares(&e, &max_shares);
    }

    // Sets the max shares the Insurance Fund can have.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    // * `max_shares` - The max number of shares.ƒ
    fn resolve_liquidity_deficit(e: Env, admin: Address, pool_address: Address) {
        admin.require_auth();

        let insurance_vault_amount = get_insurance_vault_amount(&e);

        let pay_from_insurance = e.invoke_contract(
            &pool_address,
            &Symbol::new(&e, "get_pay_from_insurance"),
            Vec::from_array(
                &e,
                [
                    e.current_contract_address().to_val(),
                    insurance_vault_amount.into_val(&e),
                ],
            ),
        );

        if pay_from_insurance > 0 {
            validate!(
                &e,
                pay_from_insurance < insurance_vault_amount,
                InsuranceFundError::InsufficientCollateral,
                "Insurance Fund balance InsufficientCollateral for payment: !{} < {}",
                pay_from_insurance,
                insurance_vault_amount
            );

            e.authorize_as_current_contract(vec![
                &e,
                InvokerContractAuthEntry::Contract(SubContractInvocation {
                    context: ContractContext {
                        contract: get_token(&e).clone(),
                        fn_name: Symbol::new(&e, "transfer"),
                        args: (
                            e.current_contract_address(),
                            pool_address.clone(),
                            pay_from_insurance as i128,
                        )
                            .into_val(&e),
                    },
                    sub_invocations: vec![&e],
                }),
            ]);

            e.invoke_contract(
                &pool_address,
                &Symbol::new(&e, "pay_insurance_claim"),
                Vec::from_array(
                    &e,
                    [
                        e.current_contract_address().to_val(),
                        pay_from_insurance.into_val(&e),
                    ],
                ),
            );

            let new_insurance_vault_amount = get_insurance_vault_amount(&e);
            validate!(
                &e,
                new_insurance_vault_amount > 0,
                InsuranceFundError::InvalidIFDetected,
                "insurance_fund_vault_amount must remain > 0"
            );
        }
    }

    // Stops the insurance fund deposits instantly.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    fn kill_deposit(e: Env, admin: Address) {
        admin.require_auth();
        require_pause_or_emergency_pause_admin_or_owner(&e, &admin);

        set_is_killed_deposit(&e, &true);
        FundEvents::new(&e).kill_deposit();
    }

    // Stops the pool withdrawals instantly.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    fn kill_request_withdraw(e: Env, admin: Address) {
        admin.require_auth();
        require_pause_or_emergency_pause_admin_or_owner(&e, &admin);

        set_is_killed_request_withdraw(&e, &true);
        FundEvents::new(&e).kill_request_withdraw();
    }

    // Stops the pool swaps instantly.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    fn kill_withdraw(e: Env, admin: Address) {
        admin.require_auth();
        require_pause_or_emergency_pause_admin_or_owner(&e, &admin);

        set_is_killed_withdraw(&e, &true);
        FundEvents::new(&e).kill_withdraw();
    }

    // Resumes the pool deposits.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    fn unkill_deposit(e: Env, admin: Address) {
        admin.require_auth();
        require_pause_admin_or_owner(&e, &admin);

        set_is_killed_deposit(&e, &false);
        FundEvents::new(&e).unkill_deposit();
    }

    // Resumes the pool swaps.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    fn unkill_request_withdraw(e: Env, admin: Address) {
        admin.require_auth();
        require_pause_admin_or_owner(&e, &admin);

        set_is_killed_request_withdraw(&e, &false);
        FundEvents::new(&e).unkill_request_withdraw();
    }

    // Resumes the pool withdrawals.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    fn unkill_withdraw(e: Env, admin: Address) {
        admin.require_auth();
        require_pause_admin_or_owner(&e, &admin);

        set_is_killed_withdraw(&e, &false);
        FundEvents::new(&e).unkill_withdraw();
    }

    // Get deposit killswitch status.
    fn get_is_killed_deposit(e: Env) -> bool {
        get_is_killed_deposit(&e)
    }

    // Get swap killswitch status.
    fn get_is_killed_request_withdraw(e: Env) -> bool {
        get_is_killed_request_withdraw(&e)
    }

    // Get withdraw killswitch status.
    fn get_is_killed_withdraw(e: Env) -> bool {
        get_is_killed_withdraw(&e)
    }
}

// The `TransferableContract` trait provides the interface for transferring ownership of the contract.
#[contractimpl]
impl TransferableContract for InsuranceFund {
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
