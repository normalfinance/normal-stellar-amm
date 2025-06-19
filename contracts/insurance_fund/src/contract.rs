use crate::errors::InsuranceFundError;
use crate::events::Events as FundEvents;
use crate::events::InsuranceFundEvents;
use crate::interest::calculate_rate;
use crate::interest::calculate_utilization;
use crate::interface::{ AdminInterface, InsuranceFundTrait };
use crate::stake::Stake;
use crate::stake::{
    apply_rebase_to_insurance_fund,
    apply_rebase_to_stake,
    calculate_if_shares_lost,
    get_stake,
    if_shares_to_vault_amount,
    save_stake,
    vault_amount_to_if_shares,
    StakeAction,
};
use crate::storage::{
    get_base_rate,
    get_coverage_buffer,
    get_optimal_coverage,
    get_optimal_utilization,
    get_rate_slope_a,
    get_rate_slope_b,
    get_token,
    get_insurance_vault_amount,
    get_is_killed_deposit,
    get_is_killed_request_withdraw,
    get_is_killed_withdraw,
    get_shares_base,
    get_total_shares,
    get_unstaking_period,
    set_base_rate,
    set_coverage_buffer,
    set_optimal_coverage,
    set_optimal_utilization,
    set_rate_slope_a,
    set_rate_slope_b,
    set_token,
    set_is_killed_deposit,
    set_is_killed_request_withdraw,
    set_is_killed_withdraw,
    set_total_shares,
    set_unstaking_period,
};

