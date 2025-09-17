use crate::errors::InsuranceFundError;
use crate::events::Events as FundEvents;
use crate::events::InsuranceFundEvents;
use crate::interest::calculate_rate;
use crate::interest::calculate_total_reserve_value;
use crate::interest::calculate_utilization;
use crate::interface::{AdminInterface, InsuranceFundTrait};
use crate::reserve;
use crate::reserve::InsuranceFundReserve;
use crate::stake::Stake;
use crate::stake::{
    apply_rebase_to_insurance_fund, apply_rebase_to_stake, calculate_shares_lost, get_stake,
    reserve_amount_to_shares, save_stake, shares_to_reserve_amount, StakeAction,
};
use crate::storage::get_contract_token_balance;
use crate::storage::get_oracle_registry;
use crate::storage::get_pool_router;
use crate::storage::get_premium_payer_status;
use crate::storage::get_premium_token;
use crate::storage::get_reserve;
use crate::storage::get_token_whitelist;
use crate::storage::get_token_whitelist_status;
use crate::storage::get_token_whitelist_vec;
use crate::storage::put_reserve;
use crate::storage::remove_token_whitelist;
use crate::storage::set_oracle_registry;
use crate::storage::set_pool_router;
use crate::storage::set_premium_payer_status;
use crate::storage::set_premium_token;
use crate::storage::set_token_whitelist;
use crate::storage::set_token_whitelist_vec;
use crate::storage::WhitelistToken;
use crate::storage::{
    get_base_rate, get_is_killed_deposit, get_is_killed_request_withdraw, get_is_killed_withdraw,
    get_optimal_insurance, get_optimal_utilization, get_rate_slope_a, get_rate_slope_b,
    get_unstaking_period, set_base_rate, set_is_killed_deposit, set_is_killed_request_withdraw,
    set_is_killed_withdraw, set_optimal_insurance, set_optimal_utilization, set_rate_slope_a,
    set_rate_slope_b, set_unstaking_period,
};
use reentrancy_guard::{enter, exit};

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
use utils::state::pool::PoolInfo;
use utils::token::transfer_token;
use utils::token::validate_token_contract;
use utils::validate;
use utils::validation::ensure_non_zero_u128;
use utils::validation::validate_percentages;

