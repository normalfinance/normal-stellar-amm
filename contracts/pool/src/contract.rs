use crate::errors::{PoolError, PoolValidationError};
use crate::events::Events as LiquidityPoolEvents;
use crate::events::PoolEvents;
use crate::incentives::get_incentives_manager;
use crate::interface::{
    AdminInterfaceTrait, IncentivesTrait, PoolCrunch, PoolTrait, UpgradeableContract,
    UpgradeableLPTokenTrait,
};
use crate::plane::update_plane;
use crate::plane_interface::Plane;
use crate::pool::{
    get_amount_out, get_amount_out_strict_receive, get_delta_a, get_net_liquidity_imbalance,
    get_oracle_price, rebalance,
    validate_oracle_price_with_pool,
};
use crate::storage::{
    get_base_asset, get_fee_fraction, get_insurance_claim, get_insurance_fund_from_router, get_is_killed_claim, get_is_killed_deposit, get_is_killed_swap, get_is_killed_withdraw, get_last_trade_ts, get_liquidity_minted_synthetic, get_max_liquidity_imbalance, get_mint_cap_fraction, get_plane, get_protocol_fee_a, get_protocol_fee_b, get_protocol_fee_fraction, get_quote_asset, get_reserve_a, get_reserve_b, get_router, get_status, get_tier, get_token_a, get_token_b, get_token_future_wasm, get_volume_30d, has_plane, set_base_asset, set_fee_fraction, set_insurance_claim, set_is_killed_claim, set_is_killed_deposit, set_is_killed_swap, set_is_killed_withdraw, set_last_trade_ts, set_liquidity_minted_synthetic, set_max_liquidity_imbalance, set_mint_cap_fraction, set_oracle_registry, set_plane, set_protocol_fee_a, set_protocol_fee_b, set_protocol_fee_fraction, set_quote_asset, set_reserve_a, set_reserve_b, set_router, set_status, set_tier, set_token_a, set_token_b, set_token_future_wasm, set_volume_30d
};
use crate::token::{burn_synthetic_tokens, create_lp_token_contract, transfer_a, transfer_b};
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
    require_operations_admin_or_owner, require_pause_admin_or_owner,
    require_pause_or_emergency_pause_admin_or_owner, require_rewards_admin_or_owner,
};
use incentives::events::Events as RewardEvents;
use incentives::storage::{PoolIncentivesStorageTrait, RewardTokenStorageTrait};
use reentrancy_guard::{enter, exit};
// use soroban_fixed_point_math::FixedPoint;
use soroban_fixed_point_math::SorobanFixedPoint;
use soroban_sdk::token::TokenClient as SorobanTokenClient;
use soroban_sdk::{
    contract, contractimpl, contractmeta, panic_with_error, symbol_short, Address, BytesN, Env,
    IntoVal, Map, Symbol, Vec, U256,
};
use token_lp::{
    burn_lp_tokens, get_token_lp, get_total_lp_tokens, get_user_balance_lp, mint_lp_tokens,
    put_token_lp, Client as LpTokenClient,
};
use upgrade::events::Events as UpgradeEvents;
use upgrade::{apply_upgrade, commit_upgrade, revert_upgrade};
use utils::constant::{
    FEE_MULTIPLIER, INSURANCE_A_MAX, INSURANCE_B_MAX, INSURANCE_C_MAX, INSURANCE_SPECULATIVE_MAX,
    MAX_POOL_FEE, MIN_LIQUIDITY,
};
use utils::math::safe_math::SafeMath;
use utils::state::oracle_registry::NormalAction;
use utils::state::pool::{InsuranceClaim, PoolConfig, PoolDetails, SwapDirection};
use utils::state::{
    pool::{InitializeAllParams, PoolInfo, PoolResponse, PoolStatus, PoolTier},
    token::AddressAndAmount,
};
use utils::token::transfer_token;
use utils::u256_math::ExtraMath;
use utils::validate;
use utils::validation::ensure_non_zero_u128;

contractmeta!(
    key = "Description",
    val = "Constant product AMM for synthetic assets automatically minting/burning the synthetic asset to maintain an oracle price peg"
);

#[contract]
pub struct Pool;

#[contractimpl]
impl PoolCrunch for Pool {
    // Initializes all the components of the liquidity pool.
    //
    // # Arguments
    //
    // * `params` - The params to initialize all the pool with.
    fn initialize_all(e: Env, params: InitializeAllParams) {
        // merge whole initialize process into one because lack of caching of VM components
        // https://github.com/stellar/rs-soroban-env/issues/827
        Self::init_pools_plane(e.clone(), params.plane);
        Self::initialize(e.clone(), params.config);
        Self::initialize_incentives_config(e.clone(), params.reward_config.reward_token);
    }
}

#[contractimpl]
impl PoolTrait for Pool {
    // Initializes the liquidity pool.
    //
    // # Arguments
    //
    // * `config` - The config to initialize the pool with.
    fn initialize(e: Env, config: PoolConfig) {
        let access_control = AccessControl::new(&e);
        if access_control.get_role_safe(&Role::Admin).is_some() {
            panic_with_error!(&e, PoolError::AlreadyInitialized);
        }
        access_control.set_role_address(&Role::Admin, &config.admin);
        access_control.set_role_address(
            &Role::EmergencyAdmin,
            &config.privileged_addrs.emergency_admin,
        );
        access_control
            .set_role_address(&Role::RewardsAdmin, &config.privileged_addrs.rewards_admin);
        access_control.set_role_address(
            &Role::OperationsAdmin,
            &config.privileged_addrs.operations_admin,
        );
        access_control.set_role_address(&Role::PauseAdmin, &config.privileged_addrs.pause_admin);
        access_control.set_role_addresses(
            &Role::EmergencyPauseAdmin,
            &config.privileged_addrs.emergency_pause_admins,
        );

        set_router(&e, &config.router);
        set_oracle_registry(&e, &config.oracle_registry);

        // validate oracle assets
        let (base_asset, quote_asset) = config.assets;

        get_oracle_price(&e, &base_asset, NormalAction::PoolInit);
        get_oracle_price(&e, &quote_asset, NormalAction::PoolInit);

        // deploy and initialize LP token contract
        let share_contract = create_lp_token_contract(
            &e,
            config.lp_token_info.token_wasm_hash,
            &config.token_a_sac_address,
            &config.token_b,
        );
        LpTokenClient::new(&e, &share_contract).initialize(
            &e.current_contract_address(),
            &7u32,
            &config.lp_token_info.name.into_val(&e),
            &config.lp_token_info.symbol.into_val(&e),
        );

        if config.fee_fraction > MAX_POOL_FEE {
            panic_with_error!(&e, PoolValidationError::FeeOutOfBounds);
        }

        put_token_lp(&e, share_contract);
        set_token_a(&e, &config.token_a_sac_address);
        set_token_b(&e, &config.token_b);
        set_tier(&e, &config.tier);
        set_status(&e, &PoolStatus::Initialized);
        set_base_asset(&e, &base_asset);
        set_quote_asset(&e, &quote_asset);
        set_fee_fraction(&e, &config.fee_fraction);
        set_insurance_claim(&e, &InsuranceClaim::new(config.max_insurance));
        set_max_liquidity_imbalance(&e, &0_u128);

        // update plane data for every pool update
        update_plane(&e);
    }

    //  ___      ___       __        __    _____  ___
    // |"  \    /"  |     /""\      |" \  (\"   \|"  \
    //  \   \  //   |    /    \     ||  | |.\\   \    |
    //  /\\  \/.    |   /' /\  \    |:  | |: \.   \\  |
    // |: \.        |  //  __'  \   |.  | |.  \    \. |
    // |.  \    /:  | /   /  \\  \  /\  |\|    \    \ |
    // |___|\__/|___|(___/    \___)(__\_|_)\___|\____\)