use access_control::access::{ AccessControl, AccessControlTrait };
use access_control::emergency::{ get_emergency_mode, set_emergency_mode };
use access_control::errors::AccessControlError;
use access_control::events::Events as AccessControlEvents;
use access_control::interface::TransferableContract;
use access_control::management::SingleAddressManagementTrait;
use access_control::role::{ Role, SymbolRepresentation };
use access_control::transfer::TransferOwnershipTrait;
use access_control::utils::{ require_admin };
use soroban_sdk::{
    contract,
    contractimpl,
    panic_with_error,
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
use utils::math::safe_math::SafeMath;
use utils::token::transfer_token;
use utils::validate;

#[contract]
pub struct InsuranceFund;

// The `InsuranceFundTrait` trait provides the interface for interacting with a liquidity pool.
#[contractimpl]
impl InsuranceFundTrait for InsuranceFund {
    fn initialize(
        e: Env,
        admin: Address,
        emergency_admin: Address,
        token: Address,
        unstaking_period: u64,
        coverage_buffer: u128,
        optimal_utilization: u32,
        base_rate: i32,
        rate_slopes: (i32, i32)
    ) {
        admin.require_auth();

        let access_control = AccessControl::new(&e);
        if access_control.get_role_safe(&Role::Admin).is_some() {
            panic_with_error!(&e, AccessControlError::AdminAlreadySet);
        }
        access_control.set_role_address(&Role::Admin, &admin);
        access_control.set_role_address(&Role::EmergencyAdmin, &emergency_admin);

        set_token(&e, &token);

        set_unstaking_period(&e, &unstaking_period);
        set_coverage_buffer(&e, &coverage_buffer);

        set_optimal_utilization(&e, &optimal_utilization);
        set_base_rate(&e, &base_rate);
        set_rate_slope_a(&e, &rate_slopes.0);
        set_rate_slope_b(&e, &rate_slopes.1);
    }

    /**
     * Deposit Tests
     * [ ] Singular deposit
     * [ ] Multiple deposits, same user
     * [ ] Multiple deposits, different users
     * [x] Deposit over optimal coverage FAIL 20
     * [x] Deposit while withdraw in progress FAIL 9
     *
     */
    fn deposit(e: Env, user: Address, amount: u128) {
        user.require_auth();

        if get_is_killed_deposit(&e) {
            panic_with_error!(e, InsuranceFundError::FundDepositKilled);
        }

        let now = e.ledger().timestamp();

        let optimal_coverage = get_optimal_coverage(&e);
        let coverage_buffer = get_coverage_buffer(&e);
        let insurance_vault_amount = get_insurance_vault_amount(&e);

        // Ensure amount will not put Insurance Fund over optimal coverage (plus the buffer if it's set)
        validate!(
            e,
            insurance_vault_amount + amount <= optimal_coverage + coverage_buffer,
            InsuranceFundError::TooMuchInsurance
        );

        let mut stake = get_stake(&e, &user);

        // "withdraw request in progress"
        validate!(
            &e,
            stake.last_withdraw_request_shares == 0 && stake.last_withdraw_request_value == 0,
            InsuranceFundError::IFWithdrawRequestInProgress
        );

        apply_rebase_to_insurance_fund(&e, insurance_vault_amount);
        apply_rebase_to_stake(&e, &mut stake);

        let total_shares = get_total_shares(&e);

        let if_shares_before = stake.checked_if_shares(&e);
        let total_if_shares_before = total_shares;

        let n_shares = vault_amount_to_if_shares(&e, amount, total_shares, insurance_vault_amount);

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

        FundEvents::new(&e).if_stake_record(
            now,
            user.clone(),
            StakeAction::Deposit,
            amount,
            insurance_vault_amount,
            if_shares_before,
            total_if_shares_before,
            if_shares_after,
            new_total_shares
        );

        save_stake(&e, &user, &stake);

        transfer_token(&e, &get_token(&e), &user, &e.current_contract_address(), &(amount as i128));
    }

    fn request_withdraw(e: Env, user: Address, amount: u128) {
        user.require_auth();

        if get_is_killed_request_withdraw(&e) {
            panic_with_error!(e, InsuranceFundError::FundRequestWithdrawKilled);
        }

        let now = e.ledger().timestamp();
        let mut stake = get_stake(&e, &user);

        // "Withdraw request is already in progress"
        validate!(
            &e,
            stake.last_withdraw_request_shares == 0,
            InsuranceFundError::IFWithdrawRequestInProgress
        );

        let total_shares = get_total_shares(&e);
        let insurance_vault_amount = get_insurance_vault_amount(&e);

        let n_shares = vault_amount_to_if_shares(&e, amount, total_shares, insurance_vault_amount);

        // "Requested lp_shares = 0"
        validate!(&e, n_shares > 0, InsuranceFundError::IFWithdrawRequestTooSmall);

        let user_if_shares = stake.checked_if_shares(&e);
        validate!(&e, user_if_shares >= n_shares, InsuranceFundError::InsufficientIFShares);

        stake.last_withdraw_request_shares = n_shares;

        apply_rebase_to_insurance_fund(&e, insurance_vault_amount);
        apply_rebase_to_stake(&e, &mut stake);

        let total_shares = get_total_shares(&e);
        let shares_base = get_shares_base(&e);

        let if_shares_before = stake.checked_if_shares(&e);
        let total_if_shares_before = total_shares;

        // "last_withdraw_request_shares exceeds if_shares {} > {}",
        validate!(
            &e,
            stake.last_withdraw_request_shares <= stake.checked_if_shares(&e),
            InsuranceFundError::InvalidInsuranceUnstakeSize
        );

        // "if stake base != base"
        validate!(&e, stake.if_base == shares_base, InsuranceFundError::InvalidIFRebase);

        stake.last_withdraw_request_value = if_shares_to_vault_amount(
            &e,
            stake.last_withdraw_request_shares,
            total_shares,
            insurance_vault_amount
        ).min(insurance_vault_amount.saturating_sub(1));

        //  "Requested withdraw value is not below Insurance Fund balance"
        validate!(
            &e,
            stake.last_withdraw_request_value == 0 ||
                stake.last_withdraw_request_value < insurance_vault_amount,
            InsuranceFundError::InvalidIFUnstakeSize
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
            total_shares
        );

        stake.last_withdraw_request_ts = now;

        save_stake(&e, &user, &stake);
    }

    fn cancel_request_withdraw(e: Env, user: Address) {
        user.require_auth();

        let now = e.ledger().timestamp();
        let mut stake = get_stake(&e, &user);

        //  "No withdraw request in progress"
        validate!(
            &e,
            stake.last_withdraw_request_shares != 0,
            InsuranceFundError::NoIFWithdrawRequestInProgress
        );

        let insurance_vault_amount = get_insurance_vault_amount(&e);

        apply_rebase_to_insurance_fund(&e, insurance_vault_amount);
        apply_rebase_to_stake(&e, &mut stake);

        let total_shares = get_total_shares(&e);
        let shares_base = get_shares_base(&e);

        let if_shares_before = stake.checked_if_shares(&e);
        let total_if_shares_before = total_shares;

        //  "if stake base != base"
        validate!(&e, stake.if_base == shares_base, InsuranceFundError::InvalidIFRebase);

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
            total_shares
        );

        stake.last_withdraw_request_shares = 0;
        stake.last_withdraw_request_value = 0;
        stake.last_withdraw_request_ts = now;

        save_stake(&e, &user, &stake);
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
            InsuranceFundError::TryingToRemoveLiquidityTooFast
        );

        let insurance_vault_amount = get_insurance_vault_amount(&e);

        apply_rebase_to_insurance_fund(&e, insurance_vault_amount);
        apply_rebase_to_stake(&e, &mut stake);

        let total_shares = get_total_shares(&e);

        let if_shares_before = stake.checked_if_shares(&e);
        let total_if_shares_before = total_shares;

        let n_shares = stake.last_withdraw_request_shares;

        //  "Must submit withdraw request and wait the escrow period"
        validate!(&e, n_shares > 0, InsuranceFundError::InvalidIFUnstake);

        validate!(&e, if_shares_before >= n_shares, InsuranceFundError::InsufficientIFShares);

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
            total_shares
        );

        save_stake(&e, &user, &stake);

        transfer_token(
            &e,
            &get_token(&e),
            &e.current_contract_address(),
            &user,
            &(withdraw_amount as i128)
        );

        let insurance_vault_amount = get_insurance_vault_amount(&e);
        // "insurance_fund_vault.amount must remain > 0"
        validate!(&e, insurance_vault_amount > 0, InsuranceFundError::InvalidIFDetected);
    }

    fn pay_premium(e: Env, sender: Address, amount: u128) {
        sender.require_auth();

        transfer_token(
            &e,
            &get_token(&e),
            &sender,
            &e.current_contract_address(),
            &(amount as i128)
        );

        FundEvents::new(&e).collect_premium(sender, amount);
    }

    //   _______    _______  ___________  ___________  _______   _______    ________
    //  /" _   "|  /"     "|("     _   ")("     _   ")/"     "| /"      \  /"       )
    // (: ( \___) (: ______) )__/  \\__/  )__/  \\__/(: ______)|:        |(:   \___/
    //  \/ \       \/    |      \\_ /        \\_ /    \/    |  |_____/   ) \___  \
    //  //  \ ___  // ___)_     |.  |        |.  |    // ___)_  //      /   __/  \\
    // (:   _(  _|(:      "|    \:  |        \:  |   (:      "||:  __   \  /" \   :)
    //  \_______)  \_______)     \__|         \__|    \_______)|__|  \___)(_______/

    fn get_token(e: Env) -> Address {
        get_token(&e)
    }

    fn get_unstaking_period(e: Env) -> u64 {
        get_unstaking_period(&e)
    }

    fn get_optimal_coverage(e: Env) -> u128 {
        get_optimal_coverage(&e)
    }

    fn get_coverage_buffer(e: Env) -> u128 {
        get_coverage_buffer(&e)
    }

    fn get_total_shares(e: Env) -> u128 {
        get_total_shares(&e)
    }

    fn get_share_base(e: Env) -> u128 {
        get_shares_base(&e)
    }

    fn get_stake(e: Env, user: Address) -> Stake {
        get_stake(&e, &user)
    }

    fn get_optimal_utilization(e: Env) -> u32 {
        get_optimal_utilization(&e)
    }

    fn get_utilization(e: Env) -> u32 {
        let insurance_vault_amount = get_insurance_vault_amount(&e);
        let optimal_coverage = get_optimal_coverage(&e);
        calculate_utilization(insurance_vault_amount, optimal_coverage)
    }

    fn get_rate(e: Env) -> i32 {
        let optimal_utilization = get_optimal_utilization(&e);
        let base_rate = get_base_rate(&e);

        let insurance_vault_amount = get_insurance_vault_amount(&e);
        let optimal_coverage = get_optimal_coverage(&e);
        let utilization = calculate_utilization(insurance_vault_amount, optimal_coverage);

        let (slope1, slope2) = (get_rate_slope_a(&e), get_rate_slope_b(&e));

        calculate_rate(&e, utilization, optimal_utilization, base_rate, slope1, slope2)
    }

    fn get_base_rate(e: Env) -> i32 {
        get_base_rate(&e)
    }

    fn get_rate_slopes(e: Env) -> (i32, i32) {
        (get_rate_slope_a(&e), get_rate_slope_b(&e))
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
    fn set_unstaking_period(e: Env, admin: Address, unstaking_period: u64) {
        admin.require_auth();
        require_admin(&e, &admin);

        set_unstaking_period(&e, &unstaking_period);
    }

    // Sets the max shares the Insurance Fund can have.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    // * `optimal_coverage` - The max number of shares.
    fn set_optimal_coverage(e: Env, admin: Address, optimal_coverage: u128) {
        admin.require_auth();
        require_admin(&e, &admin);

        set_optimal_coverage(&e, &optimal_coverage);
    }

    fn set_coverage_buffer(e: Env, admin: Address, coverage_buffer: u128) {
        admin.require_auth();
        require_admin(&e, &admin);

        set_coverage_buffer(&e, &coverage_buffer);
    }

    fn set_rate_config(
        e: Env,
        admin: Address,
        optimal_utilization: u32,
        base_rate: i32,
        rate_slope_a: i32,
        rate_slope_b: i32
    ) {
        admin.require_auth();
        require_admin(&e, &admin);

        set_optimal_utilization(&e, &optimal_utilization);
        set_base_rate(&e, &base_rate);
        set_rate_slope_a(&e, &rate_slope_a);
        set_rate_slope_b(&e, &rate_slope_b);
    }

    // Sets the max shares the Insurance Fund can have.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    // * `max_shares` - The max number of shares.ƒ
    fn resolve_liquidity_deficit(e: Env, admin: Address, pool_address: Address) {
        admin.require_auth();
        require_admin(&e, &admin);

        let insurance_vault_amount = get_insurance_vault_amount(&e);

        let pay_from_insurance: u128 = e.invoke_contract(
            &pool_address,
            &Symbol::new(&e, "pay_insurance_claim"),
            Vec::from_array(&e, [
                e.current_contract_address().to_val(),
                insurance_vault_amount.into_val(&e),
            ])
        );

        if pay_from_insurance > 0 {
            // "Insurance Fund balance InsufficientCollateral for payment: !{} < {}",
            validate!(
                &e,
                pay_from_insurance < insurance_vault_amount,
                InsuranceFundError::InsufficientCollateral
            );

            let new_insurance_vault_amount = get_insurance_vault_amount(&e);
            // "insurance_fund_vault_amount must remain > 0"
            validate!(&e, new_insurance_vault_amount > 0, InsuranceFundError::InvalidIFDetected);
        }

        // TODO: add event
    }

    // Stops the insurance fund deposits instantly.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    fn kill_deposit(e: Env, admin: Address) {
        admin.require_auth();
        require_admin(&e, &admin);

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
        require_admin(&e, &admin);

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
        require_admin(&e, &admin);

        set_is_killed_withdraw(&e, &true);
        FundEvents::new(&e).kill_withdraw();
    }

    // Resumes insurance fund deposits.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    fn unkill_deposit(e: Env, admin: Address) {
        admin.require_auth();
        require_admin(&e, &admin);

        set_is_killed_deposit(&e, &false);
        FundEvents::new(&e).unkill_deposit();
    }

    // Resumes insurance fund withdrawal requests.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    fn unkill_request_withdraw(e: Env, admin: Address) {
        admin.require_auth();
        require_admin(&e, &admin);

        set_is_killed_request_withdraw(&e, &false);
        FundEvents::new(&e).unkill_request_withdraw();
    }

    // Resumes insurance fund withdrawals.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    fn unkill_withdraw(e: Env, admin: Address) {
        admin.require_auth();
        require_admin(&e, &admin);

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
            0 =>
                match access_control.get_role_safe(&role) {
                    Some(address) => address,
                    None => panic_with_error!(&e, AccessControlError::RoleNotFound),
                }
            _ => access_control.get_future_address(&role),
        }
    }
}
