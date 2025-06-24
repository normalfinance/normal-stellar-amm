use crate::errors::BufferError;
use crate::events::{BufferEvents, Events};
use crate::interface::{AdminInterface, BufferTrait};
use crate::reserve::Reserve;
use crate::storage::{
    get_buffer_reserve_amount, get_is_killed_deposit, get_is_killed_resolve_liquidity_deficit,
    get_last_payout_timestamp, get_min_reserve_ratio, get_min_time_between_payouts, get_reserve,
    put_reserve, set_is_killed_deposit, set_is_killed_resolve_liquidity_deficit,
    set_last_payout_timestamp, set_min_reserve_ratio, set_min_time_between_payouts,
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
use soroban_sdk::{
    contract, contractimpl, contractmeta, panic_with_error, Address, BytesN, Env, IntoVal, Symbol,
    Vec,
};
use upgrade::events::Events as UpgradeEvents;
use upgrade::interface::UpgradeableContract;
use upgrade::{apply_upgrade, commit_upgrade, revert_upgrade};
use utils::math::safe_math::SafeMath;
use utils::token::{transfer_token, validate_token_contract};

contractmeta!(
    key = "Description",
    val = "Senior tranche (first payout) backstop fund to cover pool liquidity deficits using protocol revenue"
);

#[contract]
pub struct Buffer;

// The `BufferTrait` trait provides the interface for interacting with the buffer.
#[contractimpl]
impl BufferTrait for Buffer {
    fn initialize(
        e: Env,
        admin: Address,
        emergency_admin: Address,
        time_bt_payouts: u64,
        min_reserve_ratio: u32,
    ) {
        admin.require_auth();

        let access_control = AccessControl::new(&e);
        if access_control.get_role_safe(&Role::Admin).is_some() {
            panic_with_error!(&e, BufferError::AlreadyInitialized);
        }
        access_control.set_role_address(&Role::Admin, &admin);
        access_control.set_role_address(&Role::EmergencyAdmin, &emergency_admin);

        set_min_time_between_payouts(&e, &time_bt_payouts);
        set_min_reserve_ratio(&e, &min_reserve_ratio);
    }

    //  ___      ___       __        __    _____  ___
    // |"  \    /"  |     /""\      |" \  (\"   \|"  \
    //  \   \  //   |    /    \     ||  | |.\\   \    |
    //  /\\  \/.    |   /' /\  \    |:  | |: \.   \\  |
    // |: \.        |  //  __'  \   |.  | |.  \    \. |
    // |.  \    /:  | /   /  \\  \  /\  |\|    \    \ |
    // |___|\__/|___|(___/    \___)(__\_|_)\___|\____\)

    // Deposits protocol revenue into the Buffer for a specific token.
    //
    // This function is typically used by the Pool during `swap()` operations to allocate a portion
    // of collected fees to the Buffer, which serves as a short-term reserve for liquidity support.
    // It can also be called externally, though the Buffer is not intended for user fundraising.
    //
    // # Arguments
    // * `e` - The Soroban environment.
    // * `sender` - The address initiating the deposit (must be authorized).
    // * `token` - The token address being deposited.
    // * `amount` - The amount of tokens to deposit.
    //
    // # Behavior
    // * Verifies the `sender` has authorized the call.
    // * Validates that deposits are not currently disabled (`BufferDepositKilled` flag).
    // * Checks that the Buffer reserve does not exceed its configured `max_balance`.
    // * Updates internal Buffer reserve accounting.
    // * Transfers the tokens from the sender to the Buffer contract address.
    // * Emits a `deposit` event.
    //
    // # Access
    // * Currently unrestricted; any address may deposit.
    // * Typically used by `PoolSwapFee.swap()`.
    // * [Future Consideration]: Restrict to Pool or Router only.
    //
    // # Panics / Errors
    // * `BufferError::BufferDepositKilled` – if deposits are disabled.
    // * `BufferError::ReserveMaxBalanceThreshold` – if deposit exceeds the reserve’s `max_balance`.
    fn deposit(e: Env, sender: Address, token: Address, amount: u128) {
        sender.require_auth();

        /* @Halborn
        Currently, anyone can deposit into the Buffer. The only structured deposits
        are from `PoolSwapFee.swap()` when the buffer fraction is removed from the
        total fee amount.

        If a user deposits into the Buffer and later wishes to remove their funds,
        there is no direct function to do this other than `skim()`. However, timing
        would be of essence.

        The Insurance Fund is specifically designed to raise funds from users, whereas
        the Buffer is for protocol revenue deposits only.

        [ ] Would it be reasonable to restrict this function to the PoolSwapFee only?
        [ ] Are `sync()` and `skim()` necessary functions?
          */

        // Ensure deposits are active
        if get_is_killed_deposit(&e) {
            panic_with_error!(e, BufferError::BufferDepositKilled);
        }

        // Validations
        validate_token_contract(&e, &token);

        // Ensure the deposit does not force the Reserve to exceed its maximum balance
        let reserve = get_reserve(&e, &token);
        if reserve.max_balance > 0 && reserve.balance + amount > reserve.max_balance {
            panic_with_error!(&e, BufferError::ReserveMaxBalanceThreshold);
        }

        // Update the Reserve
        let now = e.ledger().timestamp();
        put_reserve(&e, &token, &reserve.deposit(&e, amount, now));

        // Transfer the tokens from the sender to the Buffer
        transfer_token(
            &e,
            &token,
            &sender,
            &e.current_contract_address(),
            &(amount as i128),
        );

        Events::new(&e).deposit(token, sender, amount);
    }

    // Sync token balances with reserves.
    //
    // # Arguments
    //
    // * `sender` - The address of the sender.
    // * `token` - The address of the token to sync.
    fn sync(e: Env, sender: Address, token: Address) {
        sender.require_auth();

        validate_token_contract(&e, &token);

        let now = e.ledger().timestamp();
        let reserve = get_reserve(&e, &token);
        let balance = get_buffer_reserve_amount(&e, &token);
        put_reserve(&e, &token, &reserve.update_balance(balance, now));
    }

    // Skim excess token balances.
    //
    // # Arguments
    //
    // * `sender` - The address of the sender.
    // * `token` - The address of the token to skim.
    fn skim(e: Env, sender: Address, token: Address) {
        sender.require_auth();

        validate_token_contract(&e, &token);

        let reserve = get_reserve(&e, &token);
        let balance = get_buffer_reserve_amount(&e, &token);
        let skimmed = balance.safe_sub(&e, reserve.balance) as i128;
        if skimmed > 0 {
            transfer_token(&e, &token, &e.current_contract_address(), &sender, &skimmed);
            Events::new(&e).skim(token, sender, skimmed);
        }
    }

    //   _______    _______  ___________  ___________  _______   _______    ________
    //  /" _   "|  /"     "|("     _   ")("     _   ")/"     "| /"      \  /"       )
    // (: ( \___) (: ______) )__/  \\__/  )__/  \\__/(: ______)|:        |(:   \___/
    //  \/ \       \/    |      \\_ /        \\_ /    \/    |  |_____/   ) \___  \
    //  //  \ ___  // ___)_     |.  |        |.  |    // ___)_  //      /   __/  \\
    // (:   _(  _|(:      "|    \:  |        \:  |   (:      "||:  __   \  /" \   :)
    //  \_______)  \_______)     \__|         \__|    \_______)|__|  \___)(_______/

    fn get_min_time_between_payouts(e: Env) -> u64 {
        get_min_time_between_payouts(&e)
    }

    fn get_reserve(e: Env, token: Address) -> Reserve {
        get_reserve(&e, &token)
    }

    fn get_min_reserve_ratio(e: Env) -> u32 {
        get_min_reserve_ratio(&e)
    }

    fn get_last_payout_timestamp(e: Env) -> u64 {
        get_last_payout_timestamp(&e)
    }
}

// The `AdminInterface` trait provides the interface for administrative actions.
#[contractimpl]
impl AdminInterface for Buffer {
    //  ___      ___       __        __    _____  ___
    // |"  \    /"  |     /""\      |" \  (\"   \|"  \
    //  \   \  //   |    /    \     ||  | |.\\   \    |
    //  /\\  \/.    |   /' /\  \    |:  | |: \.   \\  |
    // |: \.        |  //  __'  \   |.  | |.  \    \. |
    // |.  \    /:  | /   /  \\  \  /\  |\|    \    \ |
    // |___|\__/|___|(___/    \___)(__\_|_)\___|\____\)

    // Resolves a liquidity deficit in a target Pool by transferring tokens from the Buffer.
    //
    // This function is typically triggered by the Buffer admin to manually assist a Pool suffering
    // from a short-term imbalance or deficit. It transfers funds from the Buffer to the specified
    // Pool and calls its `pay_insurance_claim()` method to settle the claim.
    //
    // # Arguments
    // * `e` - The Soroban environment.
    // * `admin` - The admin authorized to trigger the resolution (must be authenticated).
    // * `token` - The token address of the reserve used for payout.
    // * `amount` - The amount of tokens to send to the Pool.
    // * `pool_address` - The address of the Pool contract requesting liquidity.
    //
    // # Behavior
    // * Verifies that the caller is the authorized Buffer admin.
    // * Ensures that liquidity resolution is currently enabled.
    // * Validates the token contract and confirms the reserve balance is sufficient.
    // * Checks that a minimum interval has passed since the last payout to prevent abuse.
    // * Deducts the `amount` from the Buffer reserve and updates the timestamp.
    // * Sends tokens to the Pool and invokes `pay_insurance_claim()` to settle the deficit.
    // * Emits a `resolve_liquidity_deficit` event.
    //
    // # Access
    // * Currently restricted to Buffer admin only.
    // * [Future Consideration]: Automate in `Pool.swap()` or decentralize via governance.
    //
    // # Panics / Errors
    // * `BufferError::BufferRequestPayoutKilled` – if payouts are currently disabled.
    // * `BufferError::PayoutTooSoon` – if not enough time has passed since the last payout.
    // * `BufferError::InsufficentFunds` – if the reserve balance is insufficient.
    // * `BufferError::InvalidToken` – if the token is not a valid Soroban token contract.
    //
    // # Returns
    // * `u128` – The amount of tokens successfully paid to the Pool.
    fn resolve_liquidity_deficit(
        e: Env,
        admin: Address,
        token: Address,
        amount: u128,
        pool_address: Address,
    ) -> u128 {
        admin.require_auth();
        /* Currently, only the Buffer admin may resolve deficits, however, our goal
        is to either: a) automate within `Pool.swap()` itself; or b) decentralize via the Normal DAO */
        require_admin(&e, &admin);

        if get_is_killed_resolve_liquidity_deficit(&e) {
            panic_with_error!(e, BufferError::BufferRequestPayoutKilled);
        }

        validate_token_contract(&e, &token);

        // TODO: validate pool_address - probably by checking against `PoolRouter.pools_vec`

        // Enforce the minimum time between payouts
        let now = e.ledger().timestamp();
        let last_payout_ts = get_last_payout_timestamp(&e);
        let min_time_bt_payouts = get_min_time_between_payouts(&e);
        if now - last_payout_ts <= min_time_bt_payouts {
            panic_with_error!(&e, BufferError::PayoutTooSoon);
        }

        // Ensure the Buffer Reserve has a sufficient balance
        let reserve = get_reserve(&e, &token);
        if amount > reserve.balance {
            panic_with_error!(&e, BufferError::InsufficentFunds);
        }

        // Update the Buffer Reserve
        put_reserve(&e, &token, &reserve.payout(&e, amount, now));
        set_last_payout_timestamp(&e, &now);

        // Invoke `pay_insurance_claim()` on the Pool to cover the deficit
        let paid: u128 = e.invoke_contract(
            &pool_address,
            &Symbol::new(&e, "pay_insurance_claim"),
            Vec::from_array(
                &e,
                [e.current_contract_address().to_val(), amount.into_val(&e)],
            ),
        );

        Events::new(&e).resolve_liquidity_deficit(token, admin, amount);

        paid
    }

    // Allows the admin to withdraw surplus tokens from the Buffer reserve, above the required minimum.
    //
    // This function ensures that a reserve floor (defined by a minimum reserve ratio) is maintained
    // in the Buffer while allowing the admin to extract excess funds for protocol use (e.g., revenue).
    //
    // # Arguments
    // * `e` - The Soroban environment.
    // * `admin` - The administrator address requesting the withdrawal (must be authenticated).
    // * `token` - The token address to withdraw from the Buffer.
    // * `amount` - The amount of tokens to withdraw.
    //
    // # Behavior
    // * Authenticates the admin and checks admin permissions.
    // * Validates the token contract.
    // * Ensures the withdrawal won't bring the reserve below the minimum reserve ratio.
    // * Updates the Buffer reserve state.
    // * Transfers the tokens from the Buffer to the admin address.
    // * Emits a `withdraw_surplus` event.
    //
    // # Errors / Panics
    // * `BufferError::WithdrawalOverMinimumReserve` – If the withdrawal would violate the min reserve.
    // * `BufferError::InsufficentFunds` – If trying to withdraw more than the reserve balance.
    // * Any error from `validate_token_contract()` or `transfer_token()` (e.g. invalid token).
    //
    // # Notes
    // * The minimum reserve ratio is specified in basis points (e.g. 2500 = 25%).
    // * This function enables revenue extraction while protecting liquidity integrity.
    fn withdraw_surplus(e: Env, admin: Address, token: Address, amount: u128) {
        admin.require_auth();
        require_admin(&e, &admin);

        validate_token_contract(&e, &token);

        // Calculate the minimum reserve that must be left in the Buffer
        let reserve = get_reserve(&e, &token);
        let min_reserve_ratio = get_min_reserve_ratio(&e);
        let min_reserve = (reserve.balance * (min_reserve_ratio as u128)) / 10_000;
        if reserve.balance - amount < min_reserve {
            panic_with_error!(&e, BufferError::WithdrawalOverMinimumReserve);
        }

        if amount > reserve.balance {
            panic_with_error!(&e, BufferError::InsufficentFunds);
        }

        // Update the Buffer reserve
        let now = e.ledger().timestamp();
        put_reserve(&e, &token, &reserve.withdraw(&e, amount, now));

        // Transfer the tokens to the admin
        transfer_token(
            &e,
            &token,
            &e.current_contract_address(),
            &admin,
            &(amount as i128),
        );

        Events::new(&e).withdraw_surplus(token, admin, amount);
    }

    //   ________  _______  ___________  ___________  _______   _______    ________
    //  /"       )/"     "|("     _   ")("     _   ")/"     "| /"      \  /"       )
    // (:   \___/(: ______) )__/  \\__/  )__/  \\__/(: ______)|:        |(:   \___/
    //  \___  \   \/    |      \\_ /        \\_ /    \/    |  |_____/   ) \___  \
    //   __/  \\  // ___)_     |.  |        |.  |    // ___)_  //      /   __/  \\
    //  /" \   :)(:      "|    \:  |        \:  |   (:      "||:  __   \  /" \   :)
    // (_______/  \_______)     \__|         \__|    \_______)|__|  \___)(_______/

    fn set_min_time_between_payouts(e: Env, admin: Address, min_time: u64) {
        admin.require_auth();
        require_admin(&e, &admin);

        set_min_time_between_payouts(&e, &min_time);
    }

    fn set_min_reserve_ratio(e: Env, admin: Address, min_ratio: u32) {
        admin.require_auth();
        require_admin(&e, &admin);

        set_min_reserve_ratio(&e, &min_ratio);
    }

    fn set_reserve_max_balance(e: Env, admin: Address, token: Address, max_balance: u128) {
        admin.require_auth();
        require_admin(&e, &admin);

        let now = e.ledger().timestamp();
        let reserve = get_reserve(&e, &token);
        put_reserve(&e, &token, &reserve.update_max_balance(max_balance, now));
    }

    //    _______     __       ____  ____   ________  _______  ________
    //   |   __ "\   /""\     ("  _||_ " | /"       )/"     "||"      "\
    //   (. |__) :) /    \    |   (  ) : |(:   \___/(: ______)(.  ___  :)
    //   |:  ____/ /' /\  \   (:  |  | . ) \___  \   \/    |  |: \   ) ||
    //   (|  /    //  __'  \   \\ \__/ //   __/  \\  // ___)_ (| (___\ ||
    //  /|__/ \  /   /  \\  \  /\\ __ //\  /" \   :)(:      "||:       :)
    // (_______)(___/    \___)(__________)(_______/  \_______)(________/

    // Stops the buffer deposits instantly.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    fn kill_deposit(e: Env, admin: Address) {
        admin.require_auth();
        require_admin(&e, &admin);

        set_is_killed_deposit(&e, &true);
        Events::new(&e).kill_deposit();
    }

    // Stops the buffer payouts instantly.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    fn kill_resolve_liquidity_deficit(e: Env, admin: Address) {
        admin.require_auth();
        require_admin(&e, &admin);

        set_is_killed_resolve_liquidity_deficit(&e, &true);
        Events::new(&e).kill_resolve_liquidity_deficit();
    }

    // Resumes the buffer deposits.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    fn unkill_deposit(e: Env, admin: Address) {
        admin.require_auth();
        require_admin(&e, &admin);

        set_is_killed_deposit(&e, &false);
        Events::new(&e).unkill_deposit();
    }

    // Resumes the buffer payouts.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    fn unkill_resolve_liquidity_deficit(e: Env, admin: Address) {
        admin.require_auth();
        require_admin(&e, &admin);

        set_is_killed_resolve_liquidity_deficit(&e, &false);
        Events::new(&e).unkill_resolve_liquidity_deficit();
    }

    // Get deposit killswitch status.
    fn get_is_killed_deposit(e: Env) -> bool {
        get_is_killed_deposit(&e)
    }

    // Get payout killswitch status.
    fn get_is_killed_resolve_deficit(e: Env) -> bool {
        get_is_killed_resolve_liquidity_deficit(&e)
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
            0 => match access_control.get_role_safe(&role) {
                Some(address) => address,
                None => panic_with_error!(&e, AccessControlError::RoleNotFound),
            },
            _ => access_control.get_future_address(&role),
        }
    }
}