    // Handles a deposit of Token B into the liquidity pool and mints LP tokens to the user.
    //
    // This function performs the following:
    // - Validates that deposits are allowed and the user has authorized the action.
    // - Transfers Token B from the user to the pool contract.
    // - Updates the pool's reserves and rebalances the pool using oracle prices.
    // - Mints new LP tokens proportional to the deposit amount.
    // - Updates the user's reward tracking through the incentives manager.
    // - Emits a `deposit_liquidity` event with the deposit details.
    //
    // # Arguments
    // * `e` - Soroban environment reference.
    // * `user` - The address of the user making the deposit.
    // * `desired_amount` - The amount of Token B the user is depositing.
    // * `min_shares` - The amount of.
    //
    // # Returns
    // A tuple `(token_b_amount, shares_to_mint)`:
    // - `token_b_amount` — the actual amount deposited
    // - `shares_to_mint` — the amount of LP tokens minted for the user
    //
    // # Panics
    // - If deposits are currently disabled (`PoolDepositKilled`).
    // - If the user tries to initialize the pool without providing both tokens (`AllCoinsRequired`).
    // - If the user fails to authorize the transaction.
    //
    // # Side Effects
    // - Transfers Token B to the pool.
    // - Mints LP tokens to the user.
    // - Updates reserves, oracle-based pricing, reward checkpoints, and emits an event.
    fn deposit(
        e: Env,
        user: Address,
        desired_amount: u128,
        min_shares: u128,
    ) -> (u128, u128, i128) {
        user.require_auth();

        ensure_non_zero_u128(&e, desired_amount);

        enter(&e);

        if get_is_killed_deposit(&e) {
            panic_with_error!(e, PoolError::PoolDepositKilled);
        }

        let token_b = get_token_b(&e);
        let action = NormalAction::AddLiquidity;

        let (reserve_a, reserve_b) = (get_reserve_a(&e), get_reserve_b(&e));

        // Before actual changes were made to the pool, update total rewards data and refresh/initialize user reward
        let incentives = get_incentives_manager(&e);
        let total_shares = get_total_lp_tokens(&e);
        let user_shares = get_user_balance_lp(&e, &user);
        incentives
            .manager()
            .checkpoint_user(&user, total_shares, user_shares, 0);

        if reserve_a == 0 && reserve_b == 0 && desired_amount == 0 {
            panic_with_error!(&e, PoolValidationError::AllCoinsRequired);
        }

        // Rebalance the pool
        Self::rebalance(e.clone(), user.clone(), action.clone());

        // Increase reserves
        set_reserve_b(&e, &(reserve_b + desired_amount));

        // Deposit Token B
        transfer_token(
            &e,
            &token_b,
            &user,
            &e.current_contract_address(),
            &(desired_amount as i128),
        );

        let delta_a = new_reserve_a - reserve_a;
        let prev_value = get_liquidity_minted_synthetic(&e);
        set_liquidity_minted_synthetic(&e, &(prev_value + delta_a));

        // Now calculate how many new pool shares to mint
        let (new_reserve_a, new_reserve_b) = (get_reserve_a(&e), get_reserve_b(&e));

        // First deposit: mint MIN_LIQUIDITY to contract itself to prevent dust attacks
        if total_shares == 0 {
            mint_lp_tokens(&e, &e.current_contract_address(), MIN_LIQUIDITY as i128);
            let events = LiquidityPoolEvents::new(&e);
            events.permanently_locked_liquidity(MIN_LIQUIDITY);
            shares_to_mint = shares_to_mint.saturating_sub(MIN_LIQUIDITY);
        }

        let zero = 0;
        let new_total_shares = if reserve_a > zero && reserve_b > zero {
            let shares_a = new_reserve_a.fixed_mul_floor(&e, &total_shares, &reserve_a);
            let shares_b = new_reserve_b.fixed_mul_floor(&e, &total_shares, &reserve_b);
            shares_a.min(shares_b)
        } else {
            // if .mul doesn't fail, sqrt also won't -> safe to unwrap
            U256::from_u128(&e, new_reserve_a)
                .mul(&U256::from_u128(&e, new_reserve_b))
                .sqrt()
                .to_u128()
                .unwrap()
        };

        let shares_to_mint = new_total_shares - total_shares;
        if shares_to_mint < min_shares {
            panic_with_error!(&e, PoolValidationError::OutMinNotSatisfied);
        }
        mint_lp_tokens(&e, &user, shares_to_mint as i128);
        set_reserve_a(&e, &new_reserve_a);
        set_reserve_b(&e, &new_reserve_b);

        // Checkpoint resulting working balance
        incentives.manager().update_working_balance(
            &user,
            total_shares + shares_to_mint,
            user_shares + shares_to_mint,
        );

        // update plane data for every pool update
        update_plane(&e);

        // Finds how many synthetic tokens were minted/burned by finding the difference between reserve_a
        let delta_a: i128 = (new_reserve_a as i128).saturating_sub(reserve_a as i128);

        LiquidityPoolEvents::new(&e).deposit_liquidity(
            token_b.clone(),
            user,
            desired_amount,
            shares_to_mint,
            delta_a,
        );

        exit(&e);

        (desired_amount, shares_to_mint, delta_a)
    }

    // Swaps tokens in the pool by transferring an input token from the user and returning an output token,
    // ensuring pool invariants, oracle validity, and slippage constraints are upheld.
    //
    // This function performs:
    // - Authorization and safety checks (e.g., pool not killed, valid token indices, non-zero amounts).
    // - Oracle-based rebalancing before and after the swap.
    // - Fee-aware invariant enforcement using residue accounting.
    // - Slippage protection by enforcing `out_min`.
    // - State updates for reserves and volume tracking.
    // - Emits a `trade` event for indexing.
    //
    // # Arguments
    // * `e` - Soroban environment reference.
    // * `user` - Address of the user initiating the swap.
    // * `direction` - The direction of the swap: either buy or sell token_a.
    // * `in_amount` - Amount of the input token being sold.
    // * `out_min` - Minimum acceptable amount of output token (slippage guard).
    //
    // # Returns
    // * `u128` — The amount of the output token received by the user.
    //
    // # Panics
    // - If swaps are disabled (`PoolSwapKilled`)
    // - If reserves are empty
    // - If the resulting output amount is below `out_min`
    // - If the invariant does not hold post-swap
    //
    // # Side Effects
    // - Transfers `in_amount` from the user and sends `out` to them.
    // - Updates reserves, oracle TWAPs, and volume.
    // - Emits a trade event and updates on-chain plane data.
    fn swap(
        e: Env,
        user: Address,
        direction: SwapDirection,
        in_amount: u128,
        out_min: u128,
    ) -> (u128, i128, i128) {
        user.require_auth();

        enter(&e);

        if get_is_killed_swap(&e) {
            panic_with_error!(e, PoolError::PoolSwapKilled);
        }

        ensure_non_zero_u128(&e, in_amount);

        let action = NormalAction::Swap;

        let now = e.ledger().timestamp();

        let reserve_a_before_prior_rebalance = get_reserve_a(&e) as i128;

        // Rebalance the pool
        Self::rebalance(e.clone(), user.clone(), action.clone());

        let reserve_a = get_reserve_a(&e);
        let reserve_b = get_reserve_b(&e);
        let reserves = Vec::from_array(&e, [reserve_a, reserve_b]);
        let tokens = Self::get_tokens(e.clone());

        let delta_a_prior = reserve_a_before_prior_rebalance.saturating_sub(reserve_a as i128);

        let (in_idx, out_idx) = if direction == SwapDirection::Buy {
            (1, 0)
        } else {
            (0, 1)
        };

        let reserve_sell = reserves.get(in_idx).unwrap();
        let reserve_buy = reserves.get(out_idx).unwrap();
        if reserve_sell == 0 || reserve_buy == 0 {
            panic_with_error!(&e, PoolValidationError::EmptyPool);
        }

        let (out, total_fee) = get_amount_out(&e, in_amount, reserve_sell, reserve_buy);
        let protocol_fee = (total_fee * (get_protocol_fee_fraction(&e) as u128)) / FEE_MULTIPLIER;
        let lp_fee = total_fee - protocol_fee;

        if out < out_min {
            panic_with_error!(&e, PoolValidationError::OutMinNotSatisfied);
        }

        if in_idx == 0 {
            set_reserve_a(&e, &(reserve_a + in_amount));
        } else {
            set_reserve_b(&e, &(reserve_b + in_amount));
        }

        let (mut new_reserve_a, mut new_reserve_b) = (get_reserve_a(&e), get_reserve_b(&e));

        // Transfer the amount being sold to the contract
        let sell_token = tokens.get(in_idx).unwrap();
        let sell_token_client = SorobanTokenClient::new(&e, &sell_token);
        sell_token_client.transfer(&user, &e.current_contract_address(), &(in_amount as i128));

        // residue_numerator and residue_denominator are the amount that the invariant considers after
        // deducting the fee, scaled up by FEE_MULTIPLIER to avoid fractions
        let base_fee_fraction = get_fee_fraction(&e) as u128; // e.g. 30 = 0.3%
        let protocol_fee_frac =
            (base_fee_fraction * (get_protocol_fee_fraction(&e) as u128)) / FEE_MULTIPLIER; // e.g. 30 * 50 / 100 = 0.15% admin fee
        let pool_fee_frac = base_fee_fraction - protocol_fee_frac; // e.g. 15 = 0.15% stays in pool
        let residue_numerator = FEE_MULTIPLIER - pool_fee_frac; // e.g. 10000 - 15  = 9985
        let residue_denominator = U256::from_u128(&e, FEE_MULTIPLIER);

        let new_invariant_factor = |reserve: u128, old_reserve: u128, out: u128| {
            if reserve - old_reserve > out {
                residue_denominator
                    .mul(&U256::from_u128(&e, old_reserve))
                    .add(
                        &U256::from_u128(&e, residue_numerator)
                            .mul(&U256::from_u128(&e, reserve - old_reserve - out)),
                    )
            } else {
                residue_denominator
                    .mul(&U256::from_u128(&e, old_reserve))
                    .add(&residue_denominator.mul(&U256::from_u128(&e, reserve)))
                    .sub(&residue_denominator.mul(&U256::from_u128(&e, old_reserve + out)))
            }
        };

        let (out_a, out_b) = if out_idx == 0 { (out, 0) } else { (0, out) };

        let new_inv_a = new_invariant_factor(new_reserve_a, reserve_a, out_a);
        let new_inv_b = new_invariant_factor(new_reserve_b, reserve_b, out_b);
        let old_inv_a = residue_denominator.mul(&U256::from_u128(&e, reserve_a));
        let old_inv_b = residue_denominator.mul(&U256::from_u128(&e, reserve_b));

        if new_inv_a.mul(&new_inv_b) < old_inv_a.mul(&old_inv_b) {
            panic_with_error!(&e, PoolError::InvariantDoesNotHold);
        }

        if out_idx == 0 {
            transfer_a(&e, &user, out_a);
            new_reserve_a = new_reserve_a - out_a;
            new_reserve_b = new_reserve_b - protocol_fee;
            set_protocol_fee_b(&e, &(get_protocol_fee_b(&e) + protocol_fee));
        } else {
            transfer_b(&e, &user, out_b);
            new_reserve_a = new_reserve_a - protocol_fee;
            new_reserve_b = new_reserve_b - out_b;
            set_protocol_fee_a(&e, &(get_protocol_fee_a(&e) + protocol_fee));
        }
        set_reserve_a(&e, &new_reserve_a);
        set_reserve_b(&e, &new_reserve_b);

        // Update volume metrics
        let quote_asset_amount = if in_idx == 1 { in_amount } else { out };
        crate::pool::update_volume(&e, quote_asset_amount, now);

        // Rebalance the pool
        Self::rebalance(e.clone(), user.clone(), action.clone());

        let reserve_a_final = get_reserve_a(&e) as i128;
        let delta_a_post = reserve_a_final.saturating_sub(reserve_a as i128);

        // update plane data for every pool update
        update_plane(&e);

        LiquidityPoolEvents::new(&e).swap(
            user,
            sell_token,
            tokens.get(out_idx).unwrap(),
            in_amount,
            out,
            delta_a_prior,
            delta_a_post,
        );
        LiquidityPoolEvents::new(&e)
            .update_reserves(Vec::from_array(&e, [new_reserve_a, new_reserve_b]));

        exit(&e);

        (out, delta_a_prior, delta_a_post)
    }

