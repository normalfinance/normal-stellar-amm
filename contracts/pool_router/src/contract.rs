use crate::errors::PoolRouterError;
use crate::events::{Events, PoolRouterEvents};
use crate::liquidity_calculator::LiquidityCalculatorClient;
use crate::pool_interface::{
    PoolInterfaceTrait, PoolPlaneInterface, PoolsManagementTrait, RewardsInterfaceTrait,
};
use crate::pool_utils::{deploy_pool, get_total_liquidity};
use crate::rewards::get_rewards_manager;
use crate::router_interface::AdminInterface;
use crate::storage::{
    get_insurance_fund, get_liquidity_calculator, get_pool, get_pool_base, get_pool_plane,
    get_pools_vec, get_reward_tokens, get_reward_tokens_detailed, get_rewards_config,
    set_insurance_fund, set_liquidity_calculator, set_oracle_registry, set_pool_hash,
    set_pool_plane, set_reward_tokens, set_reward_tokens_detailed, set_rewards_config,
    set_token_share_hash, GlobalRewardsConfig, PoolRewardInfo,
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
use access_control::utils::{require_admin, require_rewards_admin_or_owner};
use reentrancy_guard::{enter, exit};
use rewards::storage::RewardTokenStorageTrait;
use soroban_sdk::token::Client as SorobanTokenClient;
use soroban_sdk::{
    contract, contractimpl, panic_with_error, symbol_short, Address, BytesN, Env, IntoVal, Map,
    String, Symbol, Val, Vec, U256,
};
use upgrade::events::Events as UpgradeEvents;
use upgrade::interface::UpgradeableContract;
use upgrade::{apply_upgrade, commit_upgrade, revert_upgrade};
use utils::constant::MAX_POOL_FEE;
use utils::state::pool::{PoolInfo, PoolTier, SwapDirection};
use utils::validation::ensure_non_zero_u128;

#[contract]
pub struct PoolRouter;

// The `PoolInterfaceTrait` trait provides the interface for interacting with a liquidity pool.
#[contractimpl]
impl PoolInterfaceTrait for PoolRouter {
    //  ___      ___       __        __    _____  ___
    // |"  \    /"  |     /""\      |" \  (\"   \|"  \
    //  \   \  //   |    /    \     ||  | |.\\   \    |
    //  /\\  \/.    |   /' /\  \    |:  | |: \.   \\  |
    // |: \.        |  //  __'  \   |.  | |.  \    \. |
    // |.  \    /:  | /   /  \\  \  /\  |\|    \    \ |
    // |___|\__/|___|(___/    \___)(__\_|_)\___|\____\)

    // Deposits Token B into the liquidity pool associated with a given synthetic asset.
    //
    // This function is called through the pool router to forward user deposits to the
    // correct pool contract. It performs authentication, retrieves the pool address for
    // the specified asset, and invokes the pool's `deposit` method.
    //
    // # Arguments
    // * `e` - The current Soroban environment.
    // * `user` - The address of the user initiating the deposit (must authorize the call).
    // * `asset` - The symbol representing the synthetic asset the pool is tied to.
    // * `token_b_amount` - The amount of Token B to deposit.
    //
    // # Returns
    // * `(u128, u128)` - A tuple containing:
    //     - The amount of Token B deposited.
    //     - The number of liquidity provider (LP) shares minted to the user.
    //
    // # Behavior
    // * Calls `get_pool` to fetch the associated pool contract for the asset.
    // * Forwards the deposit call to the underlying pool contract via cross-contract invocation.
    // * Emits a `deposit` event with the asset, user, pool address, and deposit details.
    //
    // # Panics
    // * If the user does not authorize the operation.
    // * If the asset has no associated pool registered.

    fn deposit(
        e: Env,
        user: Address,
        asset: Symbol,
        desired_amount: u128,
        min_shares: u128,
    ) -> (u128, u128) {
        user.require_auth();

        ensure_non_zero_u128(&e, desired_amount);

        enter(&e);

        let pool = get_pool(&e, &asset);

        let (amount, share_amount, delta_a): (u128, u128, i128) = e.invoke_contract(
            &pool,
            &symbol_short!("deposit"),
            Vec::from_array(
                &e,
                [
                    user.clone().into_val(&e),
                    desired_amount.into_val(&e),
                    min_shares.into_val(&e),
                ],
            ),
        );
        Events::new(&e).deposit_liquidity(asset, pool, user, amount, share_amount, delta_a);

        exit(&e);

        (amount, share_amount)
    }

    // Executes a token swap within the liquidity pool associated with a synthetic asset.
    //
    // This router function locates the pool for the given asset and forwards the swap request
    // to that pool contract. It supports flexible token indexing using the `tokens` vector to
    // determine input and output indices.
    //
    // # Arguments
    // * `e` - The current Soroban environment.
    // * `user` - The address of the user performing the swap (must authorize the call).
    // * `asset` - The synthetic asset symbol identifying the pool to use.
    // * `token_in` - The token address being sold (input).
    // * `token_out` - The token address being bought (output).
    // * `in_amount` - The amount of the input token to swap.
    // * `out_min` - The minimum acceptable amount of the output token.
    //
    // # Returns
    // * `u128` - The amount of output token received from the swap.
    //
    // # Behavior
    // * Validates the user's authorization.
    // * Resolves the correct pool for the given asset using `get_pool`.
    // * Determines `in_idx` and `out_idx` from the `tokens` vector.
    // * Forwards the swap call to the identified pool contract.
    // * Emits a `swap` event with full context of the operation.
    //
    // # Panics
    // * If the user does not authorize the transaction.
    // * If the token addresses are not found in the `tokens` vector.
    // * If the pool contract is not registered for the given asset.
    fn swap(
        e: Env,
        user: Address,
        asset: Symbol,
        direction: SwapDirection,
        in_amount: u128,
        out_min: u128,
    ) -> u128 {
        user.require_auth();

        ensure_non_zero_u128(&e, in_amount);

        enter(&e);

        let pool_address = get_pool(&e, &asset);

        let (out_amount, delta_a_pre, delta_a_post): (u128, i128, i128) = e.invoke_contract(
            &pool_address,
            &symbol_short!("swap"),
            Vec::from_array(
                &e,
                [
                    user.clone().into_val(&e),
                    direction.into_val(&e),
                    in_amount.into_val(&e),
                    out_min.into_val(&e),
                ],
            ),
        );

        Events::new(&e).swap(
            asset,
            pool_address,
            user,
            direction,
            in_amount,
            out_amount,
            delta_a_pre,
            delta_a_post,
        );

        exit(&e);

        out_amount
    }

    // Estimates the output amount and fee for a token swap without executing it.
    //
    // This function queries the associated pool contract for a synthetic asset to simulate a swap
    // from `token_in` to `token_out`, based on the provided input amount. It is useful for frontend
    // price discovery, slippage estimation, and swap previews.
    //
    // # Arguments
    // * `e` - The current Soroban environment.
    // * `asset` - The synthetic asset symbol representing the target liquidity pool.
    // * `in_amount` - The amount of input token to simulate a swap for.
    //
    // # Returns
    // * `(u128, i128)` - A tuple containing:
    //     - The estimated amount of output token received.
    //     - The estimated delta_a (can be negative or positive depending on implementation).
    //
    // # Panics
    // * If `token_in` or `token_out` are not found in the `tokens` vector.
    // * If no pool is registered for the provided `asset`.
    //
    // # Notes
    // * This call does not mutate state and is intended for off-chain estimation.
    // * Token ordering must match the pool's internal ordering for index lookups to work correctly.
    fn estimate_swap(
        e: Env,
        asset: Symbol,
        direction: SwapDirection,
        in_amount: u128,
    ) -> (u128, i128) {
        ensure_non_zero_u128(&e, in_amount);

        let pool_address = get_pool(&e, &asset);

        e.invoke_contract(
            &pool_address,
            &Symbol::new(&e, "estimate_swap"),
            Vec::from_array(&e, [direction.into_val(&e), in_amount.into_val(&e)]),
        )
    }

    // Withdraws liquidity from the pool associated with a synthetic asset.
    //
    // This function delegates the withdrawal request to the pool contract linked with the specified `asset`,
    // burning the user's share tokens and transferring the corresponding amount of underlying tokens back to the user.
    //
    // # Arguments
    // * `e` - The current Soroban environment.
    // * `user` - The address of the user initiating the withdrawal.
    // * `asset` - The symbol of the synthetic asset tied to the pool.
    // * `share_amount` - The number of LP (liquidity provider) shares the user wishes to redeem.
    //
    // # Returns
    // * `u128` - The amount of underlying token (usually Token B) returned to the user upon withdrawal.
    //
    // # Panics
    // * If the `user` is not authorized.
    // * If no pool is registered for the specified `asset`.
    //
    // # Emits
    // * A `withdraw` event recording the asset, user, pool, withdrawn amount, and share amount.
    fn withdraw(
        e: Env,
        user: Address,
        asset: Symbol,
        share_amount: u128,
        min_amounts: Vec<u128>,
    ) -> u128 {
        user.require_auth();

        ensure_non_zero_u128(&e, share_amount);

        enter(&e);

        let pool_address = get_pool(&e, &asset);

        let (amount, delta_a): (u128, i128) = e.invoke_contract(
            &pool_address,
            &symbol_short!("withdraw"),
            Vec::from_array(
                &e,
                [
                    user.clone().into_val(&e),
                    share_amount.into_val(&e),
                    min_amounts.into_val(&e),
                ],
            ),
        );

        Events::new(&e).withdraw_liquidity(
            asset,
            pool_address,
            user,
            share_amount,
            amount,
            delta_a,
        );

        exit(&e);

        amount
    }

    //   _______    _______  ___________  ___________  _______   _______    ________
    //  /" _   "|  /"     "|("     _   ")("     _   ")/"     "| /"      \  /"       )
    // (: ( \___) (: ______) )__/  \\__/  )__/  \\__/(: ______)|:        |(:   \___/
    //  \/ \       \/    |      \\_ /        \\_ /    \/    |  |_____/   ) \___  \
    //  //  \ ___  // ___)_     |.  |        |.  |    // ___)_  //      /   __/  \\
    // (:   _(  _|(:      "|    \:  |        \:  |   (:      "||:  __   \  /" \   :)
    //  \_______)  \_______)     \__|         \__|    \_______)|__|  \___)(_______/

    // Returns a map of privileged roles.
    fn get_privileged_addrs(e: Env) -> Map<Symbol, Vec<Address>> {
        let access_control = AccessControl::new(&e);
        let mut result: Map<Symbol, Vec<Address>> = Map::new(&e);
        for role in [
            Role::Admin,
            Role::EmergencyAdmin,
            Role::RewardsAdmin,
            Role::OperationsAdmin,
            Role::PauseAdmin,
        ] {
            result.set(
                role.as_symbol(&e),
                match access_control.get_role_safe(&role) {
                    Some(v) => Vec::from_array(&e, [v]),
                    None => Vec::new(&e),
                },
            );
        }

        result.set(
            Role::EmergencyPauseAdmin.as_symbol(&e),
            access_control.get_role_addresses(&Role::EmergencyPauseAdmin),
        );

        result
    }

    fn get_info(e: Env, asset: Symbol) -> Map<Symbol, Val> {
        let pool_id = get_pool(&e, &asset);
        e.invoke_contract(&pool_id, &Symbol::new(&e, "get_info"), Vec::new(&e))
    }

    fn get_pool(e: Env, asset: Symbol) -> Address {
        get_pool(&e, &asset)
    }

    fn share_id(e: Env, asset: Symbol) -> Address {
        let pool_id = get_pool(&e, &asset);
        e.invoke_contract(&pool_id, &Symbol::new(&e, "share_id"), Vec::new(&e))
    }

    fn get_total_shares(e: Env, asset: Symbol) -> u128 {
        let pool_id = get_pool(&e, &asset);
        e.invoke_contract(&pool_id, &Symbol::new(&e, "get_total_shares"), Vec::new(&e))
    }

    fn get_reserves(e: Env, asset: Symbol) -> Vec<u128> {
        let pool_id = get_pool(&e, &asset);
        e.invoke_contract(&pool_id, &Symbol::new(&e, "get_reserves"), Vec::new(&e))
    }

    fn get_fee_fraction(e: Env, asset: Symbol) -> u32 {
        let pool_id = get_pool(&e, &asset);
        e.invoke_contract(&pool_id, &Symbol::new(&e, "get_fee_fraction"), Vec::new(&e))
    }

    fn get_insurance_coverage(e: Env, asset: Symbol) -> u128 {
        let pool_id = get_pool(&e, &asset);
        e.invoke_contract(
            &pool_id,
            &Symbol::new(&e, "get_insurance_coverage"),
            Vec::new(&e),
        )
    }

    // Returns the total liquidity of the pool.
    fn get_liquidity(e: Env, asset: Symbol) -> U256 {
        let pool_id = get_pool(&e, &asset);

        let calculator = get_liquidity_calculator(&e);
        match LiquidityCalculatorClient::new(&e, &calculator)
            .get_liquidity(&Vec::from_array(&e, [pool_id]))
            .get(0)
        {
            Some(v) => v,
            None => panic_with_error!(&e, PoolRouterError::LiquidityCalculationError),
        }
    }

    fn get_liquidity_calculator(e: Env) -> Address {
        get_liquidity_calculator(&e)
    }
}

// The `UpgradeableContract` trait provides the interface for upgrading the contract.
#[contractimpl]
impl UpgradeableContract for PoolRouter {
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
impl AdminInterface for PoolRouter {
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

    //   ________  _______  ___________  ___________  _______   _______    ________
    //  /"       )/"     "|("     _   ")("     _   ")/"     "| /"      \  /"       )
    // (:   \___/(: ______) )__/  \\__/  )__/  \\__/(: ______)|:        |(:   \___/
    //  \___  \   \/    |      \\_ /        \\_ /    \/    |  |_____/   ) \___  \
    //   __/  \\  // ___)_     |.  |        |.  |    // ___)_  //      /   __/  \\
    //  /" \   :)(:      "|    \:  |        \:  |   (:      "||:  __   \  /" \   :)
    // (_______/  \_______)     \__|         \__|    \_______)|__|  \___)(_______/

    fn set_privileged_addrs(
        e: Env,
        admin: Address,
        rewards_admin: Address,
        operations_admin: Address,
        pause_admin: Address,
        emergency_pause_admins: Vec<Address>,
    ) {
        admin.require_auth();
        let access_control = AccessControl::new(&e);
        access_control.assert_address_has_role(&admin, &Role::Admin);

        access_control.set_role_address(&Role::RewardsAdmin, &rewards_admin);
        access_control.set_role_address(&Role::OperationsAdmin, &operations_admin);
        access_control.set_role_address(&Role::PauseAdmin, &pause_admin);
        access_control.set_role_addresses(&Role::EmergencyPauseAdmin, &emergency_pause_admins);
        AccessControlEvents::new(&e).set_privileged_addrs(
            rewards_admin,
            operations_admin,
            pause_admin,
            emergency_pause_admins,
        );
    }

    fn set_insurance_fund(e: Env, admin: Address, insurance_fund: Address) {
        admin.require_auth();
        AccessControl::new(&e).assert_address_has_role(&admin, &Role::Admin);
        set_insurance_fund(&e, &insurance_fund);
    }

    fn set_liquidity_calculator(e: Env, admin: Address, calculator: Address) {
        admin.require_auth();
        AccessControl::new(&e).assert_address_has_role(&admin, &Role::Admin);
        set_liquidity_calculator(&e, &calculator);
    }

    fn set_oracle_registry(e: Env, admin: Address, oracle_registry: Address) {
        admin.require_auth();
        AccessControl::new(&e).assert_address_has_role(&admin, &Role::Admin);
        set_oracle_registry(&e, &oracle_registry);
    }

    // Sets the liquidity pool share token wasm hash.
    fn set_token_share_hash(e: Env, admin: Address, new_hash: BytesN<32>) {
        admin.require_auth();
        AccessControl::new(&e).assert_address_has_role(&admin, &Role::Admin);
        set_token_share_hash(&e, &new_hash);
    }

    // Sets the pool wasm hash.
    fn set_pool_hash(e: Env, admin: Address, new_hash: BytesN<32>) {
        admin.require_auth();
        AccessControl::new(&e).assert_address_has_role(&admin, &Role::Admin);
        set_pool_hash(&e, &new_hash);
    }

    // Sets the reward token.
    fn set_reward_token(e: Env, admin: Address, reward_token: Address) {
        admin.require_auth();
        AccessControl::new(&e).assert_address_has_role(&admin, &Role::Admin);
        get_rewards_manager(&e)
            .storage()
            .put_reward_token(reward_token);
    }

    //   _______    _______  ___________  ___________  _______   _______    ________
    //  /" _   "|  /"     "|("     _   ")("     _   ")/"     "| /"      \  /"       )
    // (: ( \___) (: ______) )__/  \\__/  )__/  \\__/(: ______)|:        |(:   \___/
    //  \/ \       \/    |      \\_ /        \\_ /    \/    |  |_____/   ) \___  \
    //  //  \ ___  // ___)_     |.  |        |.  |    // ___)_  //      /   __/  \\
    // (:   _(  _|(:      "|    \:  |        \:  |   (:      "||:  __   \  /" \   :)
    //  \_______)  \_______)     \__|         \__|    \_______)|__|  \___)(_______/

    fn get_insurance_fund(e: Env) -> Address {
        get_insurance_fund(&e)
    }
}

// The `RewardsInterfaceTrait` trait provides the interface for interacting with rewards.
#[contractimpl]
impl RewardsInterfaceTrait for PoolRouter {
    // Retrieves the global rewards configuration and returns it as a `Map`.
    //
    // This function fetches the global rewards configuration from the contract's state.
    // The configuration includes the rewards per second (`tps`) and the expiration timestamp (`expired_at`)
    //
    // # Returns
    //
    // A `Map` where each key is a `Symbol` representing a configuration parameter, and the value is the corresponding value.
    // The keys are "tps" and "expired_at".
    fn get_rewards_config(e: Env) -> Map<Symbol, i128> {
        let rewards_config = get_rewards_config(&e);
        let mut result = Map::new(&e);
        result.set(symbol_short!("tps"), rewards_config.tps as i128);
        result.set(symbol_short!("exp_at"), rewards_config.expired_at as i128);
        result
    }

    // Returns a mapping of token addresses to their respective reward information.
    //
    // # Returns
    //
    // A `Map` where each key is a `Symbol` representing an asset, and the value is a tuple
    // `(bool, U256)`. The tuple elements represent the processed status, and total liquidity
    // of the tokens respectively.
    fn get_tokens_for_reward(e: Env) -> Map<Symbol, (bool, U256)> {
        let tokens = get_reward_tokens(&e);
        let mut result = Map::new(&e);
        for (key, value) in tokens {
            result.set(key, (value.processed, value.total_liquidity));
        }
        result
    }

    // Sums up the liquidity of all pools for given tokens set and returns the total liquidity
    //
    // # Arguments
    //
    // * `asset` - A asset for which to calculate the total liquidity.
    //
    // # Returns
    //
    // A `U256` value representing the total liquidity for the given set of tokens.
    fn get_total_liquidity(e: Env, asset: Symbol) -> U256 {
        let pool_address = get_pool(&e, &asset);

        let calculator = get_liquidity_calculator(&e);
        let mut pools_vec: Vec<Address> = Vec::new(&e);
        pools_vec.push_back(pool_address.clone());

        let pools_liquidity =
            LiquidityCalculatorClient::new(&e, &calculator).get_liquidity(&pools_vec);
        let mut result = U256::from_u32(&e, 0);
        for liquidity in pools_liquidity {
            result = result.add(&liquidity);
        }
        result
    }

    // Configures the global rewards for the liquidity pool.
    //
    // # Arguments
    //
    // * `user` - This user must be authenticated and have admin or operator privileges.
    // * `reward_tps` - The rewards per second. This value is scaled by 1e7 for precision.
    // * `expired_at` - The timestamp at which the rewards configuration will expire.
    // * `assets` - A vector of symbols.
    fn config_global_rewards(
        e: Env,
        user: Address,
        reward_tps: u128, // value with 7 decimal places. example: 600_0000000
        expired_at: u64,  // timestamp
        assets: Vec<Symbol>,
    ) {
        user.require_auth();
        require_rewards_admin_or_owner(&e, &user);

        let mut tokens_with_liquidity = Map::new(&e);
        for asset in assets {
            tokens_with_liquidity.set(
                asset,
                PoolRewardInfo {
                    processed: false,
                    total_liquidity: U256::from_u32(&e, 0),
                },
            );
        }

        set_reward_tokens(&e, &tokens_with_liquidity);
        set_rewards_config(
            &e,
            &GlobalRewardsConfig {
                tps: reward_tps,
                expired_at,
            },
        )
    }

    // Fills the aggregated liquidity information for a given set of tokens.
    //
    // # Arguments
    //
    // * `user` - This user must be authenticated and have admin or operator privileges.
    // * `asset` - A symbol of the asset for which to fill the liquidity.
    fn fill_liquidity(e: Env, user: Address, asset: Symbol) {
        user.require_auth();
        require_rewards_admin_or_owner(&e, &user);

        let calculator = get_liquidity_calculator(&e);
        let total_liquidity = get_total_liquidity(&e, asset.clone(), calculator);

        if total_liquidity == U256::from_u32(&e, 0) {
            return;
        }

        let pool_with_processed_info = (total_liquidity.clone(), false);

        let mut tokens_with_liquidity = get_reward_tokens(&e);
        let mut token_data = match tokens_with_liquidity.get(asset.clone()) {
            Some(v) => v,
            None => panic_with_error!(e, PoolRouterError::TokensAreNotForReward),
        };
        if token_data.processed {
            panic_with_error!(e, PoolRouterError::LiquidityAlreadyFilled);
        }
        token_data.processed = true;
        token_data.total_liquidity = total_liquidity;
        tokens_with_liquidity.set(asset.clone(), token_data);
        set_reward_tokens(&e, &tokens_with_liquidity);
        set_reward_tokens_detailed(&e, &asset, &pool_with_processed_info);
    }

    // Configures the rewards for a specific pool.
    //
    // This function is used to set up the rewards configuration for a specific pool.
    // It calculates the pool's share of the total rewards based on its liquidity and sets the pool's rewards configuration.
    //
    // # Arguments
    //
    // * `user` - This user must be authenticated and have admin or operator privileges.
    // * `asset` - A symbol of the asset that the pool consists of.
    //
    // # Returns
    //
    // * `pool_tps` - The total reward tokens per second (TPS) to be distributed to the pool.
    //
    // # Errors
    //
    // This function will panic if:
    //
    // * The pool does not exist.
    // * The tokens are not found in the current rewards configuration.
    // * The liquidity for the tokens has not been filled.
    fn config_pool_rewards(e: Env, user: Address, asset: Symbol) -> u128 {
        user.require_auth();
        require_rewards_admin_or_owner(&e, &user);

        let pool_address = get_pool(&e, &asset);

        let rewards_config = get_rewards_config(&e);
        let mut tokens_detailed = get_reward_tokens_detailed(&e, asset.clone());
        let tokens_reward = get_reward_tokens(&e);
        let tokens_reward_info = tokens_reward.get(asset.clone());

        let (pool_liquidity, pool_configured) = if tokens_reward_info.is_some() {
            tokens_detailed
        } else {
            (U256::from_u32(&e, 0), false)
        };

        if pool_configured {
            panic_with_error!(&e, PoolRouterError::RewardsAlreadyConfigured);
        }

        let reward_info = match tokens_reward_info {
            Some(v) => v,
            // if tokens not found in current config, deactivate them
            None => PoolRewardInfo {
                processed: true,
                total_liquidity: U256::from_u32(&e, 0),
            },
        };

        if !reward_info.processed {
            panic_with_error!(&e, PoolRouterError::LiquidityNotFilled);
        }
        // it's safe to convert tps to u128 since it cannot be bigger than total tps which is u128
        let pool_tps = if pool_liquidity > U256::from_u32(&e, 0) {
            U256::from_u128(&e, rewards_config.tps)
                .mul(&pool_liquidity)
                .div(&reward_info.total_liquidity)
                .to_u128()
                .unwrap()
        } else {
            0
        };

        e.invoke_contract::<Val>(
            &pool_address,
            &Symbol::new(&e, "set_rewards_config"),
            Vec::from_array(
                &e,
                [
                    e.current_contract_address().to_val(),
                    rewards_config.expired_at.into_val(&e),
                    pool_tps.into_val(&e),
                ],
            ),
        );

        if pool_tps > 0 {
            // mark pool as configured to avoid reentrancy
            set_reward_tokens_detailed(&e, &asset, &(pool_liquidity, true));
        }

        Events::new(&e).config_rewards(asset, pool_address, pool_tps, rewards_config.expired_at);

        pool_tps
    }

    // Get rewards status for the pool, including amount available for the user
    //
    // # Arguments
    //
    // * `e` - The environment.
    // * `user` - The address of the user.
    // * `asset` - An asset symbol.
    //
    // # Returns
    //
    // A map of symbols to integers representing the rewards info.
    fn get_rewards_info(e: Env, user: Address, asset: Symbol) -> Map<Symbol, i128> {
        let pool_address = get_pool(&e, &asset);

        e.invoke_contract(
            &pool_address,
            &Symbol::new(&e, "get_rewards_info"),
            Vec::from_array(&e, [user.clone().into_val(&e)]),
        )
    }

    // Get amount of reward tokens available for the user to claim.
    //
    // # Arguments
    //
    // * `e` - The environment.
    // * `user` - The address of the user.
    // * `asset` - An asset symbol.
    //
    // # Returns
    //
    // The user reward as a u128.
    fn get_user_reward(e: Env, user: Address, asset: Symbol) -> u128 {
        let pool_address = get_pool(&e, &asset);

        e.invoke_contract(
            &pool_address,
            &Symbol::new(&e, "get_user_reward"),
            Vec::from_array(&e, [user.clone().into_val(&e)]),
        )
    }

    // Returns the total accumulated reward.
    //
    // # Arguments
    //
    // * `e` - The environment.
    // * `asset` - An asset symbol.
    //
    // # Returns
    //
    // The total accumulated reward as a u128.
    fn get_total_accumulated_reward(e: Env, asset: Symbol) -> u128 {
        let pool_address = get_pool(&e, &asset);

        e.invoke_contract(
            &pool_address,
            &Symbol::new(&e, "get_total_accumulated_reward"),
            Vec::new(&e),
        )
    }

    // Returns the total configured reward.
    //
    // # Arguments
    //
    // * `e` - The environment.
    // * `asset` - An asset symbol.
    //
    // # Returns
    //
    // The total configured reward as a u128.
    fn get_total_configured_reward(e: Env, asset: Symbol) -> u128 {
        let pool_address = get_pool(&e, &asset);

        e.invoke_contract(
            &pool_address,
            &Symbol::new(&e, "get_total_configured_reward"),
            Vec::new(&e),
        )
    }

    // Returns the total claimed reward.
    //
    // # Arguments
    //
    // * `e` - The environment.
    // * `asset` - An asset symbol.
    //
    // # Returns
    //
    // The total claimed reward as a u128.
    fn get_total_claimed_reward(e: Env, asset: Symbol) -> u128 {
        let pool_address = get_pool(&e, &asset);

        e.invoke_contract(
            &pool_address,
            &Symbol::new(&e, "get_total_claimed_reward"),
            Vec::new(&e),
        )
    }

    // Calculate difference between total configured reward and total claimed reward.
    // Helps to estimate the amount of missing reward tokens pool has configured to distribute
    fn get_total_outstanding_reward(e: Env, asset: Symbol) -> u128 {
        let pool_address = get_pool(&e, &asset);

        let configured_reward: u128 = e.invoke_contract(
            &pool_address,
            &Symbol::new(&e, "get_total_configured_reward"),
            Vec::new(&e),
        );
        let claimed_reward: u128 = e.invoke_contract(
            &pool_address,
            &Symbol::new(&e, "get_total_claimed_reward"),
            Vec::new(&e),
        );

        let rewards = get_rewards_manager(&e);
        let reward_token = rewards.storage().get_reward_token();
        let reward_token_client = SorobanTokenClient::new(&e, &reward_token);
        let mut pool_reward_balance = reward_token_client.balance(&pool_address) as u128;

        // handle edge case - if pool has reward token in reserves
        let pool_tokens: Vec<Address> =
            e.invoke_contract(&pool_address, &Symbol::new(&e, "get_tokens"), Vec::new(&e));

        match pool_tokens.first_index_of(reward_token) {
            Some(i) => {
                let pool_reserves: Vec<u128> = e.invoke_contract(
                    &pool_address,
                    &Symbol::new(&e, "get_reserves"),
                    Vec::new(&e),
                );
                let reward_token_reserve = pool_reserves.get(i).unwrap();
                pool_reward_balance -= reward_token_reserve;
            }
            None => {}
        }
        configured_reward.saturating_sub(claimed_reward + pool_reward_balance)
    }

    // Transfer outstanding reward to the pool
    fn distribute_outstanding_reward(e: Env, user: Address, from: Address, asset: Symbol) -> u128 {
        user.require_auth();
        require_rewards_admin_or_owner(&e, &user);

        let pool_address = get_pool(&e, &asset);

        let outstanding_reward = Self::get_total_outstanding_reward(e.clone(), asset.clone());
        let rewards = get_rewards_manager(&e);
        let reward_token = rewards.storage().get_reward_token();

        if from != e.current_contract_address() {
            SorobanTokenClient::new(&e, &reward_token).transfer_from(
                &e.current_contract_address(),
                &from,
                &pool_address,
                &(outstanding_reward as i128),
            );
        } else {
            SorobanTokenClient::new(&e, &reward_token).transfer(
                &e.current_contract_address(),
                &pool_address,
                &(outstanding_reward as i128),
            );
        }
        outstanding_reward
    }

    // Claims the reward.
    //
    // # Arguments
    //
    // * `e` - The environment.
    // * `user` - The address of the user.
    // * `asset` - An asset symbol.
    //
    // # Returns
    //
    // The amount of tokens rewarded to the user as a u128.
    fn claim(e: Env, user: Address, asset: Symbol) -> u128 {
        user.require_auth();

        let pool_address = get_pool(&e, &asset);

        let amount = e.invoke_contract(
            &pool_address,
            &symbol_short!("claim"),
            Vec::from_array(&e, [user.clone().into_val(&e)]),
        );

        Events::new(&e).claim(
            asset,
            user,
            pool_address,
            get_rewards_manager(&e).storage().get_reward_token(),
            amount,
        );

        amount
    }
}

// The `PoolsManagementTrait` trait provides the interface for managing liquidity pools.
#[contractimpl]
impl PoolsManagementTrait for PoolRouter {
    //  ___      ___       __        __    _____  ___
    // |"  \    /"  |     /""\      |" \  (\"   \|"  \
    //  \   \  //   |    /    \     ||  | |.\\   \    |
    //  /\\  \/.    |   /' /\  \    |:  | |: \.   \\  |
    // |: \.        |  //  __'  \   |.  | |.  \    \. |
    // |.  \    /:  | /   /  \\  \  /\  |\|    \    \ |
    // |___|\__/|___|(___/    \___)(__\_|_)\___|\____\)

    // Initializes a pool with custom arguments.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin initializing the pool.
    // * `assets` - A tuple of the base and quote asset Oracle Registry assets.
    // * `token_b` - A token address of the pool's quote token.
    // * `token_a_sac_address` - .
    // * `share_token_info` - A tuple of the LP token name and symbol.
    // * `fee_fraction` - The fee fraction for the pool (in basis points).
    // * `tier` - The risk tier of the target asset.
    // * `max_insurance` - The max coverage the pool may claim from the Insurance Fund.
    //
    // # Returns
    //
    // A tuple containing:
    // * The pool index hash.
    // * The address of the pool.
    fn init_pool(
        e: Env,
        admin: Address,
        assets: (Symbol, Symbol),
        token_b: Address,
        token_a_sac_address: Address,
        share_token_info: (String, String),
        fees_config: (u32, u32),
        tier: PoolTier,
        max_insurance: u128,
    ) -> Address {
        admin.require_auth();
        require_admin(&e, &admin);

        if fees_config.0 > MAX_POOL_FEE {
            panic_with_error!(&e, PoolRouterError::BadFee);
        }

        // base_asset
        match get_pool_base(&e, assets.clone().0) {
            Some(pool_address) => pool_address,
            None => deploy_pool(
                &e,
                &token_b,
                &assets,
                &token_a_sac_address,
                &share_token_info,
                &fees_config,
                &tier,
                max_insurance,
            ),
        }
    }

    //   _______    _______  ___________  ___________  _______   _______    ________
    //  /" _   "|  /"     "|("     _   ")("     _   ")/"     "| /"      \  /"       )
    // (: ( \___) (: ______) )__/  \\__/  )__/  \\__/(: ______)|:        |(:   \___/
    //  \/ \       \/    |      \\_ /        \\_ /    \/    |  |_____/   ) \___  \
    //  //  \ ___  // ___)_     |.  |        |.  |    // ___)_  //      /   __/  \\
    // (:   _(  _|(:      "|    \:  |        \:  |   (:      "||:  __   \  /" \   :)
    //  \_______)  \_______)     \__|         \__|    \_______)|__|  \___)(_______/

    fn query_pool_details(e: Env, asset: Symbol) -> PoolInfo {
        let pool_id = get_pool(&e, &asset);

        let pool_response: PoolInfo =
            e.invoke_contract(&pool_id, &Symbol::new(&e, "get_info"), Vec::new(&e));
        pool_response
    }

    fn query_all_pools_details(e: Env) -> Vec<PoolInfo> {
        let pools_vec = get_pools_vec(&e);

        let mut result = Vec::new(&e);
        for pool in pools_vec {
            let pool_response: PoolInfo =
                e.invoke_contract(&pool, &Symbol::new(&e, "get_info"), Vec::new(&e));

            result.push_back(pool_response);
        }

        result
    }

    fn get_pools(e: Env) -> Vec<Address> {
        get_pools_vec(&e)
    }

    fn get_total_liquidity_imbalance(e: Env) -> i128 {
        let pools = get_pools_vec(&e);

        let mut total_liquidity_imbalance = 0_i128;

        for idx in 0..pools.len() {
            let pool_address = pools.get(idx).unwrap();
            let pool_liquidity_imbalance: i128 = e.invoke_contract(
                &pool_address,
                &Symbol::new(&e, "get_liquidity_imbalance"),
                Vec::new(&e),
            );
            total_liquidity_imbalance =
                total_liquidity_imbalance.saturating_add(pool_liquidity_imbalance);
        }

        total_liquidity_imbalance
    }
}

// The `PoolPlaneInterface` trait provides the interface for interacting with a pool plane.
#[contractimpl]
impl PoolPlaneInterface for PoolRouter {
    // Sets the pool plane.
    // Pool plane is a contract which knows current state of every pool
    // and can be used to estimate swaps without calling pool contracts.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin user.
    // * `plane` - The address of the plane.
    fn set_pools_plane(e: Env, admin: Address, plane: Address) {
        admin.require_auth();
        AccessControl::new(&e).assert_address_has_role(&admin, &Role::Admin);

        set_pool_plane(&e, &plane);
    }

    // Returns the address of the pool plane.
    fn get_plane(e: Env) -> Address {
        get_pool_plane(&e)
    }
}

// The `TransferableContract` trait provides the interface for transferring ownership of the contract.
#[contractimpl]
impl TransferableContract for PoolRouter {
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
