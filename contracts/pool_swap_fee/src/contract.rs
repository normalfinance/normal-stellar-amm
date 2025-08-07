use core::cmp::max;

use crate::errors::Error;
use crate::events::{Events, ProviderFeeEvents};
use crate::incentives::get_incentives_manager;
use crate::interface::{AdminInterface, PoolSwapFeeInterface};
use crate::storage::{
    get_buffer, get_buffer_fraction, get_fee_destination, get_insurance_fund, get_last_trade_ts,
    get_lp_revenue_fraction, get_router, get_volume_30d, set_buffer, set_buffer_fraction,
    set_fee_destination, set_insurance_fund, set_lp_revenue_fraction, set_router, set_volume_30d,
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
use pool_tokens::{get_total_lp_tokens, get_user_balance_lp};
use soroban_fixed_point_math::FixedPoint;
use soroban_sdk::auth::{ContractContext, InvokerContractAuthEntry, SubContractInvocation};
use soroban_sdk::token::Client as SorobanTokenClient;
use soroban_sdk::{
    contract, contractimpl, panic_with_error, vec, Address, BytesN, Env, IntoVal, Symbol, Vec,
};
use upgrade::events::Events as UpgradeEvents;
use upgrade::interface::UpgradeableContract;
use upgrade::{apply_upgrade, commit_upgrade, revert_upgrade};
use utils::constant::{FEE_DENOMINATOR, PRICE_PRECISION, THIRTY_DAY};
use utils::math::safe_math::SafeMath;
use utils::math::stats::calculate_rolling_sum;
use utils::state::pool::{PoolInfo, SwapDirection};
use utils::token::transfer_token;

#[contract]
pub struct PoolSwapFeeCollector;

#[contractimpl]
impl PoolSwapFeeInterface for PoolSwapFeeCollector {
    //  ___      ___       __        __    _____  ___
    // |"  \    /"  |     /""\      |" \  (\"   \|"  \
    //  \   \  //   |    /    \     ||  | |.\\   \    |
    //  /\\  \/.    |   /' /\  \    |:  | |: \.   \\  |
    // |: \.        |  //  __'  \   |.  | |.  \    \. |
    // |.  \    /:  | /   /  \\  \  /\  |\|    \    \ |
    // |___|\__/|___|(___/    \___)(__\_|_)\___|\____\)

    /// Executes a token swap through a delegated pool contract with integrated fee routing, rewards, and accounting.
    ///
    /// This function facilitates the user swapping one token for another using the appropriate pool
    /// linked to a synthetic asset symbol. It handles:
    /// - Token transfer from user to router
    /// - Routing the swap to the pool via the router contract
    /// - Applying protocol-level fees
    /// - Distributing LP revenue, buffer reserves, and insurance fund premiums
    /// - Tracking long-term metrics like volume and incentives
    ///
    /// # Arguments
    /// * `e` - The Soroban environment.
    /// * `user` - The address of the user performing the swap.
    /// * `tokens` - A vector of token addresses, typically [Token A, Token B].
    /// * `token_in` - The address of the input token (sold by the user).
    /// * `token_out` - The address of the output token (received by the user).
    /// * `asset` - The synthetic asset symbol tied to the pool.
    /// * `in_amount` - Amount of the input token being swapped.
    /// * `out_min` - Minimum acceptable output token amount (slippage protection).
    ///
    /// # Returns
    /// * `u128` - The actual amount of output token received (after all fees).
    ///
    /// # Panics
    /// * If the user is not authorized.
    /// * If the output after fees is below `out_min`.
    /// * If cross-contract calls fail (e.g., due to misconfiguration or insufficient balances).
    ///
    /// # Side Effects
    /// * Transfers tokens between user, pool, router, buffer, and insurance fund.
    /// * Emits swap-related events: trade, buffer deposit, insurance premium, protocol fee charged.
    /// * Updates rolling volume and LP incentive checkpoints.
    fn swap(
        e: Env,
        user: Address,
        asset: Symbol,
        direction: SwapDirection,
        in_amount: u128,
        out_min: u128,
    ) -> u128 {
        user.require_auth();

        let now = e.ledger().timestamp();

        // Fetch the pool
        let router = get_router(&e);
        let pool_info: PoolInfo = e.invoke_contract(
            &router,
            &Symbol::new(&e, "query_pool_details"),
            Vec::from_array(
                &e,
                [
                    e.current_contract_address().to_val(),
                    asset.clone().to_val(),
                ],
            ),
        );

        let (token_in, token_out) = if direction == SwapDirection::Buy {
            (
                pool_info.pool_response.token_b.address,
                pool_info.pool_response.token_a.address,
            )
        } else {
            (
                pool_info.pool_response.token_a.address,
                pool_info.pool_response.token_b.address,
            )
        };

        transfer_token(
            &e,
            &token_in,
            &user,
            &e.current_contract_address(),
            &(in_amount as i128),
        );

        // Always collect the fee in token_b
        let mut fee_amount = 0;
        let mut quote_asset_amount = 0;
        let mut in_amount_mut = in_amount;
        let mut amount_out_w_fee = 0;

        // Update fee if on token_in
        if direction == SwapDirection::Buy {
            quote_asset_amount = in_amount;
            fee_amount = (in_amount * (pool_info.pool_response.pool.fee_fraction as u128))
                / (FEE_DENOMINATOR as u128);
            in_amount_mut = in_amount - fee_amount;
        }

        e.authorize_as_current_contract(vec![
            &e,
            InvokerContractAuthEntry::Contract(SubContractInvocation {
                context: ContractContext {
                    contract: token_in.clone(),
                    fn_name: Symbol::new(&e, "transfer"),
                    args: (
                        e.current_contract_address(),
                        router.clone(),
                        in_amount_mut as i128,
                    )
                        .into_val(&e),
                },
                sub_invocations: vec![&e],
            }),
        ]);
        let amount_out: u128 = e.invoke_contract(
            &router,
            &Symbol::new(&e, "swap"),
            Vec::from_array(
                &e,
                [
                    user.clone().to_val(),
                    asset.clone().to_val(),
                    direction.clone().into_val(&e),
                    in_amount_mut.into_val(&e),
                    out_min.into_val(&e),
                ],
            ),
        );

        // Update fee if on token_out
        if direction == SwapDirection::Sell {
            quote_asset_amount = amount_out;
            fee_amount = (amount_out * (pool_info.pool_response.pool.fee_fraction as u128))
                / (FEE_DENOMINATOR as u128);
        }

        amount_out_w_fee = amount_out - fee_amount;
        if amount_out_w_fee < out_min {
            panic_with_error!(&e, Error::OutMinNotSatisfied);
        }

        // Send token_out to the user
        transfer_token(
            &e,
            &token_out,
            &e.current_contract_address(),
            &user,
            &(amount_out_w_fee as i128),
        );

        // UPDATE METRICS
        let volume_30d = get_volume_30d(&e);
        let since_last = max(1_u64, now.safe_sub(&e, get_last_trade_ts(&e)));
        let updated_volume_30d =
            calculate_rolling_sum(&e, volume_30d, quote_asset_amount, since_last, THIRTY_DAY);
        set_volume_30d(&e, &updated_volume_30d);

        // LP FEES
        let lp_revenue_fraction = get_lp_revenue_fraction(&e);
        let lp_fee_amount =
            (fee_amount * (lp_revenue_fraction as u128)) / (FEE_DENOMINATOR as u128);

        let mut protocol_fee_amount = fee_amount.safe_sub(&e, lp_fee_amount);

        // BUFFER
        let buffer_fraction = get_buffer_fraction(&e);
        let fee_amount_for_buffer = (protocol_fee_amount * (buffer_fraction as u128)) / 10_000_u128;
        let buffer = get_buffer(&e);

        e.authorize_as_current_contract(vec![
            &e,
            InvokerContractAuthEntry::Contract(SubContractInvocation {
                context: ContractContext {
                    contract: token_out.clone(),
                    fn_name: Symbol::new(&e, "transfer"),
                    args: (
                        e.current_contract_address(),
                        buffer.clone(),
                        fee_amount_for_buffer as i128,
                    )
                        .into_val(&e),
                },
                sub_invocations: vec![&e],
            }),
        ]);
        let _: u128 = e.invoke_contract(
            &get_buffer(&e),
            &Symbol::new(&e, "deposit"),
            Vec::from_array(
                &e,
                [
                    e.current_contract_address().to_val(),
                    token_out.clone().to_val(),
                    fee_amount_for_buffer.into_val(&e),
                ],
            ),
        );

        protocol_fee_amount = protocol_fee_amount - fee_amount_for_buffer;

        // INSURANCE FUND
        let insurance_fund = get_insurance_fund(&e);
        let insurance_premium_rate: i32 = e.invoke_contract(
            &insurance_fund,
            &Symbol::new(&e, "get_rate"),
            Vec::from_array(&e, [e.current_contract_address().to_val()]),
        );
        let pool_insurance_coverage: u128 = e.invoke_contract(
            &router,
            &Symbol::new(&e, "get_insurance_coverage"),
            Vec::from_array(
                &e,
                [
                    e.current_contract_address().to_val(),
                    asset.clone().to_val(),
                ],
            ),
        );

        let mut if_premium_paid: u128 = 0;
        if insurance_premium_rate > 0 {
            let estimated_annual_volume = updated_volume_30d.fixed_mul_floor(365, 30).unwrap();

            let total_annual_premium = pool_insurance_coverage
                .fixed_mul_floor(insurance_premium_rate as u128, PRICE_PRECISION)
                .unwrap();
            let premium_per_dollar_swapped =
                total_annual_premium.safe_div(&e, estimated_annual_volume);
            // Lesser of premium or what's left of protocol fee
            let insurance_premium_to_pay = quote_asset_amount
                .safe_mul(&e, premium_per_dollar_swapped)
                .min(protocol_fee_amount);

            if insurance_premium_to_pay > 0 {
                // TODO: must we also call the Pool to update last_revenue_withdraw_ts and rev_withdraw_since_last_settle?

                if_premium_paid = e.invoke_contract(
                    &insurance_fund,
                    &Symbol::new(&e, "pay_premium"),
                    Vec::from_array(
                        &e,
                        [
                            e.current_contract_address().to_val(),
                            insurance_premium_to_pay.into_val(&e),
                        ],
                    ),
                );

                protocol_fee_amount = protocol_fee_amount - insurance_premium_to_pay;
            }
        }

        Events::new(&e).swap(
            asset,
            pool_info.pool_address,
            user.clone(),
            direction,
            in_amount,
            amount_out,
            fee_amount,
            lp_fee_amount,
            fee_amount_for_buffer,
            if_premium_paid,
            protocol_fee_amount,
        );

        // Update total incentives data and refresh/initialize user incentive
        let out_idx = 0;
        let incentives = get_incentives_manager(&e);
        let total_shares = get_total_lp_tokens(&e);
        let user_shares = get_user_balance_lp(&e, &user);
        let token_b_fee = if out_idx == 0 { 0 } else { lp_fee_amount };
        incentives
            .manager()
            .checkpoint_user(&user, total_shares, user_shares, token_b_fee);

        amount_out_w_fee
    }

    //   _______    _______  ___________  ___________  _______   _______    ________
    //  /" _   "|  /"     "|("     _   ")("     _   ")/"     "| /"      \  /"       )
    // (: ( \___) (: ______) )__/  \\__/  )__/  \\__/(: ______)|:        |(:   \___/
    //  \/ \       \/    |      \\_ /        \\_ /    \/    |  |_____/   ) \___  \
    //  //  \ ___  // ___)_     |.  |        |.  |    // ___)_  //      /   __/  \\
    // (:   _(  _|(:      "|    \:  |        \:  |   (:      "||:  __   \  /" \   :)
    //  \_______)  \_______)     \__|         \__|    \_______)|__|  \___)(_______/

    fn get_router(e: Env) -> Address {
        get_router(&e)
    }

    fn get_buffer(e: Env) -> Address {
        get_buffer(&e)
    }

    fn get_fee_destination(e: Env) -> Address {
        get_fee_destination(&e)
    }

    fn get_buffer_fraction(e: Env) -> u32 {
        get_buffer_fraction(&e)
    }

    fn get_lp_revenue_fraction(e: Env) -> u32 {
        get_lp_revenue_fraction(&e)
    }
}

// The `AdminInterface` trait provides the interface for administrative actions.
#[contractimpl]
impl AdminInterface for PoolSwapFeeCollector {
    // Initializes the admin user.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin user.
    fn init_admin(e: Env, admin: Address, emergency_admin: Address) {
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

    /// Claims and transfers all accumulated protocol fees of a given token to the fee destination.
    ///
    /// This function is restricted to the admin and is used to move all collected fees (in `token`)
    /// from the contract's balance to the designated fee recipient address.
    ///
    /// # Arguments
    /// * `e` - The Soroban environment.
    /// * `admin` - The admin address authorized to claim fees.
    /// * `token` - The token address representing the asset whose fees are being claimed.
    ///
    /// # Returns
    /// * `u128` - The total amount of the token transferred to the fee destination.
    ///
    /// # Panics
    /// * If the caller is not authorized as admin.
    ///
    /// # Side Effects
    /// * Transfers the full token balance from the contract to the `fee_destination`.
    /// * Emits a `claim_fee` event recording the token and amount.
    fn claim_fees(e: Env, admin: Address, token: Address) -> u128 {
        admin.require_auth();
        require_admin(&e, &admin);

        let token_client = SorobanTokenClient::new(&e, &token);
        let amount = token_client.balance(&e.current_contract_address());

        transfer_token(
            &e,
            &token,
            &e.current_contract_address(),
            &get_fee_destination(&e),
            &amount,
        );
        Events::new(&e).claim_fees(token, admin, amount as u128);
        amount as u128
    }

    //   ________  _______  ___________  ___________  _______   _______    ________
    //  /"       )/"     "|("     _   ")("     _   ")/"     "| /"      \  /"       )
    // (:   \___/(: ______) )__/  \\__/  )__/  \\__/(: ______)|:        |(:   \___/
    //  \___  \   \/    |      \\_ /        \\_ /    \/    |  |_____/   ) \___  \
    //   __/  \\  // ___)_     |.  |        |.  |    // ___)_  //      /   __/  \\
    //  /" \   :)(:      "|    \:  |        \:  |   (:      "||:  __   \  /" \   :)
    // (_______/  \_______)     \__|         \__|    \_______)|__|  \___)(_______/

    fn set_router(e: Env, admin: Address, router: Address) {
        admin.require_auth();
        require_admin(&e, &admin);

        set_router(&e, &router);
    }

    fn set_buffer(e: Env, admin: Address, buffer: Address) {
        admin.require_auth();
        require_admin(&e, &admin);

        set_buffer(&e, &buffer);
    }

    fn set_insurance_fund(e: Env, admin: Address, insurance_fund: Address) {
        admin.require_auth();
        require_admin(&e, &admin);

        set_insurance_fund(&e, &insurance_fund);
    }

    fn set_fee_destination(e: Env, admin: Address, fee_destination: Address) {
        admin.require_auth();
        require_admin(&e, &admin);

        set_fee_destination(&e, &fee_destination);
    }

    fn set_buffer_fraction(e: Env, admin: Address, fraction: u32) {
        admin.require_auth();
        require_admin(&e, &admin);

        set_buffer_fraction(&e, &fraction);
    }

    fn set_lp_revenue_fraction(e: Env, admin: Address, fraction: u32) {
        admin.require_auth();
        require_admin(&e, &admin);

        set_lp_revenue_fraction(&e, &fraction);
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
            0 => match access_control.get_role_safe(&role) {
                Some(address) => address,
                None => panic_with_error!(&e, AccessControlError::RoleNotFound),
            },
            _ => access_control.get_future_address(&role),
        }
    }
}