    // Estimates the result of a swap operation.
    //
    // # Arguments
    //
    // * `direction` - The direction of the swap: either buy or sell token_a.
    // * `in_amount` - The amount of the input token to be swapped.
    //
    // # Returns
    //
    // A tuple containing the estimated amount of the output token that would be received and the amount of token_a to mint/burn.
    fn estimate_swap(e: Env, direction: SwapDirection, in_amount: u128) -> (u128, i128) {
        ensure_non_zero_u128(&e, in_amount);

        let reserve_a = get_reserve_a(&e);
        let reserve_b = get_reserve_b(&e);

        let (in_idx, out_idx) = if direction == SwapDirection::Buy {
            (1, 0)
        } else {
            (0, 1)
        };

        let reserves = Vec::from_array(&e, [reserve_a, reserve_b]);
        let reserve_sell = reserves.get(in_idx).unwrap();
        let reserve_buy = reserves.get(out_idx).unwrap();

        let out = get_amount_out(&e, in_amount, reserve_sell, reserve_buy).0;

        let base_oracle_price_data = get_oracle_price(&e, &get_base_asset(&e), NormalAction::Swap);
        let quote_oracle_price_data =
            get_oracle_price(&e, &get_quote_asset(&e), NormalAction::Swap);
        let delta_a = get_delta_a(
            &e,
            reserve_a,
            reserve_b,
            base_oracle_price_data.last_oracle_price_twap,
            quote_oracle_price_data.last_oracle_price_twap,
        );

        (out, delta_a)
    }

    // Swaps tokens in the pool.
    // Perform an exchange between two coins with strict amount to receive.
    //
    // # Arguments
    //
    // * `user` - The address of the user swapping the tokens.
    // * `direction` - The direction of the swap: either buy or sell token_a.
    // * `out_amount` - Amount of out_idx being exchanged
    // * `in_max` - Maximum amount of in_idx to send
    //
    // # Returns
    //
    // The amount of the input token sent.
    fn swap_strict_receive(
        e: Env,
        user: Address,
        direction: SwapDirection,
        out_amount: u128,
        in_max: u128,
    ) -> (u128, i128, i128) {
        user.require_auth();

        ensure_non_zero_u128(&e, out_amount);

        enter(&e);

        if get_is_killed_swap(&e) {
            panic_with_error!(e, PoolError::PoolSwapKilled);
        }

        let now = e.ledger().timestamp();
        let action = NormalAction::Swap;

        let reserve_a_before_prior_rebalance = get_reserve_a(&e) as i128;

        // Rebalance the pool
        Self::rebalance(e.clone(), user.clone(), action.clone());

        let (in_idx, out_idx) = if direction == SwapDirection::Buy {
            (1, 0)
        } else {
            (0, 1)
        };

        let reserve_a = get_reserve_a(&e);
        let reserve_b = get_reserve_b(&e);
        let reserves = Vec::from_array(&e, [reserve_a, reserve_b]);
        let tokens = Self::get_tokens(e.clone());

        let delta_a_prior = reserve_a_before_prior_rebalance.saturating_sub(reserve_a as i128);

        let reserve_sell = reserves.get(in_idx).unwrap();
        let reserve_buy = reserves.get(out_idx).unwrap();
        if reserve_sell == 0 || reserve_buy == 0 {
            panic_with_error!(&e, PoolValidationError::EmptyPool);
        }

        let (in_amount, total_fee) =
            get_amount_out_strict_receive(&e, out_amount, reserve_sell, reserve_buy);

        if in_amount > in_max {
            panic_with_error!(&e, PoolValidationError::InMaxNotSatisfied);
        }

        if in_idx == 0 {
            set_reserve_a(&e, &(reserve_a + in_amount));
        } else {
            set_reserve_b(&e, &(reserve_b + in_amount));
        }

        let (mut new_reserve_a, mut new_reserve_b) = (get_reserve_a(&e), get_reserve_b(&e));

        // Transfer the amount being sold to the contract
        let sell_token = tokens.get(in_idx).unwrap();
        let sell_token_client = SorobanTokenClient::new(&e, &sell_token);
        sell_token_client.transfer(&user, &e.current_contract_address(), &(in_max as i128));

        // Return the difference
        sell_token_client.transfer(
            &e.current_contract_address(),
            &user,
            &((in_max - in_amount) as i128),
        );

        // residue_numerator and residue_denominator are the amount that the invariant considers after
        // deducting the fee, scaled up by FEE_MULTIPLIER to avoid fractions
        let base_fee_fraction = get_fee_fraction(&e) as u128; // e.g. 30 = 0.3%
        let protocol_fee_frac =
            (base_fee_fraction * (get_protocol_fee_fraction(&e) as u128)) / FEE_MULTIPLIER; // e.g. 30 * 5000 / 10000 = 0.15% admin fee
        let pool_fee_frac = base_fee_fraction - protocol_fee_frac; // e.g. 15 = 0.15% stays in pool
        let residue_numerator = FEE_MULTIPLIER - pool_fee_frac; // e.g. 10000 - 15  = 9985
        let residue_denominator = U256::from_u128(&e, FEE_MULTIPLIER);

        let new_invariant_factor = |reserve: u128, old_reserve: u128, out: u128| {
            if reserve - old_reserve > out {
                residue_denominator
                    .mul(&U256::from_u128(&e, old_reserve))
                    .add(
                        &U256::from_u128(&e, residue_numerator)
                            .mul(&U256::from_u128(&e, reserve - old_reserve - out)),
                    )
            } else {
                residue_denominator
                    .mul(&U256::from_u128(&e, old_reserve))
                    .add(&residue_denominator.mul(&U256::from_u128(&e, reserve)))
                    .sub(&residue_denominator.mul(&U256::from_u128(&e, old_reserve + out)))
            }
        };

        let (out_a, out_b) = if in_idx == 0 {
            (out_amount, 0)
        } else {
            (0, out_amount)
        };

        let new_inv_a = new_invariant_factor(new_reserve_a, reserve_a, out_a);
        let new_inv_b = new_invariant_factor(new_reserve_b, reserve_b, out_b);
        let old_inv_a = residue_denominator.mul(&U256::from_u128(&e, reserve_a));
        let old_inv_b = residue_denominator.mul(&U256::from_u128(&e, reserve_b));

        if new_inv_a.mul(&new_inv_b) < old_inv_a.mul(&old_inv_b) {
            panic_with_error!(&e, PoolError::InvariantDoesNotHold);
        }

        // collect protocol_fee on input side
        let protocol_fee = (total_fee * (get_protocol_fee_fraction(&e) as u128)) / FEE_MULTIPLIER;
        let lp_fee = total_fee - protocol_fee;

        // give trader the exact out_amount
        if out_idx == 0 {
            transfer_a(&e, &user, out_amount);
            new_reserve_a = new_reserve_a - out_amount;
            new_reserve_b = new_reserve_b - protocol_fee;
            set_protocol_fee_b(&e, &(get_protocol_fee_b(&e) + protocol_fee));
        } else {
            transfer_b(&e, &user, out_amount);
            new_reserve_a = new_reserve_a - protocol_fee;
            new_reserve_b = new_reserve_b - out_amount;
            set_protocol_fee_a(&e, &(get_protocol_fee_a(&e) + protocol_fee));
        }
        set_reserve_a(&e, &new_reserve_a);
        set_reserve_b(&e, &new_reserve_b);

        // Update volume metrics
        let quote_asset_amount = if in_idx == 1 { in_amount } else { out_amount };
        crate::pool::update_volume(&e, quote_asset_amount, now);

        // Rebalance the pool
        Self::rebalance(e.clone(), user.clone(), action.clone());

        let reserve_a_final = get_reserve_a(&e) as i128;
        let delta_a_post = reserve_a_final.saturating_sub(reserve_a as i128);

        // update plane data for every pool update
        update_plane(&e);

        LiquidityPoolEvents::new(&e).swap(
            user.clone(),
            sell_token,
            tokens.get(out_idx).unwrap(),
            in_amount,
            out_amount,
            delta_a_prior,
            delta_a_post,
        );

        exit(&e);

        (in_amount, delta_a_prior, delta_a_post)
    }

