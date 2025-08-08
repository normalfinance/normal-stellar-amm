use crate::errors::InsuranceFundError;
use crate::events::Events as FundEvents;
use crate::events::InsuranceFundEvents;
use crate::interest::calculate_rate;
use crate::interest::calculate_utilization;
use crate::interface::{AdminInterface, InsuranceFundTrait};
use crate::stake::Stake;
use crate::stake::{
    apply_rebase_to_insurance_fund, apply_rebase_to_stake, calculate_if_shares_lost, get_stake,
    if_shares_to_vault_amount, save_stake, vault_amount_to_if_shares, StakeAction,
};
use crate::storage::{
    get_base_rate, get_insurance_vault_amount, get_is_killed_deposit,
    get_is_killed_request_withdraw, get_is_killed_withdraw, get_optimal_insurance,
    get_optimal_utilization, get_rate_slope_a, get_rate_slope_b, get_shares_base, get_token,
    get_total_shares, get_unstaking_period, set_base_rate, set_is_killed_deposit,
    set_is_killed_request_withdraw, set_is_killed_withdraw, set_optimal_insurance,
    set_optimal_utilization, set_rate_slope_a, set_rate_slope_b, set_token, set_total_shares,
    set_unstaking_period,
};

use access_control::access::{AccessControl, AccessControlTrait};
use access_control::emergency::{get_emergency_mode, set_emergency_mode};
use access_control::errors::AccessControlError;
use access_control::events::Events as AccessControlEvents;
use access_control::interface::TransferableContract;
use access_control::management::SingleAddressManagementTrait;
use access_control::role::{Role, SymbolRepresentation};
use access_control::transfer::TransferOwnershipTrait;
use access_control::utils::require_admin;
use soroban_sdk::contractmeta;
use soroban_sdk::{
    contract, contractimpl, panic_with_error, Address, BytesN, Env, IntoVal, Symbol, Vec,
};
use upgrade::events::Events as UpgradeEvents;
use upgrade::interface::UpgradeableContract;
use upgrade::{apply_upgrade, commit_upgrade, revert_upgrade};
use utils::math::safe_math::SafeMath;
use utils::token::transfer_token;
use utils::validate;
use utils::validation::validate_percentages;