contractmeta!(
    key = "Description",
    val = "Backstop fund to cover pool liquidity deficits"
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
        oracle_registry: Address,
        pool_router: Address,
        premium_token: Address,
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

        set_oracle_registry(&e, &oracle_registry);
        set_pool_router(&e, &pool_router);

        validate_token_contract(&e, &premium_token);
        set_premium_token(&e, &premium_token);

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
    // * `token` -
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
    fn deposit(e: Env, user: Address, token: Address, amount: u128) {
        user.require_auth();

        // Validations
        validate_token_contract(&e, &token);
        ensure_non_zero_u128(&e, amount);

        // Re-entrancy check
        enter(&e);

        // Status check
        if get_is_killed_deposit(&e) {
            panic_with_error!(e, InsuranceFundError::FundDepositKilled);
        }

        // Whitelisted token check
        if !get_token_whitelist_status(&e, &token) {
            panic_with_error!(e, InsuranceFundError::UnsupportedToken);
        }

        let now = e.ledger().timestamp();
        let optimal_insurance = get_optimal_insurance(&e);

        let mut reserve = get_reserve(&e, &token);
        let reserve_balance_before = reserve.balance;

        // Ensure the new stake will not exceed the optimal insurance
        validate!(
            e,
            reserve_balance_before + amount <= optimal_insurance,
            InsuranceFundError::TooMuchInsurance
        );

        let mut stake = get_stake(&e, &user, &token);

        // Error if a withdrawal request is already in progress
        validate!(
            &e,
            stake.last_withdraw_request_shares == 0 && stake.last_withdraw_request_value == 0,
            InsuranceFundError::IFWithdrawRequestInProgress
        );

        // Rebase the Insurance Fund and Stake
        apply_rebase_to_insurance_fund(&e, &mut reserve);
        apply_rebase_to_stake(&e, &mut stake);

        let stake_shares_before = stake.checked_shares(&e);
        let total_shares_before = reserve.total_shares;

        let n_shares = reserve_amount_to_shares(&e, amount, &reserve);

        // Reset cost basis if no shares
        stake.cost_basis = if stake_shares_before == 0 {
            amount
        } else {
            stake.cost_basis.safe_add(&e, amount)
        };

        // Increase the Fund and Stake shares
        stake.increase_shares(&e, n_shares);
        reserve.add_total_shares(&e, n_shares, now);

        // Update the Reserve and Stake
        reserve.save(&e);
        stake.save(&e);

        // Deposit tokens from the user to the Fund
        transfer_token(
            &e,
            &token,
            &user,
            &e.current_contract_address(),
            &(amount as i128),
        );

        FundEvents::new(&e).insurance_stake_record(
            user.clone(),
            token.clone(),
            StakeAction::Deposit,
            amount,
            reserve_balance_before,
            stake_shares_before,
            total_shares_before,
            stake.shares,
            reserve.total_shares,
        );

        exit(&e);
    }

    // Initiates a withdrawal request from the Insurance Fund by locking a portion of the user's shares.
    //
    // This function allows a user to signal intent to withdraw a specific amount of tokens from the fund.
    // The request must later be settled through a separate withdrawal execution mechanism.
    //
    // # Arguments
    // * `e` - The Soroban environment.
    // * `user` - The address of the user making the withdrawal request.
    // * `token` -
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
    fn request_withdraw(e: Env, user: Address, token: Address, amount: u128) {
        user.require_auth();

        // Validations
        validate_token_contract(&e, &token);
        ensure_non_zero_u128(&e, amount);

        // Re-entrancy check
        enter(&e);

        // Status check
        if get_is_killed_request_withdraw(&e) {
            panic_with_error!(e, InsuranceFundError::FundRequestWithdrawKilled);
        }

        // Withdrawals do not require a whitelist token to be active. Inactive tokens
        // must be withdrawn so the reserve.balance equals zero before they can be removed.
        get_token_whitelist(&e, &token);

        let now = e.ledger().timestamp();
        let mut stake = get_stake(&e, &user, &token);

        // Error if a withdraw request is already in progress
        validate!(
            &e,
            stake.last_withdraw_request_shares == 0,
            InsuranceFundError::IFWithdrawRequestInProgress
        );

        let mut reserve = get_reserve(&e, &token);
        let reserve_balance_before = reserve.balance;

        // Convert token amount to # of shares
        let n_shares = reserve_amount_to_shares(&e, amount, &reserve);

        validate!(
            &e,
            n_shares > 0,
            InsuranceFundError::IFWithdrawRequestTooSmall
        );

        // Error if user does not have enough shares to satisfy the request
        let stake_shares = stake.checked_shares(&e);
        validate!(
            &e,
            stake_shares >= n_shares,
            InsuranceFundError::InsufficientIFShares
        );

        // Update the Stake
        stake.last_withdraw_request_shares = n_shares;

        // Rebase the Insurance Fund and Stake
        apply_rebase_to_insurance_fund(&e, &mut reserve);
        apply_rebase_to_stake(&e, &mut stake);

        let stake_shares_before = stake.checked_shares(&e);
        let total_shares_before = reserve.total_shares;

        validate!(
            &e,
            stake.last_withdraw_request_shares <= stake.checked_shares(&e),
            InsuranceFundError::InvalidInsuranceUnstakeSize
        );

        validate!(
            &e,
            stake.base == reserve.shares_base,
            InsuranceFundError::InvalidIFRebase
        );

        stake.last_withdraw_request_value =
            shares_to_reserve_amount(&e, stake.last_withdraw_request_shares, &reserve)
                .min(reserve.balance.safe_sub(&e, 1));

        validate!(
            &e,
            stake.last_withdraw_request_value == 0
                || stake.last_withdraw_request_value < reserve.balance,
            InsuranceFundError::InvalidIFUnstakeSize
        );

        stake.last_withdraw_request_ts = now;

        // Update the Reserve and Stake
        reserve.save(&e);
        stake.save(&e);

        FundEvents::new(&e).insurance_stake_record(
            user.clone(),
            token.clone(),
            StakeAction::WithdrawRequest,
            stake.last_withdraw_request_value,
            reserve_balance_before,
            stake_shares_before,
            total_shares_before,
            stake.shares,
            reserve.total_shares,
        );

        exit(&e);
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
    fn cancel_request_withdraw(e: Env, user: Address, token: Address) {
        user.require_auth();

        // Validations
        validate_token_contract(&e, &token);

        // Re-entrancy check
        enter(&e);

        // Whitelist token check
        get_token_whitelist(&e, &token);

        let now = e.ledger().timestamp();
        let mut stake = get_stake(&e, &user, &token);

        // No withdraw request in progress
        validate!(
            &e,
            stake.last_withdraw_request_shares != 0,
            InsuranceFundError::NoIFWithdrawRequestInProgress
        );

        let mut reserve = get_reserve(&e, &token);
        let reserve_balance_before = reserve.balance;

        // Rebase the Insurance Fund and Stake
        apply_rebase_to_insurance_fund(&e, &mut reserve);
        apply_rebase_to_stake(&e, &mut stake);

        let stake_shares_before = stake.checked_shares(&e);
        let total_shares_before = reserve.total_shares;

        // if stake base != base
        validate!(
            &e,
            stake.base == reserve.shares_base,
            InsuranceFundError::InvalidIFRebase
        );

        // Decrease the Stake shares
        let stake_shares_lost = calculate_shares_lost(&e, &stake, &reserve);

        stake.decrease_shares(&e, stake_shares_lost);

        validate!(
            &e,
            reserve.total_shares >= stake_shares_lost,
            InsuranceFundError::InsufficientIFShares
        );

        // Decrease the Fund shares
        reserve.remove_total_shares(&e, stake_shares_lost, now);

        stake.last_withdraw_request_shares = 0;
        stake.last_withdraw_request_value = 0;
        stake.last_withdraw_request_ts = now;

        // Update the Reserve and Stake
        reserve.save(&e);
        stake.save(&e);

        FundEvents::new(&e).insurance_stake_record(
            user.clone(),
            token.clone(),
            StakeAction::WithdrawCancelRequest,
            0,
            reserve_balance_before,
            stake_shares_before,
            total_shares_before,
            stake.shares,
            reserve.total_shares,
        );

        exit(&e);
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
    fn withdraw(e: Env, user: Address, token: Address) {
        user.require_auth();

        // Validations
        validate_token_contract(&e, &token);

        // Re-entrancy check
        enter(&e);

        // Status check
        if get_is_killed_withdraw(&e) {
            panic_with_error!(e, InsuranceFundError::FundWithdrawKilled);
        }

        get_token_whitelist(&e, &token);

        let now = e.ledger().timestamp();
        let mut stake = get_stake(&e, &user, &token);

        // Add bounds checking to prevent underflow when system clock goes backwards
        validate!(
            &e,
            now >= stake.last_withdraw_request_ts,
            InsuranceFundError::InvalidTimestamp
        );
        let time_since_withdraw_request = now - stake.last_withdraw_request_ts;

        // Error if the unstaking period has not yet elapsed
        validate!(
            &e,
            time_since_withdraw_request >= get_unstaking_period(&e),
            InsuranceFundError::TryingToRemoveLiquidityTooFast
        );

        let mut reserve = get_reserve(&e, &token);
        let reserve_balance_before = reserve.balance;

        // Rebase the Insurance Fund and Stake
        apply_rebase_to_insurance_fund(&e, &mut reserve);
        apply_rebase_to_stake(&e, &mut stake);

        let stake_shares_before = stake.checked_shares(&e);
        let total_shares_before = reserve.total_shares;

        let n_shares = stake.last_withdraw_request_shares;

        // Must submit withdraw request and wait the escrow period
        validate!(&e, n_shares > 0, InsuranceFundError::InvalidIFUnstake);

        validate!(
            &e,
            stake_shares_before >= n_shares,
            InsuranceFundError::InsufficientIFShares
        );

        let amount = shares_to_reserve_amount(&e, n_shares, &reserve);

        let _if_shares_lost = calculate_shares_lost(&e, &stake, &reserve);

        let withdraw_amount = amount.min(stake.last_withdraw_request_value);

        stake.decrease_shares(&e, n_shares);

        // Add bounds checking to prevent underflow when withdrawing more than cost basis
        validate!(
            &e,
            stake.cost_basis >= withdraw_amount,
            InsuranceFundError::CostBasisUnderflow
        );
        stake.cost_basis = stake.cost_basis.safe_sub(&e, withdraw_amount);

        // Add bounds checking to prevent critical share tracking underflow
        validate!(
            &e,
            reserve.total_shares >= n_shares,
            InsuranceFundError::InsufficientIFShares
        );

        reserve.remove_total_shares(&e, n_shares, now);

        // Reset stake withdraw request info
        stake.last_withdraw_request_shares = 0;
        stake.last_withdraw_request_value = 0;
        stake.last_withdraw_request_ts = now;

        // Update the Reserve and Stake
        reserve.save(&e);
        stake.save(&e);

        // Send tokens from the Fund to the user
        transfer_token(
            &e,
            &token,
            &e.current_contract_address(),
            &user,
            &(withdraw_amount as i128),
        );

        FundEvents::new(&e).insurance_stake_record(
            user.clone(),
            token.clone(),
            StakeAction::Withdraw,
            withdraw_amount,
            reserve_balance_before,
            stake_shares_before,
            total_shares_before,
            stake.shares,
            reserve.total_shares,
        );

        // Additional validation
        let new_reserve = get_reserve(&e, &token);
        validate!(
            &e,
            new_reserve.balance > 0,
            InsuranceFundError::InvalidIFDetected
        );

        exit(&e);
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

        ensure_non_zero_u128(&e, amount);

        enter(&e);

        if !get_premium_payer_status(&e, &sender) {
            panic_with_error!(&e, InsuranceFundError::NotAuthorized);
        }

        let premium_token = get_premium_token(&e);
        transfer_token(
            &e,
            &premium_token,
            &sender,
            &e.current_contract_address(),
            &(amount as i128),
        );

        FundEvents::new(&e).collect_premium(sender, premium_token, amount);

        exit(&e);
    }

    // Sync token balances with reserves.
    //
    // # Arguments
    //
    // * `sender` - The address of the sender.
    // * `token` - The address of the token to sync.
    fn sync(e: Env, sender: Address, token: Address) {
        sender.require_auth();

        enter(&e);

        get_token_whitelist(&e, &token);

        let now = e.ledger().timestamp();

        let balance = get_contract_token_balance(&e, &token);

        let mut reserve = get_reserve(&e, &token);
        reserve.update_balance(balance, now);
        reserve.save(&e);

        FundEvents::new(&e).sync(sender, token, 0);

        exit(&e);
    }

    // Skim excess token balances.
    //
    // # Arguments
    //
    // * `sender` - The address of the sender.
    // * `token` - The address of the token to skim.
    fn skim(e: Env, sender: Address, token: Address) {
        sender.require_auth();

        enter(&e);

        get_token_whitelist(&e, &token);

        let reserve = get_reserve(&e, &token);

        let balance = get_contract_token_balance(&e, &token) as i128;
        let skimmed = balance.safe_sub(&e, reserve.balance as i128);

        if skimmed > 0 {
            transfer_token(&e, &token, &e.current_contract_address(), &sender, &skimmed);
            FundEvents::new(&e).skim(sender, token, skimmed);
        }

        exit(&e);
    }

    //   _______    _______  ___________  ___________  _______   _______    ________
    //  /" _   "|  /"     "|("     _   ")("     _   ")/"     "| /"      \  /"       )
    // (: ( \___) (: ______) )__/  \\__/  )__/  \\__/(: ______)|:        |(:   \___/
    //  \/ \       \/    |      \\_ /        \\_ /    \/    |  |_____/   ) \___  \
    //  //  \ ___  // ___)_     |.  |        |.  |    // ___)_  //      /   __/  \\
    // (:   _(  _|(:      "|    \:  |        \:  |   (:      "||:  __   \  /" \   :)
    //  \_______)  \_______)     \__|         \__|    \_______)|__|  \___)(_______/

    fn get_oracle_registry(e: Env) -> Address {
        get_oracle_registry(&e)
    }

    fn get_pool_router(e: Env) -> Address {
        get_pool_router(&e)
    }

    fn get_premium_token(e: Env) -> Address {
        get_premium_token(&e)
    }

    fn get_unstaking_period(e: Env) -> u64 {
        get_unstaking_period(&e)
    }

    fn get_optimal_insurance(e: Env) -> u128 {
        get_optimal_insurance(&e)
    }

    fn get_reserve(e: Env, token: Address) -> InsuranceFundReserve {
        get_reserve(&e, &token)
    }

    fn get_stake(e: Env, user: Address, token: Address) -> Stake {
        get_stake(&e, &user, &token)
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
        let total_reserve_value = calculate_total_reserve_value(&e);
        let optimal_insurance = get_optimal_insurance(&e);

        calculate_utilization(&e, total_reserve_value, optimal_insurance)
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

        let total_reserve_value = calculate_total_reserve_value(&e);
        let optimal_insurance = get_optimal_insurance(&e);
        let utilization = calculate_utilization(&e, total_reserve_value, optimal_insurance);

        let (slope1, slope2) = (get_rate_slope_a(&e), get_rate_slope_b(&e));

        calculate_rate(utilization, optimal_utilization, base_rate, slope1, slope2)
    }

    fn get_base_rate(e: Env) -> i32 {
        get_base_rate(&e)
    }

    fn get_rate_slopes(e: Env) -> (u32, u32) {
        (get_rate_slope_a(&e), get_rate_slope_b(&e))
    }

    fn get_token_whitelist(e: Env, token: Address) -> WhitelistToken {
        get_token_whitelist(&e, &token)
    }

    fn get_premium_payer_status(e: Env, address: Address) -> bool {
        get_premium_payer_status(&e, &address)
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

    // Synchronizes the target “optimal insurance” level with current system-wide liquidity imbalance.
    //
    // This admin-only function queries the Pool Router for the aggregate liquidity imbalance across
    // all pools and updates the Insurance Fund’s `optimal_insurance` value accordingly. The target is
    // set to zero when the system is balanced (≤ 0) and to the positive imbalance otherwise.
    //
    // # Arguments
    // * `e` - The Soroban environment.
    // * `admin` - The address that must authorize the sync operation.
    //
    // # Behavior
    // * Requires admin authentication (`admin.require_auth()` + `require_admin()`).
    // * Reads the current `optimal_insurance`.
    // * Invokes the Pool Router’s `get_total_liquidity_imbalance()` (no args) to fetch the latest
    //   system-wide imbalance as an `i128`.
    // * Computes `updated_optimal_insurance` as:
    //   * `0` if `total_liquidity_imbalance <= 0`,
    //   * `total_liquidity_imbalance as u128` otherwise.
    // * Persists the new `optimal_insurance`.
    // * Emits a `FundEvents::sync_optimal_insurance(admin, old, new)` event for observability.
    //
    // # Rationale
    // * Using `max(0, imbalance)` prevents negative targets and ensures the stored value fits in
    //   `u128`.
    // * Centralizing the read (`Pool Router`) and write (`Insurance Fund`) keeps calculation logic
    //   composable and auditable.
    //
    // # Security
    // * Restricted to the Insurance Fund admin; unauthorized callers are rejected.
    // * The function performs a single external call to the Pool Router; any failure there aborts
    //   the update and leaves state unchanged.
    // * No tokens are moved; this only updates a configuration/target value.
    //
    // # Panics / Errors
    // * Authorization fails if `admin` does not sign or is not recognized by `require_admin()`.
    // * Any error from the Pool Router `get_total_liquidity_imbalance` call is bubbled up.
    // * Storage write errors (e.g., out-of-budget) will abort the transaction.
    //
    // # Events
    // * `sync_optimal_insurance(admin, previous_optimal, updated_optimal)` is emitted after a
    //   successful update to aid indexers and off-chain monitoring.
    fn sync_optimal_insurance(e: Env, admin: Address) {
        admin.require_auth();
        require_admin(&e, &admin);

        let current_optimal_insurance = get_optimal_insurance(&e);

        // Fetch the total liquidity imbalance from the Pool Router
        let total_liquidity_imbalance: i128 = e.invoke_contract(
            &get_pool_router(&e),
            &Symbol::new(&e, "get_total_liquidity_imbalance"),
            Vec::from_array(&e, []),
        );

        let updated_optimal_insurance = if total_liquidity_imbalance <= 0 {
            0_u128
        } else {
            // Safe conversion with bounds checking - prevent silent truncation
            u128::try_from(total_liquidity_imbalance).unwrap_or_else(|_| {
                panic_with_error!(&e, InsuranceFundError::ConversionOverflow);
            })
        };

        set_optimal_insurance(&e, &updated_optimal_insurance);

        FundEvents::new(&e).sync_optimal_insurance(
            admin,
            current_optimal_insurance,
            updated_optimal_insurance,
        );
    }

    // Resolves a liquidity deficit in a pool by transferring insurance coverage from the Insurance Fund.
    //
    // This function is invoked by the Insurance Fund admin when a liquidity pool reports a deficit
    // (e.g. due to under-collateralization or volatile price movements). It calls into the pool
    // contract’s `pay_insurance_claim` method, which computes and deducts the insurance coverage needed.
    //
    // # Arguments
    // * `e` - The Soroban environment.
    // * `admin` - The address authorized to trigger the resolution.
    // * `asset` - The asset of the affected pool requesting coverage.
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
    fn file_claim(e: Env, admin: Address, token: Address, asset: Symbol) {
        admin.require_auth();
        require_admin(&e, &admin);

        enter(&e);

        let pool_details_result = e.try_invoke_contract::<PoolInfo, soroban_sdk::Error>(
            &get_pool_router(&e),
            &Symbol::new(&e, "query_pool_details"),
            Vec::from_array(&e, [asset.into_val(&e)]),
        );

        match pool_details_result {
            Ok(Err(_pool_error)) => {
                panic_with_error!(&e, InsuranceFundError::QueryPoolFailed);
            }
            Err(_contract_error) => {
                panic_with_error!(&e, InsuranceFundError::QueryPoolFailed);
            }
            Ok(Ok(pool_info)) => {
                let reserve = get_reserve(&e, &token);

                // Call `Pool.pay_insurance_claim()` to calculate how much insurance is needed
                // and to update the `Pool.insurance_claim`
                let pay_from_insurance_result = e.try_invoke_contract::<u128, soroban_sdk::Error>(
                    &pool_info.pool_address,
                    &Symbol::new(&e, "pay_insurance_claim"),
                    Vec::from_array(
                        &e,
                        [
                            e.current_contract_address().to_val(),
                            reserve.balance.into_val(&e),
                        ],
                    ),
                );

                match pay_from_insurance_result {
                    Ok(Err(_pool_error)) => {
                        panic_with_error!(&e, InsuranceFundError::PayInsuranceClaimFailed);
                    }
                    Err(_contract_error) => {
                        panic_with_error!(&e, InsuranceFundError::PayInsuranceClaimFailed);
                    }
                    Ok(Ok(pay_from_insurance)) => {
                        if pay_from_insurance > 0 {
                            // Error if there is not enough insurance to cover the claim
                            validate!(
                                &e,
                                pay_from_insurance < reserve.balance,
                                InsuranceFundError::InsufficientCollateral
                            );

                            // Error if a claim leaves removes all insurance
                            let updated_reserve = get_reserve(&e, &token);
                            validate!(
                                &e,
                                updated_reserve.balance > 0,
                                InsuranceFundError::InvalidIFDetected
                            );
                        }

                        exit(&e);
                    }
                }
            }
        }
    }

    //   ________  _______  ___________  ___________  _______   _______    ________
    //  /"       )/"     "|("     _   ")("     _   ")/"     "| /"      \  /"       )
    // (:   \___/(: ______) )__/  \\__/  )__/  \\__/(: ______)|:        |(:   \___/
    //  \___  \   \/    |      \\_ /        \\_ /    \/    |  |_____/   ) \___  \
    //   __/  \\  // ___)_     |.  |        |.  |    // ___)_  //      /   __/  \\
    //  /" \   :)(:      "|    \:  |        \:  |   (:      "||:  __   \  /" \   :)
    // (_______/  \_______)     \__|         \__|    \_______)|__|  \___)(_______/

    fn set_oracle_registry(e: Env, admin: Address, oracle_registry: Address) {
        admin.require_auth();
        require_admin(&e, &admin);

        set_oracle_registry(&e, &oracle_registry);
    }

    fn set_pool_router(e: Env, admin: Address, pool_router: Address) {
        admin.require_auth();
        require_admin(&e, &admin);

        set_pool_router(&e, &pool_router);
    }

    fn set_premium_payer_status(e: Env, admin: Address, payer: Address, status: bool) {
        admin.require_auth();
        require_admin(&e, &admin);

        let old_status = get_premium_payer_status(&e, &payer);
        set_premium_payer_status(&e, &payer, status);

        let current_time = e.ledger().timestamp();

        FundEvents::new(&e).premium_payer_status_updated(
            current_time,
            admin,
            payer,
            old_status,
            status,
        );
    }

    fn set_unstaking_period(e: Env, admin: Address, unstaking_period: u64) {
        admin.require_auth();
        require_admin(&e, &admin);

        set_unstaking_period(&e, &unstaking_period);
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

    fn set_optimal_insurance(e: Env, admin: Address, optimal_insurance: u128) {
        admin.require_auth();
        require_admin(&e, &admin);

        set_optimal_insurance(&e, &optimal_insurance);
    }

    fn add_token_whitelist(e: Env, admin: Address, token: WhitelistToken) {
        admin.require_auth();
        require_admin(&e, &admin);

        // Error if token already exists
        if get_token_whitelist_status(&e, &token.address) {
            panic_with_error!(&e, InsuranceFundError::AdminNotSet);
        }

        validate_token_contract(&e, &token.address);

        // Validate oracle
        let _: u128 = e.invoke_contract(
            &get_oracle_registry(&e),
            &Symbol::new(&e, "get_price"),
            Vec::from_array(&e, [token.symbol.to_val()]),
        );

        set_token_whitelist(&e, &token);

        let mut token_vec = get_token_whitelist_vec(&e);
        token_vec.push_back(token.address.clone());
        set_token_whitelist_vec(&e, &token_vec);

        // Setup reserve
        let reserve = get_reserve(&e, &token.address);
        put_reserve(&e, &token.address, &reserve);

        FundEvents::new(&e).whitelist_token(admin, token.address, token.symbol);
    }

    fn set_token_whitelist_status(e: Env, admin: Address, token: Address, status: bool) {
        admin.require_auth();
        require_admin(&e, &admin);

        let token = get_token_whitelist(&e, &token);

        set_token_whitelist(
            &e,
            &(WhitelistToken {
                active: status,
                ..token
            }),
        );
    }

    fn remove_whitelist_token(e: Env, admin: Address, token: Address) {
        admin.require_auth();
        require_admin(&e, &admin);

        let whitelist_token = get_token_whitelist(&e, &token);

        // Error if not inactive token
        if whitelist_token.active {
            panic_with_error!(&e, InsuranceFundError::AdminNotSet);
        }

        let reserve = get_reserve(&e, &whitelist_token.address);

        // Error if reserve has not been depleted yet
        if reserve.balance != 0 {
            panic_with_error!(&e, InsuranceFundError::AdminNotSet);
        }

        remove_token_whitelist(&e, &whitelist_token.address);

        let mut token_vec = get_token_whitelist_vec(&e);

        for i in 0..token_vec.len() {
            if let Some(existing) = token_vec.get(i) {
                if existing == whitelist_token.address {
                    token_vec.remove_unchecked(i);
                }
            }
        }
        set_token_whitelist_vec(&e, &token_vec);

        FundEvents::new(&e).remove_whitelist_token(admin, whitelist_token.address, reserve.balance);
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