    // Estimates the result of a swap_strict_receive operation.
    //
    // # Arguments
    //
    // * `direction` - The direction of the swap: either buy or sell token_a.
    // * `out_amount` - The amount of the output token to be received.
    //
    // # Returns
    //
    // A tuple containing the estimated amount of the output token that would be received and the amount of token_a to mint/burn.
    fn estimate_swap_strict_receive(
        e: Env,
        direction: SwapDirection,
        out_amount: u128,
    ) -> (u128, i128) {
        ensure_non_zero_u128(&e, out_amount);

        let (in_idx, out_idx) = if direction == SwapDirection::Buy {
            (1, 0)
        } else {
            (0, 1)
        };

        let reserve_a = get_reserve_a(&e);
        let reserve_b = get_reserve_b(&e);
        let reserves = Vec::from_array(&e, [reserve_a, reserve_b]);
        let reserve_sell = reserves.get(in_idx).unwrap();
        let reserve_buy = reserves.get(out_idx).unwrap();

        let out = get_amount_out_strict_receive(&e, out_amount, reserve_sell, reserve_buy).0;

        let base_oracle_price_data = get_oracle_price(&e, &get_base_asset(&e), NormalAction::Swap);
        let quote_oracle_price_data =
            get_oracle_price(&e, &get_quote_asset(&e), NormalAction::Swap);
        let delta_a = get_delta_a(
            &e,
            reserve_a,
            reserve_b,
            base_oracle_price_data.last_oracle_price_twap,
            quote_oracle_price_data.last_oracle_price_twap,
        );

        (out, delta_a)
    }

    // Withdraws liquidity from the pool by burning the user's LP tokens and transferring back Token B,
    // while updating rewards, rebalancing the pool, and maintaining invariant and accounting consistency.
    //
    // This function performs:
    // - Authorization and validation checks (e.g., withdrawals enabled).
    // - Burns LP tokens from the user based on the provided `share_amount`.
    // - Transfers the corresponding amount of Token B back to the user.
    // - Rebalances the pool using current oracle prices for base and quote assets.
    // - Updates the incentive/reward checkpoint and internal state.
    // - Emits a `withdraw_liquidity` event for tracking.
    //
    // # Arguments
    // * `e` - Soroban environment reference.
    // * `user` - The address of the user requesting a withdrawal.
    // * `share_amount` - The number of LP tokens to burn and withdraw.
    // * `min_amounts` - The
    //
    // # Returns
    // * `u128` — The amount of Token B transferred back to the user (equal to `share_amount`).
    //
    // # Panics
    // - If withdrawals are disabled (`PoolWithdrawKilled`)
    // - If the user is not authorized
    // - If the reserves are insufficient to fulfill the withdrawal
    //
    // # Side Effects
    // - Burns LP tokens from the user.
    // - Transfers Token B from the pool to the user.
    // - Rebalances the pool using oracle data.
    // - Updates incentive tracking and on-chain accounting.
    // - Emits a liquidity withdrawal event.
    fn withdraw(e: Env, user: Address, share_amount: u128, min_amounts: Vec<u128>) -> (u128, i128) {
        user.require_auth();

        ensure_non_zero_u128(&e, share_amount);

        if min_amounts.len() != 2 {
            panic_with_error!(&e, PoolValidationError::WrongInputVecSize);
        }

        // FIXME: will there be issues here"
        enter(&e);

        if get_is_killed_withdraw(&e) {
            panic_with_error!(e, PoolError::PoolWithdrawKilled);
        }

        let action = NormalAction::RemoveLiquidity;

        // Rebalance the pool
        Self::rebalance(e.clone(), user.clone(), action.clone());

        // Before actual changes were made to the pool, update total rewards data and refresh user reward
        let incentives = get_incentives_manager(&e);
        let total_shares = get_total_lp_tokens(&e);
        if total_shares - share_amount < MIN_LIQUIDITY {
            panic_with_error!(e, PoolError::WithdrawExceedsMinLiquidity);
        }
        let user_shares = get_user_balance_lp(&e, &user);
        incentives
            .manager()
            .checkpoint_user(&user, total_shares, user_shares, 0);

        burn_lp_tokens(&e, &user, share_amount);

        let (reserve_a, reserve_b) = (get_reserve_a(&e), get_reserve_b(&e));

        // Now calculate the withdraw amounts
        let mut out_a = reserve_a.fixed_mul_floor(&e, &share_amount, &total_shares);
        let out_b = reserve_b.fixed_mul_floor(&e, &share_amount, &total_shares);

        let min_a = min_amounts.get(0).unwrap();
        let min_b = min_amounts.get(1).unwrap();

        if out_a < min_a || out_b < min_b {
            panic_with_error!(&e, PoolValidationError::OutMinNotSatisfied);
        }

        // sfdsf
        let liquidity_minted_synthetic = get_liquidity_minted_synthetic(&e);
        let token_a_to_burn =
            liquidity_minted_synthetic.fixed_mul_floor(&e, &share_amount, &total_shares);

        burn_synthetic_tokens(&e, &e.current_contract_address(), token_a_to_burn);
        out_a = out_a - token_a_to_burn;

        // Transfer and update
        transfer_a(&e, &user, out_a);
        transfer_b(&e, &user, out_b);
        let new_reserve_a = reserve_a - out_a;
        let new_reserve_b = reserve_b - out_b;
        set_reserve_a(&e, &new_reserve_a);
        set_reserve_b(&e, &new_reserve_b);

        // Rebalance the pool
        Self::rebalance(e.clone(), user.clone(), action.clone());

        let reserve_a_after_rebalance = get_reserve_a(&e);

        // Checkpoint resulting working balance
        incentives.manager().update_working_balance(
            &user,
            total_shares - share_amount,
            user_shares - share_amount,
        );

        // update plane data for every pool update
        update_plane(&e);

        // Finds how many synthetic tokens were minted/burned by finding the difference between reserve_a
        let delta_a: i128 = (reserve_a_after_rebalance as i128).saturating_sub(reserve_a as i128);

        LiquidityPoolEvents::new(&e).withdraw_liquidity(
            get_token_b(&e),
            user,
            share_amount,
            share_amount,
            delta_a,
        );

        exit(&e);

        (share_amount, delta_a)
    }