contractmeta!(
    key = "Description",
    val = "Junior tranche (last payout) backstop fund to cover pool liquidity deficits using user staked funds"
);

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
        optimal_utilization: u32,
        base_rate: i32,
        rate_slopes: (u32, u32),
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
        set_optimal_utilization(&e, &optimal_utilization);
        set_base_rate(&e, &base_rate);
        set_rate_slope_a(&e, &rate_slopes.0);
        set_rate_slope_b(&e, &rate_slopes.1);
    }

    //  ___      ___       __        __    _____  ___
    // |"  \    /"  |     /""\      |" \  (\"   \|"  \
    //  \   \  //   |    /    \     ||  | |.\\   \    |
    //  /\\  \/.    |   /' /\  \    |:  | |: \.   \\  |
    // |: \.        |  //  __'  \   |.  | |.  \    \. |
    // |.  \    /:  | /   /  \\  \  /\  |\|    \    \ |
    // |___|\__/|___|(___/    \___)(__\_|_)\___|\____\)

    // Deposits tokens into the Insurance Fund in exchange for share tokens representing stake ownership.
    //
    // This function allows a user to contribute liquidity to the Insurance Fund, which can be used to
    // cover deficits in associated pools. The user receives fund shares proportional to the deposit amount.
    //
    // # Arguments
    // * `e` - The Soroban environment.
    // * `user` - The address of the user making the deposit.
    // * `amount` - The number of tokens to deposit into the fund.
    //
    // # Behavior
    // * Validates that deposits are not paused (`PoolDepositKilled`).
    // * Ensures no outstanding withdrawal request is in progress for the user.
    // * Applies rebasing logic to sync fund and stake accounting with vault state.
    // * Prevents deposits that would push the fund beyond its optimal insurance target.
    // * Mints new share tokens to the user proportional to the deposit.
    // * Records the updated cost basis and share amount for the user’s stake.
    // * Emits an `if_stake_record` event documenting the deposit action.
    //
    // # Side Effects
    // * Updates total shares in the fund.
    // * Transfers `amount` tokens from the user to the Insurance Fund vault.
    // * Saves the updated stake record to storage.
    //
    // # Panics
    // * If deposits are currently disabled (`FundDepositKilled`).
    // * If user has a pending withdrawal request.
    // * If the deposit exceeds the configured optimal insurance capacity.
    fn deposit(e: Env, user: Address, amount: u128) {
        user.require_auth();

        if get_is_killed_deposit(&e) {
            panic_with_error!(e, InsuranceFundError::FundDepositKilled);
        }

        let now = e.ledger().timestamp();

        // TODO: Automically update optimal insurance instead of relying on manual admin updates

        let optimal_insurance = get_optimal_insurance(&e);
        let insurance_vault_amount = get_insurance_vault_amount(&e);

        // Ensure the new stake will not exceed the optimal insurance
        validate!(
            e,
            insurance_vault_amount + amount <= optimal_insurance,
            InsuranceFundError::TooMuchInsurance
        );

        let mut stake = get_stake(&e, &user);

        // Error if a withdrawal request is in progress
        validate!(
            &e,
            stake.last_withdraw_request_shares == 0 && stake.last_withdraw_request_value == 0,
            InsuranceFundError::IFWithdrawRequestInProgress
        );

        // Rebase Insurance Fund and Stake
        apply_rebase_to_insurance_fund(&e, insurance_vault_amount);
        apply_rebase_to_stake(&e, &mut stake);

        // Get updated total shares after rebase
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
            user.clone(),
            StakeAction::Deposit,
            amount,
            insurance_vault_amount,
            if_shares_before,
            total_if_shares_before,
            if_shares_after,
            new_total_shares,
        );

        save_stake(&e, &user, &stake);

        transfer_token(
            &e,
            &get_token(&e),
            &user,
            &e.current_contract_address(),
            &(amount as i128),
        );
    }

    // Initiates a withdrawal request from the Insurance Fund by locking a portion of the user's shares.
    //
    // This function allows a user to signal intent to withdraw a specific amount of tokens from the fund.
    // The request must later be settled through a separate withdrawal execution mechanism.
    //
    // # Arguments
    // * `e` - The Soroban environment.
    // * `user` - The address of the user making the withdrawal request.
    // * `amount` - The token-denominated amount the user wishes to withdraw.
    //
    // # Behavior
    // * Ensures withdrawals are not currently disabled (`FundRequestWithdrawKilled`).
    // * Validates that no other withdrawal request is already pending for the user.
    // * Converts the requested token amount into fund shares (`n_shares`) based on the current vault state.
    // * Verifies the user has enough shares to fulfill the withdrawal.
    // * Rebases both the global insurance fund and the individual stake to reflect up-to-date state.
    // * Computes the withdrawable vault value from the requested shares.
    // * Records the withdrawal request and timestamp in the user’s stake.
    // * Emits a `WithdrawRequest` event to signal the pending unstake.
    //
    // # Side Effects
    // * Locks `n_shares` in the user’s stake as a withdrawal request.
    // * Updates the value of the withdrawal in vault token terms.
    // * Rebases internal accounting to ensure accurate conversion between shares and vault assets.
    //
    // # Panics / Errors
    // * If withdrawals are disabled via kill switch.
    // * If the user already has a pending withdrawal request.
    // * If the requested amount is too small to result in shares.
    // * If the user has insufficient shares to cover the request.
    // * If rebase information is inconsistent or invalid (e.g., mismatched base).
    // * If calculated withdrawal value exceeds the vault balance.
    fn request_withdraw(e: Env, user: Address, amount: u128) {
        user.require_auth();

        if get_is_killed_request_withdraw(&e) {
            panic_with_error!(e, InsuranceFundError::FundRequestWithdrawKilled);
        }

        let now = e.ledger().timestamp();
        let mut stake = get_stake(&e, &user);

        // Error if a withdraw request is already in progress
        validate!(
            &e,
            stake.last_withdraw_request_shares == 0,
            InsuranceFundError::IFWithdrawRequestInProgress
        );

        // Convert token amount to # of shares
        let total_shares = get_total_shares(&e);
        let insurance_vault_amount = get_insurance_vault_amount(&e);
        let n_shares = vault_amount_to_if_shares(&e, amount, total_shares, insurance_vault_amount);

        validate!(
            &e,
            n_shares > 0,
            InsuranceFundError::IFWithdrawRequestTooSmall
        );

        // Error if user does not have enough shares to satisfy the request
        let user_if_shares = stake.checked_if_shares(&e);
        validate!(
            &e,
            user_if_shares >= n_shares,
            InsuranceFundError::InsufficientIFShares
        );

        // Update the user stake
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
        validate!(
            &e,
            stake.if_base == shares_base,
            InsuranceFundError::InvalidIFRebase
        );

        stake.last_withdraw_request_value = if_shares_to_vault_amount(
            &e,
            stake.last_withdraw_request_shares,
            total_shares,
            insurance_vault_amount,
        )
        .min(insurance_vault_amount.saturating_sub(1));

        //  "Requested withdraw value is not below Insurance Fund balance"
        validate!(
            &e,
            stake.last_withdraw_request_value == 0
                || stake.last_withdraw_request_value < insurance_vault_amount,
            InsuranceFundError::InvalidIFUnstakeSize
        );

        let if_shares_after = stake.checked_if_shares(&e);

        FundEvents::new(&e).if_stake_record(
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

    // Cancels a pending withdrawal request from the Insurance Fund for a given user.
    //
    // This function allows a user to cancel their previously initiated withdrawal request.
    // The shares previously marked for withdrawal are burned (i.e., forfeited) to discourage misuse.
    //
    // # Arguments
    // * `e` - The Soroban environment.
    // * `user` - The address of the user cancelling their withdrawal request.
    //
    // # Behavior
    // * Requires the caller to be authorized as `user`.
    // * Verifies that a withdrawal request is currently active.
    // * Applies any pending rebases to both the global Insurance Fund and the user’s stake.
    // * Validates consistency between the stake's internal rebase base and the global base.
    // * Calculates the number of shares to be forfeited due to cancellation (i.e., `if_shares_lost`).
    // * Burns these shares from the user and adjusts the total share supply accordingly.
    // * Clears the withdrawal request metadata (shares, value, and timestamp).
    // * Emits a `WithdrawCancelRequest` event to log the forfeiture and updated state.
    //
    // # Side Effects
    // * Reduces user’s stake and total share supply by the calculated penalty (`if_shares_lost`).
    // * Resets withdrawal request fields on the user’s stake.
    //
    // # Panics / Errors
    // * If no withdrawal request is currently in progress.
    // * If rebase metadata is inconsistent (e.g., base mismatch).
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
        validate!(
            &e,
            stake.if_base == shares_base,
            InsuranceFundError::InvalidIFRebase
        );

        let if_shares_lost = calculate_if_shares_lost(&e, &stake, insurance_vault_amount);

        stake.decrease_if_shares(&e, if_shares_lost);

        set_total_shares(&e, &total_shares.safe_sub(&e, if_shares_lost));

        let if_shares_after = stake.checked_if_shares(&e);

        FundEvents::new(&e).if_stake_record(
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

        save_stake(&e, &user, &stake);
    }

    // Completes a pending Insurance Fund withdrawal request after the unstaking period has elapsed.
    //
    // This function finalizes a user's withdrawal request by transferring the corresponding
    // token amount from the Insurance Fund to the user. The user must have previously submitted
    // a withdrawal request and waited the required escrow duration.
    //
    // # Arguments
    // * `e` - The Soroban environment.
    // * `user` - The address initiating the withdrawal.
    //
    // # Behavior
    // * Requires authorization from the `user`.
    // * Ensures the Insurance Fund is not currently paused for withdrawals.
    // * Validates that the required unstaking period has passed since the withdrawal request.
    // * Applies pending rebases to both the global fund and the user's stake.
    // * Confirms the user has enough IF shares to satisfy the request.
    // * Calculates the corresponding withdrawal amount, applying loss penalties if applicable.
    // * Reduces the user’s IF shares and the global share count accordingly.
    // * Resets the user's withdrawal request state.
    // * Emits a `Withdraw` event recording the withdrawal details.
    // * Transfers the calculated amount to the user’s address.
    // * Ensures the vault still retains a non-zero balance after withdrawal.
    //
    // # Side Effects
    // * Decreases user’s stake and cost basis.
    // * Reduces the total IF shares in circulation.
    // * Resets the withdrawal request metadata on the stake.
    //
    // # Panics / Errors
    // * If withdrawal operations are currently disabled.
    // * If no withdrawal request exists or the waiting period hasn't passed.
    // * If stake state or rebase base is invalid.
    // * If the vault balance would be zero after withdrawal.
    fn withdraw(e: Env, user: Address) {
        user.require_auth();

        if get_is_killed_withdraw(&e) {
            panic_with_error!(e, InsuranceFundError::FundWithdrawKilled);
        }

        // TODO: Do we need to check IF utilization and/or overall pool liquidity imbalance for edge
        // cases before authorizing a withdrawal?

        let now = e.ledger().timestamp();
        let mut stake = get_stake(&e, &user);

        let time_since_withdraw_request = now.safe_sub(&e, stake.last_withdraw_request_ts);

        // Error if the unstaking period has not yet elapsed
        validate!(
            &e,
            time_since_withdraw_request >= get_unstaking_period(&e),
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

        validate!(
            &e,
            if_shares_before >= n_shares,
            InsuranceFundError::InsufficientIFShares
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
            user.clone(),
            StakeAction::Withdraw,
            withdraw_amount,
            insurance_vault_amount,
            if_shares_before,
            total_if_shares_before,
            if_shares_after,
            total_shares,
        );

        save_stake(&e, &user, &stake);

        transfer_token(
            &e,
            &get_token(&e),
            &e.current_contract_address(),
            &user,
            &(withdraw_amount as i128),
        );

        let insurance_vault_amount = get_insurance_vault_amount(&e);
        // "insurance_fund_vault.amount must remain > 0"
        validate!(
            &e,
            insurance_vault_amount > 0,
            InsuranceFundError::InvalidIFDetected
        );
    }

    // Collects a premium payment from a pool or protocol participant into the Insurance Fund.
    //
    // This function is typically called by a liquidity pool when a swap occurs and a portion of
    // the collected swap fee is directed to the Insurance Fund as an insurance premium.
    // However, it remains open to calls from other authorized protocol components to support
    // future use cases such as protocol fee sharing or external revenue streams.
    //
    // # Arguments
    // * `e` - The Soroban environment.
    // * `sender` - The address paying the premium.
    // * `amount` - The amount of tokens to be transferred as a premium payment.
    //
    // # Behavior
    // * Requires authorization from the `sender`.
    // * Transfers the specified `amount` of tokens from the `sender` to the Insurance Fund.
    // * Emits a `collect_premium` event recording the payment details.
    //
    // # Security
    // * This function is intentionally left unrestricted to allow multiple protocol actors
    //   to contribute to the Insurance Fund's premium pool. Callers must still authenticate
    //   as the `sender` to authorize the transfer.
    //
    // # Use Cases
    // * Primary usage is by Pools via `PoolSwapFee::swap()` to remit premium.
    // * Future protocol-level functions or treasury logic may also call this directly.
    //
    // # Panics / Errors
    // * If the `sender` fails to authenticate.
    fn pay_premium(e: Env, sender: Address, amount: u128) {
        sender.require_auth();

        /* @Halborn
        The `Pool.insurance_claim` property defines how much coverage each Pool
        receives from the Insurance Fund. Pools pay a premium for this insurance
        via a portion of swap fees as defined in `PoolSwapFee.swap()` - where this
        function is invoked to pay premiums.

        Access to this function has been left open (not restricted to only the
        PoolSwapFee contract) to allow other methods of protocol revenue to
        eventually contribute to premium payments.
         */

        transfer_token(
            &e,
            &get_token(&e),
            &sender,
            &e.current_contract_address(),
            &(amount as i128),
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

    fn get_optimal_insurance(e: Env) -> u128 {
        get_optimal_insurance(&e)
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

    // Get the current insurance utilization.
    // Utilazation = current insurance / optimal insurance
    //
    // # Returns
    //
    // The utilization rate as a u32.
    fn get_utilization(e: Env) -> u32 {
        let insurance_vault_amount = get_insurance_vault_amount(&e);
        let optimal_insurance = get_optimal_insurance(&e);
        calculate_utilization(insurance_vault_amount, optimal_insurance)
    }

    // Get the current staking interest rate.
    // Similar implementation to Aave v3 (https://aave.com/docs/developers/smart-contracts/interest-rate-strategy).
    //
    // # Returns
    //
    // The staking interest rate as an i32.
    fn get_rate(e: Env) -> i32 {
        let optimal_utilization = get_optimal_utilization(&e);
        let base_rate = get_base_rate(&e);

        let insurance_vault_amount = get_insurance_vault_amount(&e);
        let optimal_insurance = get_optimal_insurance(&e);
        let utilization = calculate_utilization(insurance_vault_amount, optimal_insurance);

        let (slope1, slope2) = (get_rate_slope_a(&e), get_rate_slope_b(&e));

        calculate_rate(utilization, optimal_utilization, base_rate, slope1, slope2)
    }

    fn get_base_rate(e: Env) -> i32 {
        get_base_rate(&e)
    }

    fn get_rate_slopes(e: Env) -> (u32, u32) {
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
impl AdminInterface for InsuranceFund {
    //  ___      ___       __        __    _____  ___
    // |"  \    /"  |     /""\      |" \  (\"   \|"  \
    //  \   \  //   |    /    \     ||  | |.\\   \    |
    //  /\\  \/.    |   /' /\  \    |:  | |: \.   \\  |
    // |: \.        |  //  __'  \   |.  | |.  \    \. |
    // |.  \    /:  | /   /  \\  \  /\  |\|    \    \ |
    // |___|\__/|___|(___/    \___)(__\_|_)\___|\____\)

    // Resolves a liquidity deficit in a pool by transferring insurance coverage from the Insurance Fund.
    //
    // This function is invoked by the Insurance Fund admin when a liquidity pool reports a deficit
    // (e.g. due to under-collateralization or volatile price movements). It calls into the pool
    // contract’s `pay_insurance_claim` method, which computes and deducts the insurance coverage needed.
    //
    // # Arguments
    // * `e` - The Soroban environment.
    // * `admin` - The address authorized to trigger the resolution.
    // * `pool_address` - The contract address of the affected pool requesting coverage.
    //
    // # Behavior
    // * Requires admin authentication.
    // * Queries the current balance of the Insurance Fund (`insurance_vault_amount`).
    // * Invokes `pay_insurance_claim()` on the target pool, passing the available vault amount.
    // * Validates that the claim does not exceed the available balance.
    // * Validates that the Insurance Fund retains a non-zero balance after the payout.
    //
    // # Security
    // * Restricted to the Insurance Fund admin (currently centralized control).
    // * Future roadmap: automate via `Pool::swap()` or delegate authority to a DAO.
    //
    // # Panics / Errors
    // * `InsuranceFundError::InsufficientCollateral` if the claim exceeds vault balance.
    // * `InsuranceFundError::InvalidIFDetected` if the payout fully depletes the Insurance Fund.
    fn resolve_liquidity_deficit(e: Env, admin: Address, pool_address: Address) {
        admin.require_auth();
        /* Currently, only the Insurance Fund admin may resolve deficits, however, our goal
        is to either: a) automate within `Pool.swap()` itself; or b) decentralize via the Normal DAO */
        require_admin(&e, &admin);

        let insurance_vault_amount = get_insurance_vault_amount(&e);

        // Call `Pool.pay_insurance_claim()` to calculate how much insurance is needed
        // and to update the `Pool.insurance_claim`
        let pay_from_insurance: u128 = e.invoke_contract(
            &pool_address,
            &Symbol::new(&e, "pay_insurance_claim"),
            Vec::from_array(
                &e,
                [
                    e.current_contract_address().to_val(),
                    insurance_vault_amount.into_val(&e),
                ],
            ),
        );

        if pay_from_insurance > 0 {
            // Error if there is not enough insurance to cover the claim
            validate!(
                &e,
                pay_from_insurance < insurance_vault_amount,
                InsuranceFundError::InsufficientCollateral
            );

            // Error if a claim leaves removes all insurance
            let new_insurance_vault_amount = get_insurance_vault_amount(&e);
            validate!(
                &e,
                new_insurance_vault_amount > 0,
                InsuranceFundError::InvalidIFDetected
            );
        }
    }

    //   ________  _______  ___________  ___________  _______   _______    ________
    //  /"       )/"     "|("     _   ")("     _   ")/"     "| /"      \  /"       )
    // (:   \___/(: ______) )__/  \\__/  )__/  \\__/(: ______)|:        |(:   \___/
    //  \___  \   \/    |      \\_ /        \\_ /    \/    |  |_____/   ) \___  \
    //   __/  \\  // ___)_     |.  |        |.  |    // ___)_  //      /   __/  \\
    //  /" \   :)(:      "|    \:  |        \:  |   (:      "||:  __   \  /" \   :)
    // (_______/  \_______)     \__|         \__|    \_______)|__|  \___)(_______/

    fn set_unstaking_period(e: Env, admin: Address, unstaking_period: u64) {
        admin.require_auth();
        require_admin(&e, &admin);

        set_unstaking_period(&e, &unstaking_period);
    }

    fn set_optimal_insurance(e: Env, admin: Address, optimal_insurance: u128) {
        admin.require_auth();
        require_admin(&e, &admin);

        set_optimal_insurance(&e, &optimal_insurance);
    }

    fn set_rate_config(
        e: Env,
        admin: Address,
        optimal_utilization: u32,
        base_rate: i32,
        rate_slope_a: u32,
        rate_slope_b: u32,
    ) {
        admin.require_auth();
        require_admin(&e, &admin);

        validate_percentages(
            &e,
            &Vec::from_array(
                &e,
                [
                    optimal_utilization as i32,
                    base_rate,
                    rate_slope_a as i32,
                    rate_slope_b as i32,
                ],
            ),
        );

        set_optimal_utilization(&e, &optimal_utilization);
        set_base_rate(&e, &base_rate);
        set_rate_slope_a(&e, &rate_slope_a);
        set_rate_slope_b(&e, &rate_slope_b);
    }

    //    _______     __       ____  ____   ________  _______  ________
    //   |   __ "\   /""\     ("  _||_ " | /"       )/"     "||"      "\
    //   (. |__) :) /    \    |   (  ) : |(:   \___/(: ______)(.  ___  :)
    //   |:  ____/ /' /\  \   (:  |  | . ) \___  \   \/    |  |: \   ) ||
    //   (|  /    //  __'  \   \\ \__/ //   __/  \\  // ___)_ (| (___\ ||
    //  /|__/ \  /   /  \\  \  /\\ __ //\  /" \   :)(:      "||:       :)
    // (_______)(___/    \___)(__________)(_______/  \_______)(________/

    fn kill_deposit(e: Env, admin: Address) {
        admin.require_auth();
        require_admin(&e, &admin);

        set_is_killed_deposit(&e, &true);
        FundEvents::new(&e).kill_deposit();
    }

    fn kill_request_withdraw(e: Env, admin: Address) {
        admin.require_auth();
        require_admin(&e, &admin);

        set_is_killed_request_withdraw(&e, &true);
        FundEvents::new(&e).kill_request_withdraw();
    }

    fn kill_withdraw(e: Env, admin: Address) {
        admin.require_auth();
        require_admin(&e, &admin);

        set_is_killed_withdraw(&e, &true);
        FundEvents::new(&e).kill_withdraw();
    }

    fn unkill_deposit(e: Env, admin: Address) {
        admin.require_auth();
        require_admin(&e, &admin);

        set_is_killed_deposit(&e, &false);
        FundEvents::new(&e).unkill_deposit();
    }

    fn unkill_request_withdraw(e: Env, admin: Address) {
        admin.require_auth();
        require_admin(&e, &admin);

        set_is_killed_request_withdraw(&e, &false);
        FundEvents::new(&e).unkill_request_withdraw();
    }

    fn unkill_withdraw(e: Env, admin: Address) {
        admin.require_auth();
        require_admin(&e, &admin);

        set_is_killed_withdraw(&e, &false);
        FundEvents::new(&e).unkill_withdraw();
    }

    fn get_is_killed_deposit(e: Env) -> bool {
        get_is_killed_deposit(&e)
    }

    fn get_is_killed_request_withdraw(e: Env) -> bool {
        get_is_killed_request_withdraw(&e)
    }

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