    //   _______    _______  ___________  ___________  _______   _______    ________
    //  /" _   "|  /"     "|("     _   ")("     _   ")/"     "| /"      \  /"       )
    // (: ( \___) (: ______) )__/  \\__/  )__/  \\__/(: ______)|:        |(:   \___/
    //  \/ \       \/    |      \\_ /        \\_ /    \/    |  |_____/   ) \___  \
    //  //  \ ___  // ___)_     |.  |        |.  |    // ___)_  //      /   __/  \\
    // (:   _(  _|(:      "|    \:  |        \:  |   (:      "||:  __   \  /" \   :)
    //  \_______)  \_______)     \__|         \__|    \_______)|__|  \___)(_______/

    // Returns the pool's share token address.
    fn share_id(e: Env) -> Address {
        get_token_lp(&e)
    }

    fn get_total_shares(e: Env) -> u128 {
        get_total_lp_tokens(&e)
    }

    fn get_tokens(e: Env) -> Vec<Address> {
        Vec::from_array(&e, [get_token_a(&e), get_token_b(&e)])
    }

    fn get_privileged_addrs(e: Env) -> Map<Symbol, Vec<Address>> {
        let access_control = AccessControl::new(&e);
        let mut result: Map<Symbol, Vec<Address>> = Map::new(&e);
        for role in [
            Role::Admin,
            Role::EmergencyAdmin,
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

    fn get_reserves(e: Env) -> Vec<u128> {
        Vec::from_array(&e, [get_reserve_a(&e), get_reserve_b(&e)])
    }

    fn get_fee_fraction(e: Env) -> u32 {
        crate::storage::get_fee_fraction(&e)
    }

    fn get_mint_cap_fraction(e: Env) -> u32 {
        crate::storage::get_mint_cap_fraction(&e)
    }

    fn get_insurance_claim(e: Env) -> InsuranceClaim {
        crate::storage::get_insurance_claim(&e)
    }

    fn get_info(e: Env) -> PoolInfo {
        let pool_response = PoolResponse {
            pool: PoolDetails {
                assets: (get_base_asset(&e), get_quote_asset(&e)),
                status: get_status(&e),
                tier: get_tier(&e),
                fee_fraction: get_fee_fraction(&e),
                protocol_fee_fraction: get_protocol_fee_fraction(&e),
                insurance: get_insurance_claim(&e)
            },
            token_a: AddressAndAmount {
                address: get_token_a(&e),
                amount: get_reserve_a(&e),
            },
            token_b: AddressAndAmount {
                address: get_token_b(&e),
                amount: get_reserve_b(&e),
            },
            token_share: AddressAndAmount {
                address: get_token_lp(&e),
                amount: get_total_lp_tokens(&e),
            },
        };

        PoolInfo {
            pool_address: e.current_contract_address(),
            pool_response,
        }
    }

    fn get_liquidity_imbalance(e: Env) -> i128 {
        let action = NormalAction::Rebalance;

        let base_oracle_price_data = get_oracle_price(&e, &get_base_asset(&e), action);
        let quote_oracle_price_data = get_oracle_price(&e, &get_quote_asset(&e), action);

        get_net_liquidity_imbalance(
            &e,
            base_oracle_price_data.last_oracle_price_twap,
            quote_oracle_price_data.last_oracle_price_twap,
        )
    }

    // Returns the protocol fees accumulated in the pool.
    fn get_protocol_fees(e: Env) -> Vec<u128> {
        Vec::from_array(&e, [get_protocol_fee_a(&e), get_protocol_fee_b(&e)])
    }
}

#[contractimpl]
impl AdminInterfaceTrait for Pool {
    //  ___      ___       __        __    _____  ___
    // |"  \    /"  |     /""\      |" \  (\"   \|"  \
    //  \   \  //   |    /    \     ||  | |.\\   \    |
    //  /\\  \/.    |   /' /\  \    |:  | |: \.   \\  |
    // |: \.        |  //  __'  \   |.  | |.  \    \. |
    // |.  \    /:  | /   /  \\  \  /\  |\|    \    \ |
    // |___|\__/|___|(___/    \___)(__\_|_)\___|\____\)

    fn rebalance(e: Env, admin: Address, action: NormalAction) {
        admin.require_auth();
        require_operations_admin_or_owner(&e, &admin);

        enter(&e);

        let action = NormalAction::Rebalance;

        let base_oracle_price_data = get_oracle_price(&e, &get_base_asset(&e), action);
        let quote_oracle_price_data = get_oracle_price(&e, &get_quote_asset(&e), action);

        validate_oracle_price_with_pool(
            &e,
            base_oracle_price_data.last_oracle_price_twap,
            quote_oracle_price_data.last_oracle_price_twap,
            action,
        );

        rebalance(
            &e,
            base_oracle_price_data.last_oracle_price_twap,
            quote_oracle_price_data.last_oracle_price_twap,
        );

        exit(&e);
    }

    // Pays an insurance claim.
    //
    // # Arguments
    // * `e` - Soroban environment reference.
    // * `sender` - The address of the caller (can only be the Insurance Fund).
    // * `insurance_vault_amount` - The balance of the calling contract's holdings.
    //
    // # Returns
    // * `u128` — The amount of insurnace needed to withdraw.
    //
    fn pay_insurance_claim(e: Env, sender: Address, insurance_vault_amount: u128) -> u128 {
        sender.require_auth();

        enter(&e);

        let insurance_fund = get_insurance_fund_from_router(&e);

        if sender != insurance_fund {
            panic_with_error!(&e, PoolError::Unauthorized);
        }

        // check pool has liquidity deficit

        let now = e.ledger().timestamp();
        let action = NormalAction::ClaimInsurance;

        // "Pool is in settlement mode"
        let status = get_status(&e);
        validate!(
            &e,
            status == PoolStatus::Settlement,
            PoolError::PoolActionPaused
        );

        let base_oracle_price_data = get_oracle_price(&e, &get_base_asset(&e), action);
        let quote_oracle_price_data = get_oracle_price(&e, &get_quote_asset(&e), action);

        validate_oracle_price_with_pool(
            &e,
            base_oracle_price_data.last_oracle_price_twap,
            quote_oracle_price_data.last_oracle_price_twap,
            action,
        );

        // TODO: validate pool balances?

        let max_liquidity_imbalance = get_max_liquidity_imbalance(&e);

        let excess_liquidity_imbalance = if max_liquidity_imbalance > 0 {
            let net_liquidity_imbalance = get_net_liquidity_imbalance(
                &e,
                base_oracle_price_data.last_oracle_price_twap,
                quote_oracle_price_data.last_oracle_price_twap,
            );

            net_liquidity_imbalance.saturating_sub(max_liquidity_imbalance as i128)
        } else {
            0
        };

        validate!(
            &e,
            excess_liquidity_imbalance > 0,
            PoolError::LiquidityDeficitBelowThreshold
        );

        let insurance_claim = get_insurance_claim(&e);
        let max_insurance = insurance_claim.max_insurance;
        let settled_insurance = insurance_claim.settled_insurance;
        validate!(
            &e,
            max_insurance >= settled_insurance,
            PoolError::SettledExceedsMax
        );
        let max_insurance_withdraw = max_insurance - settled_insurance;

        validate!(
            &e,
            max_insurance_withdraw > 0,
            PoolError::MaxIFWithdrawReached
        );

        let insurance_withdraw = (excess_liquidity_imbalance as u128)
            .min(max_insurance_withdraw)
            .min(insurance_vault_amount.saturating_sub(1));

        validate!(&e, insurance_withdraw > 0, PoolError::NoIFWithdrawAvailable);

        // Update the Insurance Claim
        let mut updated_insurance_claim = insurance_claim.clone();
        updated_insurance_claim.rev_withdraw_since_last_settle = updated_insurance_claim
            .rev_withdraw_since_last_settle
            .safe_add(&e, insurance_withdraw as i128);

        updated_insurance_claim.settled_insurance = updated_insurance_claim
            .settled_insurance
            .safe_add(&e, insurance_withdraw);

        validate!(
            &e,
            updated_insurance_claim.settled_insurance
                <= updated_insurance_claim.max_insurance,
            PoolError::MaxIFWithdrawReached
        );

        updated_insurance_claim.last_revenue_withdraw_ts = now;
        set_insurance_claim(&e, &updated_insurance_claim);

        // Update the reserves
        let reserve_b = get_reserve_b(&e);
        set_reserve_b(&e, &(reserve_b + insurance_withdraw));

        // Deposit token_b from Insurance Fund to Pool
        transfer_token(
            &e,
            &get_token_b(&e),
            &sender,
            &e.current_contract_address(),
            &(insurance_withdraw as i128),
        );

        // Rebalance
        rebalance(
            &e,
            base_oracle_price_data.last_oracle_price_twap,
            quote_oracle_price_data.last_oracle_price_twap,
        );

        exit(&e);

        insurance_withdraw
    }

    // Claims the protocol fees accumulated in the pool.
    fn claim_protocol_fees(e: Env, admin: Address, destination: Address) -> Vec<u128> {
        admin.require_auth();
        require_operations_admin_or_owner(&e, &admin);

        let token_a = get_token_a(&e);
        let token_b = get_token_b(&e);

        let fee_a = get_protocol_fee_a(&e);
        let fee_b = get_protocol_fee_b(&e);

        if fee_a == 0 && fee_b == 0 {
            return Vec::from_array(&e, [0, 0]);
        }

        // TODO: Pay premium to the Insurance Fund
        // let insurance_fund = get_insurance_fund_from_router(&e);
        // let insurance_fund_rate_response = e.try_invoke_contract::<i32, soroban_sdk::Error>(
        //     &insurance_fund,
        //     &Symbol::new(&e, "get_rate"),
        //     Vec::from_array(&e, [e.current_contract_address().to_val()])
        // );

        // match insurance_fund_rate_response {
        //     Ok(Err(_)) | Err(_) => panic_with_error!(&e, InsuranceFundError::AdminNotSet),
        //     Ok(Ok(rate)) => {
        //         let insurance_claim = get_insurance_claim(&e);

        //         let mut insurance_premium_paid: u128 = 0;

        //         if rate > 0 {
        //             let volume_30d = get_volume_30d(&e);
        //             let estimated_annual_volume = volume_30d.fixed_mul_floor(365, 30).unwrap();

        //             let total_annual_premium = insurance_claim.max_insurance
        //                 .fixed_mul_floor(insurance_premium_rate as u128, PRICE_PRECISION)
        //                 .unwrap();

        //             let premium_per_dollar_swapped = total_annual_premium.safe_div(
        //                 &e,
        //                 estimated_annual_volume
        //             );

        //             // Lesser of premium or what's left of protocol fee
        //             let insurance_premium_to_pay = quote_asset_amount
        //                 .safe_mul(&e, premium_per_dollar_swapped)
        //                 .min(protocol_fee_amount);

        //             if insurance_premium_to_pay > 0 {
        //                 // TODO: must we also call the Pool to update last_revenue_withdraw_ts and rev_withdraw_since_last_settle?

        //                 insurance_premium_paid = e.invoke_contract(
        //                     &insurance_fund,
        //                     &Symbol::new(&e, "pay_premium"),
        //                     Vec::from_array(&e, [
        //                         e.current_contract_address().to_val(),
        //                         insurance_premium_to_pay.into_val(&e),
        //                     ])
        //                 );

        //                 protocol_fee_amount = protocol_fee_amount - insurance_premium_to_pay;
        //             }
        //         }
        //     }
        // }

        if fee_a > 0 {
            SorobanTokenClient::new(&e, &token_a).transfer(
                &e.current_contract_address(),
                &destination,
                &(fee_a as i128),
            );
            set_protocol_fee_a(&e, &0);
            LiquidityPoolEvents::new(&e).claim_protocol_fee(token_a, destination.clone(), fee_a);
        }
        if fee_b > 0 {
            SorobanTokenClient::new(&e, &token_b).transfer(
                &e.current_contract_address(),
                &destination,
                &(fee_b as i128),
            );
            set_protocol_fee_b(&e, &0);
            LiquidityPoolEvents::new(&e).claim_protocol_fee(token_b, destination, fee_b);
        }

        Vec::from_array(&e, [fee_a, fee_b])
    }

    fn delist(e: Env, admin: Address) {
        admin.require_auth();
        require_operations_admin_or_owner(&e, &admin);

        set_status(&e, &PoolStatus::Settlement);

        // TODO: finish any missing implementation
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

    fn set_fee(e: Env, admin: Address, fee_fraction: u32) {
        admin.require_auth();
        require_operations_admin_or_owner(&e, &admin);

        if fee_fraction > MAX_POOL_FEE {
            panic_with_error!(&e, PoolValidationError::FeeOutOfBounds);
        }

        set_fee_fraction(&e, &fee_fraction);
    }

    fn set_tier(e: Env, admin: Address, tier: PoolTier) {
        admin.require_auth();
        require_operations_admin_or_owner(&e, &admin);

        set_tier(&e, &tier);
    }

    fn set_status(e: Env, admin: Address, status: PoolStatus) {
        admin.require_auth();
        require_operations_admin_or_owner(&e, &admin);

        set_status(&e, &status);

        // Automatically recover minimum liquidity when pool is delisted
        if status == PoolStatus::Delisted {
            let contract_address = e.current_contract_address();
            let locked_balance = get_user_balance_lp(&e, &contract_address);

            if locked_balance > 0 {
                burn_lp_tokens(&e, &contract_address, locked_balance);

                let total_shares = get_total_lp_tokens(&e);
                let reserve_b = get_reserve_b(&e);
                let token_b_amount = if total_shares > 0 {
                    (locked_balance * reserve_b) / total_shares
                } else {
                    locked_balance
                };
                transfer_b(&e, &admin, token_b_amount);
            }
        }
    }

    fn set_max_imbalances(
        e: Env,
        admin: Address,
        liquidity_max_imbalance: u128,
        max_insurance: u128,
    ) {
        admin.require_auth();
        require_operations_admin_or_owner(&e, &admin);

        let tier = get_tier(&e);
        let max_insurance_for_tier = match tier {
            PoolTier::A => INSURANCE_A_MAX,
            PoolTier::B => INSURANCE_B_MAX,
            PoolTier::C => INSURANCE_C_MAX,
            PoolTier::Speculative => INSURANCE_SPECULATIVE_MAX,
            PoolTier::HighlySpeculative => INSURANCE_SPECULATIVE_MAX,
            PoolTier::Isolated => INSURANCE_SPECULATIVE_MAX,
        };

        validate!(
            &e,
            liquidity_max_imbalance <= max_insurance_for_tier + 1
                && max_insurance <= max_insurance_for_tier,
            PoolError::DefaultError
        );

        let insurance_claim = get_insurance_claim(&e);
        validate!(
            &e,
            insurance_claim.settled_insurance <= max_insurance,
            PoolError::DefaultError
        );

        set_max_liquidity_imbalance(&e, &liquidity_max_imbalance);
        set_insurance_claim(
            &e,
            &(InsuranceClaim {
                max_insurance,
                ..insurance_claim
            }),
        );
    }

    fn set_mint_cap_fraction(e: Env, admin: Address, mint_cap_fraction: u32) {
        admin.require_auth();
        require_operations_admin_or_owner(&e, &admin);

        set_mint_cap_fraction(&e, &mint_cap_fraction);
    }

    // Sets the protocol fraction of total fee for the pool.
    fn set_protocol_fee_fraction(e: Env, admin: Address, new_fraction: u32) {
        admin.require_auth();
        require_operations_admin_or_owner(&e, &admin);

        if (new_fraction as u128) > FEE_MULTIPLIER {
            panic_with_error!(e, PoolValidationError::FeeOutOfBounds);
        }

        set_protocol_fee_fraction(&e, &new_fraction);
        LiquidityPoolEvents::new(&e).set_protocol_fee_fraction(new_fraction);
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
        require_pause_or_emergency_pause_admin_or_owner(&e, &admin);

        set_is_killed_deposit(&e, &true);
        LiquidityPoolEvents::new(&e).kill_deposit();
    }

    fn kill_withdraw(e: Env, admin: Address) {
        admin.require_auth();
        require_pause_or_emergency_pause_admin_or_owner(&e, &admin);

        set_is_killed_withdraw(&e, &true);
        LiquidityPoolEvents::new(&e).kill_withdraw();
    }

    fn kill_swap(e: Env, admin: Address) {
        admin.require_auth();
        require_pause_or_emergency_pause_admin_or_owner(&e, &admin);

        set_is_killed_swap(&e, &true);
        LiquidityPoolEvents::new(&e).kill_swap();
    }

    fn kill_claim(e: Env, admin: Address) {
        admin.require_auth();
        require_pause_or_emergency_pause_admin_or_owner(&e, &admin);

        set_is_killed_claim(&e, &true);
        LiquidityPoolEvents::new(&e).kill_claim();
    }

    fn unkill_deposit(e: Env, admin: Address) {
        admin.require_auth();
        require_pause_admin_or_owner(&e, &admin);

        set_is_killed_deposit(&e, &false);
        LiquidityPoolEvents::new(&e).unkill_deposit();
    }

    fn unkill_withdraw(e: Env, admin: Address) {
        admin.require_auth();
        require_pause_admin_or_owner(&e, &admin);

        set_is_killed_withdraw(&e, &false);
        LiquidityPoolEvents::new(&e).unkill_withdraw();
    }

    fn unkill_swap(e: Env, admin: Address) {
        admin.require_auth();
        require_pause_admin_or_owner(&e, &admin);

        set_is_killed_swap(&e, &false);
        LiquidityPoolEvents::new(&e).unkill_swap();
    }

    fn unkill_claim(e: Env, admin: Address) {
        admin.require_auth();
        require_pause_admin_or_owner(&e, &admin);

        set_is_killed_claim(&e, &false);
        LiquidityPoolEvents::new(&e).unkill_claim();
    }

    // Get deposit killswitch status.
    fn get_is_killed_deposit(e: Env) -> bool {
        get_is_killed_deposit(&e)
    }

    // Get withdraw killswitch status.
    fn get_is_killed_withdraw(e: Env) -> bool {
        get_is_killed_withdraw(&e)
    }

    // Get swap killswitch status.
    fn get_is_killed_swap(e: Env) -> bool {
        get_is_killed_swap(&e)
    }

    // Get claim killswitch status.
    fn get_is_killed_claim(e: Env) -> bool {
        get_is_killed_claim(&e)
    }
}

// The `UpgradeableContract` trait provides the interface for upgrading the contract.
#[contractimpl]
impl UpgradeableContract for Pool {
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
    // * `new_token_wasm_hash` - The new token wasm hash to commit.
    fn commit_upgrade(
        e: Env,
        admin: Address,
        new_wasm_hash: BytesN<32>,
        token_new_wasm_hash: BytesN<32>,
    ) {
        admin.require_auth();
        AccessControl::new(&e).assert_address_has_role(&admin, &Role::Admin);
        commit_upgrade(&e, &new_wasm_hash);
        // handle token upgrade manually together with pool upgrade
        set_token_future_wasm(&e, &token_new_wasm_hash);

        UpgradeEvents::new(&e).commit_upgrade(Vec::from_array(
            &e,
            [new_wasm_hash.clone(), token_new_wasm_hash.clone()],
        ));
    }

    // Applies the committed upgrade.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin.
    fn apply_upgrade(e: Env, admin: Address) -> (BytesN<32>, BytesN<32>) {
        admin.require_auth();
        AccessControl::new(&e).assert_address_has_role(&admin, &Role::Admin);
        let new_wasm_hash = apply_upgrade(&e);
        let token_new_wasm_hash = get_token_future_wasm(&e);
        token_lp::Client::new(&e, &get_token_lp(&e))
            .upgrade(&e.current_contract_address(), &token_new_wasm_hash);

        UpgradeEvents::new(&e).apply_upgrade(Vec::from_array(
            &e,
            [new_wasm_hash.clone(), token_new_wasm_hash.clone()],
        ));

        (new_wasm_hash, token_new_wasm_hash)
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

#[contractimpl]
impl UpgradeableLPTokenTrait for Pool {
    // legacy upgrade. not compatible with token contract version 140+ due to different arguments
    fn upgrade_token_legacy(e: Env, admin: Address, new_token_wasm: BytesN<32>) {
        admin.require_auth();
        AccessControl::new(&e).assert_address_has_role(&admin, &Role::Admin);

        e.invoke_contract::<()>(
            &get_token_lp(&e),
            &symbol_short!("upgrade"),
            Vec::from_array(&e, [new_token_wasm.to_val()]),
        );
    }
}

#[contractimpl]
impl IncentivesTrait for Pool {
    // Initializes the rewards configuration.
    //
    // # Arguments
    //
    // * `e` - The environment.
    // * `reward_token` - The address of the reward token.
    fn initialize_incentives_config(e: Env, reward_token: Address) {
        let incentives = get_incentives_manager(&e);
        if incentives.storage().has_reward_token() {
            panic_with_error!(&e, PoolError::RewardsAlreadyInitialized);
        }

        incentives.storage().put_reward_token(reward_token);
    }

    // Sets the rewards configuration.
    //
    // # Arguments
    //
    // * `e` - The environment.
    // * `admin` - The address of the admin user.
    // * `expired_at` - The timestamp when the rewards expire.
    // * `tps` - The value with 7 decimal places. Example: 600_0000000
    fn set_incentives_config(
        e: Env,
        admin: Address,
        expired_at: u64, // timestamp
        tps: u128,       // value with 7 decimal places. example: 600_0000000
    ) {
        admin.require_auth();

        // rewards admin, owner and router are privileged to set the rewards config
        if admin != get_router(&e) {
            require_rewards_admin_or_owner(&e, &admin);
        }

        let incentives = get_incentives_manager(&e);
        let total_shares = get_total_lp_tokens(&e);
        incentives
            .manager()
            .set_incentive_config(total_shares, expired_at, tps);
        RewardEvents::new(&e).set_incentives_config(expired_at, tps);
    }

    // Get difference between the actual balance and the total unclaimed reward minus the reserves
    fn get_unused_reward(e: Env) -> u128 {
        let incentives = get_incentives_manager(&e);
        let mut incentives_manager = incentives.manager();
        let total_shares = get_total_lp_tokens(&e);
        let mut reward_balance_to_keep = incentives_manager
            .get_total_configured_reward(total_shares)
            - incentives_manager.get_total_claimed_reward(total_shares);

        let reward_token = incentives.storage().get_reward_token();
        let reward_balance = SorobanTokenClient::new(&e, &reward_token)
            .balance(&e.current_contract_address()) as u128;

        match Self::get_tokens(e.clone()).first_index_of(reward_token) {
            Some(idx) => {
                // since reward token is in the reserves, we need to keep also the reserves value
                reward_balance_to_keep += Self::get_reserves(e.clone()).get(idx).unwrap();
            }
            None => {}
        }

        if reward_balance > reward_balance_to_keep {
            reward_balance - reward_balance_to_keep
        } else {
            // balance is not sufficient, no surplus
            0
        }
    }

    // Return reward token above the configured amount back to router
    fn return_unused_reward(e: Env, admin: Address) -> u128 {
        admin.require_auth();
        require_rewards_admin_or_owner(&e, &admin);

        let unused_reward = Self::get_unused_reward(e.clone());

        if unused_reward == 0 {
            return 0;
        }

        let reward_token = get_incentives_manager(&e).storage().get_reward_token();
        transfer_token(
            &e,
            &reward_token,
            &e.current_contract_address(),
            &get_router(&e),
            &(unused_reward as i128),
        );
        unused_reward
    }

    // Returns the incentives information:
    //     tps, total accumulated amount for user, expiration, amount available to claim, debug info.
    //
    // # Arguments
    //
    // * `e` - The environment.
    // * `user` - The address of the user.
    //
    // # Returns
    //
    // A map of Symbols to i128 representing the incentives information.
    fn get_incentives_info(e: Env, user: Address) -> Map<Symbol, i128> {
        let incentives = get_incentives_manager(&e);
        let mut manager = incentives.manager();
        let storage = incentives.storage();
        let config = storage.get_pool_incentive_config();
        let total_shares = get_total_lp_tokens(&e);
        let user_shares = get_user_balance_lp(&e, &user);

        // pre-fill result dict with stored values
        // or values won't be affected by checkpoint in any way
        let mut result = Map::from_array(
            &e,
            [
                (symbol_short!("tps"), config.reward_tps as i128),
                (symbol_short!("exp_at"), config.reward_expired_at as i128),
                (symbol_short!("supply"), total_shares as i128),
                (
                    Symbol::new(&e, "working_balance"),
                    manager.get_working_balance(&user, user_shares) as i128,
                ),
                (
                    Symbol::new(&e, "working_supply"),
                    manager.get_working_supply(total_shares) as i128,
                ),
            ],
        );

        // display actual values
        let user_data = manager.checkpoint_user(&user, total_shares, user_shares, 0);
        let pool_data = storage.get_pool_incentive_data();

        result.set(symbol_short!("acc"), pool_data.accumulated_rewards as i128);
        result.set(
            symbol_short!("last_time"),
            pool_data.rewards_last_time as i128,
        );
        result.set(
            symbol_short!("pool_acc"),
            user_data.pool_accumulated_rewards as i128,
        );
        result.set(symbol_short!("block"), pool_data.block as i128);
        result.set(
            symbol_short!("fees_owed"),
            pool_data.fee_growth_per_lp as i128,
        );

        result.set(symbol_short!("usr_block"), user_data.last_block as i128);
        result.set(
            symbol_short!("to_claim"),
            user_data.rewards_to_claim as i128,
        );
        result.set(symbol_short!("fee_check"), user_data.fee_checkpoint as i128);

        // provide updated working balance information. if working_balance_new is bigger
        // than working_balance, it means that user has locked some tokens
        // and needs to checkpoint itself for more rewards
        result.set(
            Symbol::new(&e, "new_working_balance"),
            manager.get_working_balance(&user, user_shares) as i128,
        );
        result.set(
            Symbol::new(&e, "new_working_supply"),
            manager.get_working_supply(total_shares) as i128,
        );
        result
    }

    // Returns the amount of reward tokens available for the user to claim.
    //
    // # Arguments
    //
    // * `e` - The environment.
    // * `user` - The address of the user.
    //
    // # Returns
    //
    // The amount of reward tokens available for the user to claim as a u128.
    fn get_user_reward(e: Env, user: Address) -> u128 {
        let incentives = get_incentives_manager(&e);
        let total_shares = get_total_lp_tokens(&e);
        let user_shares = get_user_balance_lp(&e, &user);
        incentives
            .manager()
            .get_reward_amount_to_claim(&user, total_shares, user_shares)
    }

    // Get amount of LP fees available for the user to claim.
    fn get_user_fees(e: Env, user: Address) -> u128 {
        let incentives = get_incentives_manager(&e);
        let total_shares = get_total_lp_tokens(&e);
        let user_shares = get_user_balance_lp(&e, &user);
        incentives
            .manager()
            .get_fee_amounts_to_claim(&user, total_shares, user_shares)
    }

    fn checkpoint_incentive(e: Env, token_contract: Address, user: Address, user_shares: u128) {
        // checkpoint reward with provided values to avoid re-entrancy issue
        token_contract.require_auth();
        if token_contract != get_token_lp(&e) {
            panic_with_error!(&e, AccessControlError::Unauthorized);
        }
        let incentives = get_incentives_manager(&e);
        let total_lp_tokens = get_total_lp_tokens(&e);
        incentives
            .manager()
            .checkpoint_user(&user, total_lp_tokens, user_shares, 0);
    }

    fn checkpoint_working_balance(
        e: Env,
        token_contract: Address,
        user: Address,
        user_shares: u128,
    ) {
        // checkpoint working balance with provided values to avoid re-entrancy issue
        token_contract.require_auth();
        if token_contract != get_token_lp(&e) {
            panic_with_error!(&e, AccessControlError::Unauthorized);
        }
        let incentives = get_incentives_manager(&e);
        let total_lp_tokens = get_total_lp_tokens(&e);
        incentives
            .manager()
            .update_working_balance(&user, total_lp_tokens, user_shares);
    }

    // Returns the total amount of accumulated reward for the pool.
    //
    // # Arguments
    //
    // * `e` - The environment.
    //
    // # Returns
    //
    // The total amount of accumulated reward for the pool as a u128.
    fn get_total_accumulated_reward(e: Env) -> u128 {
        let incentives = get_incentives_manager(&e);
        let total_shares = get_total_lp_tokens(&e);
        incentives
            .manager()
            .get_total_accumulated_reward(total_shares)
    }

    // Returns the total amount of configured reward for the pool.
    //
    // # Arguments
    //
    // * `e` - The environment.
    //
    // # Returns
    //
    // The total amount of configured reward for the pool as a u128.
    fn get_total_configured_reward(e: Env) -> u128 {
        let incentives = get_incentives_manager(&e);
        let total_shares = get_total_lp_tokens(&e);
        incentives
            .manager()
            .get_total_configured_reward(total_shares)
    }

    // Returns the total amount of claimed reward for the pool.
    //
    // # Arguments
    //
    // * `e` - The environment.
    //
    // # Returns
    //
    // The total amount of claimed reward for the pool as a u128.
    fn get_total_claimed_reward(e: Env) -> u128 {
        let incentives = get_incentives_manager(&e);
        let total_shares = get_total_lp_tokens(&e);
        incentives.manager().get_total_claimed_reward(total_shares)
    }

    // Claims the LP fees and reward as a user.
    //
    // # Arguments
    //
    // * `e` - The environment.
    // * `user` - The address of the user.
    //
    // # Returns
    //
    // The amount of tokens rewarded to the user as a u128.
    fn claim(e: Env, user: Address) -> (u128, u128) {
        if get_is_killed_claim(&e) {
            panic_with_error!(e, PoolError::PoolClaimKilled);
        }

        let tokens = Self::get_tokens(e.clone());

        let incentives = get_incentives_manager(&e);
        let total_shares = get_total_lp_tokens(&e);
        let user_shares = get_user_balance_lp(&e, &user);
        let mut incentives_manager = incentives.manager();
        let incentives_storage = incentives.storage();
        let (reward, fees_owed) = incentives_manager.claim_incentives(
            &user,
            total_shares,
            user_shares,
            &tokens.get(1).unwrap(),
        );

        // validate reserves after claim - they should be less than or equal to the balance
        let reward_token = incentives_storage.get_reward_token();
        let reserves = Self::get_reserves(e.clone());

        for i in 0..reserves.len() {
            let token = tokens.get(i).unwrap();
            if token != reward_token {
                continue;
            }

            let balance = SorobanTokenClient::new(&e, &tokens.get(i).unwrap())
                .balance(&e.current_contract_address()) as u128;
            if reserves.get(i).unwrap() > balance {
                panic_with_error!(&e, PoolValidationError::InsufficientBalance);
            }
        }

        RewardEvents::new(&e).claim(
            user,
            reward_token,
            reward,
            tokens.get(1).unwrap(),
            fees_owed,
        );

        (reward, fees_owed)
    }
}

#[contractimpl]
impl Plane for Pool {
    // Sets the plane for the pool.
    //
    // # Arguments
    //
    // * `e` - The environment.
    // * `plane` - The address of the plane.
    //
    // # Panics
    //
    // If the plane has already been initialized.
    fn init_pools_plane(e: Env, plane: Address) {
        if has_plane(&e) {
            panic_with_error!(&e, PoolError::PlaneAlreadyInitialized);
        }

        set_plane(&e, &plane);
    }

    fn set_pools_plane(e: Env, admin: Address, plane: Address) {
        admin.require_auth();
        AccessControl::new(&e).assert_address_has_role(&admin, &Role::Admin);

        set_plane(&e, &plane);
    }

    // Returns the plane of the pool.
    //
    // # Arguments
    //
    // * `e` - The environment.
    //
    // # Returns
    //
    // The address of the plane.
    fn get_pools_plane(e: Env) -> Address {
        get_plane(&e)
    }

    // Updates the plane data in case the plane contract was updated.
    fn backfill_plane_data(e: Env) {
        update_plane(&e);
    }
}

// The `TransferableContract` trait provides the interface for transferring ownership of the contract.
#[contractimpl]
impl TransferableContract for Pool {
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
