use core::cmp::max;

use crate::errors::PoolError;
use crate::errors::PoolValidationError;
use crate::events::Events as LiquidityPoolEvents;
use crate::events::PoolEvents;
use crate::storage::get_base_asset;
use crate::storage::get_fee_fraction;
use crate::storage::get_insurance_claim;
use crate::storage::get_insurance_fund;
use crate::storage::get_last_trade_ts;
use crate::storage::get_mint_cap_fraction;
use crate::storage::get_oracle_registry;
use crate::storage::get_rebalance_minted;
use crate::storage::get_status;
use crate::storage::get_token_insurance;
use crate::storage::get_total_synthetics;
use crate::storage::get_volume_30d;
use crate::storage::set_insurance_claim;
use crate::storage::set_last_trade_ts;
use crate::storage::set_rebalance_minted;
use crate::storage::set_reserve_b;
use crate::storage::set_volume_30d;
use crate::storage::{get_reserve_a, get_reserve_b, set_reserve_a};
use crate::token::burn_synthetic_tokens;
use crate::token::mint_synthetic_tokens;
use soroban_fixed_point_math::SorobanFixedPoint;

use soroban_sdk::log;
use soroban_sdk::IntoVal;
use soroban_sdk::Symbol;
use soroban_sdk::Vec;
use soroban_sdk::{panic_with_error, Env};

use utils::constant::FEE_MULTIPLIER;
use utils::constant::PRICE_PRECISION;
use utils::constant::PRICE_PRECISION_I128;
use utils::constant::PRICE_TIMES_AMM_TO_QUOTE_PRECISION_RATIO_I128;
use utils::constant::THIRTY_DAY;
use utils::constant::TWENTY_FOUR_HOUR;
use utils::math::safe_math::SafeMath;
use utils::math::stats::calculate_rolling_sum;
use utils::state::oracle_registry::HistoricalOracleData;
use utils::state::oracle_registry::NormalAction;
use utils::state::oracle_registry::OracleGuardRails;
use utils::state::oracle_registry::OracleValidity;
use utils::state::pool::PoolStatus;
use utils::state::pool::PoolTier;
use utils::validate;

pub fn settle_swap_using_insurance(e: &Env, amount: u128, current_time: u64) -> u128 {
    if amount == 0 {
        return 0;
    }

    let asset = get_base_asset(e);
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

    validate!(
        &e,
        amount <= max_insurance_withdraw,
        PoolError::NoIFWithdrawAvailable
    );

    match e.try_invoke_contract::<u128, soroban_sdk::Error>(
        &get_insurance_fund(e),
        &Symbol::new(e, "file_claim"),
        Vec::from_array(
            e,
            [
                e.current_contract_address().into_val(e),
                get_token_insurance(e).into_val(e),
                asset.into_val(e),
                amount.into_val(e),
            ],
        ),
    ) {
        Ok(Err(_)) | Err(_) => panic_with_error!(&e, PoolError::FileInsuranceClaimError),
        Ok(Ok(insurance_paid)) => {
            // Update the Pool reserve
            let reserve_b = get_reserve_b(e);
            set_reserve_b(e, &reserve_b.safe_add(e, insurance_paid));

            // Update the Insurance Claim
            let mut updated_insurance_claim = insurance_claim.clone();
            updated_insurance_claim.rev_withdraw_since_last_settle = updated_insurance_claim
                .rev_withdraw_since_last_settle
                .safe_add(&e, insurance_paid);

            updated_insurance_claim.settled_insurance = updated_insurance_claim
                .settled_insurance
                .safe_add(&e, insurance_paid);

            validate!(
                &e,
                updated_insurance_claim.settled_insurance <= updated_insurance_claim.max_insurance,
                PoolError::MaxIFWithdrawReached
            );

            updated_insurance_claim.last_revenue_withdraw_ts = current_time;
            set_insurance_claim(&e, &updated_insurance_claim);

            return insurance_paid;
        }
    }
}

// Calculates the liquidity imbalance between synthetic token and quote asset reserve in the pool.
//
// Computes the value of synthetic base tokens using the base oracle price, and compares it to
// the value of reserve Token B using the quote oracle price. A positive result means excess
// quote-side liquidity; negative means excess synthetic base.
//
// # Arguments
// * `e` - Soroban environment reference.
// * `base_oracle_price` - Oracle price of the base (synthetic) asset.
// * `quote_oracle_price` - Oracle price of the quote asset (Token B).
//
// # Returns
// * `i128` — The imbalance: `quote_value - base_value`. Positive means excess quote asset.
pub fn get_liquidity_imbalance(e: &Env, base_oracle_price: u128, quote_oracle_price: u128) -> i128 {
    let total_synthetic_token_supply = get_total_synthetics(&e);
    let reserve_b = get_reserve_b(e);

    let total_synthetic_value = (total_synthetic_token_supply as i128)
        .safe_mul(e, base_oracle_price as i128)
        .safe_div(e, PRICE_TIMES_AMM_TO_QUOTE_PRECISION_RATIO_I128);

    let total_reserve_b_value = (reserve_b as i128)
        .safe_mul(e, quote_oracle_price as i128)
        .safe_div(e, PRICE_TIMES_AMM_TO_QUOTE_PRECISION_RATIO_I128);

    total_reserve_b_value.safe_sub(e, total_synthetic_value)
}

// Invokes the external Oracle Registry contract to fetch the current price for a given asset.
//
// This performs a cross-contract call to the `get_price` method on the Oracle Registry,
// passing the asset and action context.
//
// # Arguments
// * `e` - Soroban environment reference.
// * `asset` - Symbol representing the asset to price.
//
// # Returns
// * `HistoricalOracleData` — The current oracle price and delay since publication.
pub fn get_oracle_price(e: &Env, asset: &Symbol) -> HistoricalOracleData {
    let (historical_oracle_data, oracle_validity): (HistoricalOracleData, OracleValidity) = e
        .invoke_contract(
            &get_oracle_registry(e),
            &Symbol::new(e, "get_price"),
            Vec::from_array(e, [asset.to_val()]),
        );

    if oracle_validity != OracleValidity::Valid {
        panic_with_error!(e, PoolError::InvalidOracle);
    }

    historical_oracle_data
}

pub fn validate_oracle_price_with_pool(
    e: &Env,
    base_oracle_price_twap: u128,
    quote_oracle_price_twap: u128,
    action: NormalAction,
) {
    if action == NormalAction::PoolInit {
        return;
    }

    let (reserve_a, reserve_b) = (get_reserve_a(&e), get_reserve_b(&e));

    let peg_price = peg_price(e, base_oracle_price_twap, quote_oracle_price_twap);

    let pool_price = if reserve_a == 0 || reserve_b == 0 {
        peg_price
    } else {
        reserve_b.fixed_div_floor(e, &reserve_a, &PRICE_PRECISION)
    };

    // Find % difference b/t pool price and oracle price
    let oracle_pool_price_spread_pct =
        calculate_oracle_twap_price_spread_pct(e, pool_price, peg_price);

    // Check if the oracle price is too divergent
    let oracle_guard_rails: OracleGuardRails = e.invoke_contract(
        &get_oracle_registry(e),
        &Symbol::new(e, "get_oracle_guard_rails"),
        Vec::from_array(e, []),
    );

    let is_oracle_price_too_divergent =
        is_oracle_price_too_divergent(oracle_pool_price_spread_pct, oracle_guard_rails);

    if is_oracle_price_too_divergent {
        panic_with_error!(e, PoolError::InvalidOracle);
    }
}

// Calculates the percentage difference between the pool price and oracle TWAP.
//
// Used to evaluate whether the oracle data is in line with internal pricing,
// expressed as a percentage spread for risk assessment.
//
// # Arguments
// * `e` - Soroban environment reference.
// * `pool_price` - The current pool price.
// * `last_oracle_price_twap` - Oracle's last TWAP.
//
// # Returns
// - The price spread as a percentage (scaled by `PRICE_PRECISION_U64`).
pub fn calculate_oracle_twap_price_spread_pct(
    e: &Env,
    pool_price: u128,
    last_oracle_price_twap: u128,
) -> i64 {
    let price_spread = (pool_price as i128).saturating_sub(last_oracle_price_twap as i128);

    price_spread.fixed_div_floor(e, &(pool_price as i128), &PRICE_PRECISION_I128) as i64
}

// Determines whether the oracle price diverges too far from the reserve price.
//
// Uses protocol-defined guard rails to decide if the deviation is outside
// acceptable limits (e.g., >10%) and may indicate manipulation or lag.
//
//
// # Arguments
// * `price_spread_pct` - Absolute spread percentage between oracle and reserve.
// * `oracle_guard_rails` - .
//
// # Returns
// - `true` if the spread exceeds the maximum allowed divergence.
pub fn is_oracle_price_too_divergent(
    price_spread_pct: i64,
    oracle_guard_rails: OracleGuardRails,
) -> bool {
    let max_divergence = oracle_guard_rails
        .price_divergence
        .oracle_twap_percent_divergence;

    price_spread_pct.unsigned_abs() > max_divergence
}

// Calculates the peg price between the base and quote assets based on oracle prices.
//
// Returns `quote / base` to represent the current price ratio. If either price is zero,
// returns 0 to indicate an invalid state.
//
// # Arguments
// * `e` - Soroban environment reference.
// * `base_oracle_price` - Oracle price of the base asset.
// * `quote_oracle_price` - Oracle price of the quote asset.
//
// # Returns
// * `u128` — The derived peg price (scaled by `PRICE_PRECISION`), or 0 if invalid.
pub fn peg_price(e: &Env, base_oracle_price: u128, quote_oracle_price: u128) -> u128 {
    if base_oracle_price == 0 || quote_oracle_price == 0 {
        return 0;
    }

    base_oracle_price.fixed_div_floor(e, &quote_oracle_price, &PRICE_PRECISION)
}

// Updates the 30 day trading volume metric for the pool using a rolling average.
//
// Uses the time since the last trade and the current quote asset volume to update
// the 30 day volume accumulator. Also updates the last trade timestamp.
//
// # Arguments
// * `e` - Soroban environment reference.
// * `quote_asset_amount` - Amount of quote asset involved in the trade.
// * `now` - Current ledger timestamp.
pub fn update_volume_30d(e: &Env, quote_asset_amount: u128, now: u64) {
    let since_last = max(1_u64, now.saturating_sub(get_last_trade_ts(e)));
    let volume_30d = get_volume_30d(e);

    if volume_30d == 0 {
        set_volume_30d(e, &quote_asset_amount);
    } else {
        let sum = calculate_rolling_sum(
            e,
            volume_30d,
            quote_asset_amount,
            since_last,
            TWENTY_FOUR_HOUR,
        );

        set_volume_30d(e, &sum);
    }

    set_last_trade_ts(e, &now);
}

// Computes the delta needed to re-peg reserve A (synthetic base token) to match the target peg price.
//
// Uses current reserves and oracle prices to calculate the ideal reserve A value,
// then subtracts the actual reserve A to determine how much must be minted or burned.
//
// # Arguments
// * `e` - Soroban environment reference.
// * `base_oracle_price` - Oracle price of the base asset.
// * `quote_oracle_price` - Oracle price of the quote asset.
//
// # Returns
// * `i128` — The difference: `target_reserve_a - actual_reserve_a`.
// Positive means mint, negative means burn.
pub fn get_delta_a(
    e: &Env,
    reserve_a: u128,
    reserve_b: u128,
    base_oracle_price: u128,
    quote_oracle_price: u128,
) -> i128 {
    let peg_price = peg_price(e, base_oracle_price, quote_oracle_price);
    let target_reserve_a = reserve_b.fixed_div_floor(e, &peg_price, &PRICE_PRECISION);
    let delta_a = (target_reserve_a as i128)
        .checked_sub(reserve_a as i128)
        .unwrap();

    delta_a
}

// Mints or burns synthetic tokens (reserve A) to restore the peg between base and quote assets.
//
// Uses oracle prices to calculate the required change in synthetic token supply to match
// the peg. Adjusts the pool's reserve A accordingly and emits a rebalance event.
// In ReduceOnly mode, prevents minting new synthetic tokens to avoid increasing synthetic asset exposure.
//
// # Arguments
// * `e` - Soroban environment reference.
// * `base_oracle_price` - Oracle price of the synthetic base asset.
// * `quote_oracle_price` - Oracle price of the quote asset.
pub fn rebalance(e: &Env, base_oracle_price: u128, quote_oracle_price: u128) -> i128 {
    let status = get_status(e);
    let reduce_only = status == PoolStatus::ReduceOnly;

    let (reserve_a, reserve_b) = (get_reserve_a(e), get_reserve_b(e));

    // Find the ideal reserve_a amount such that the pool's price is equal to the oracle price
    let delta_a = get_delta_a(
        e,
        reserve_a,
        reserve_b,
        base_oracle_price,
        quote_oracle_price,
    );
    if delta_a != 0 {
        if delta_a > 0 {
            if reduce_only {
                LiquidityPoolEvents::new(e).capped_mint(
                    base_oracle_price,
                    quote_oracle_price,
                    delta_a,
                );

                // allow minting up to 0.1% of current supply per ledger
                let mint_cap = get_total_synthetics(e).fixed_mul_ceil(
                    e,
                    &(get_mint_cap_fraction(e) as u128),
                    &FEE_MULTIPLIER,
                ) as i128;
                // (get_total_synthetics(e) / (get_mint_cap_fraction(e) as u128)) as i128;

                if delta_a > mint_cap {
                    panic_with_error!(e, PoolError::SwapReduceOnly);
                }
            }

            mint_synthetic_tokens(e, &e.current_contract_address(), delta_a);
            set_reserve_a(e, &(reserve_a + (delta_a as u128)));
        }
        if delta_a < 0 {
            burn_synthetic_tokens(e, &e.current_contract_address(), delta_a.abs() as u128);
            set_reserve_a(e, &(reserve_a - (delta_a.abs() as u128)));
        }

        let (new_reserve_a, new_reserve_b) = (get_reserve_a(e), get_reserve_b(e));

        LiquidityPoolEvents::new(e).rebalance(
            reserve_a,
            reserve_b,
            new_reserve_a,
            new_reserve_b,
            delta_a,
        );
    }

    // Update RebalanceMinted to track the outstanding number of synthetic tokens minted/burned by the pool
    let rebalance_minted = get_rebalance_minted(e) as i128;
    log!(&e, "rebalance_minted", rebalance_minted);
    set_rebalance_minted(&e, &(rebalance_minted.safe_add(e, delta_a) as u128));

    delta_a
}

/// Calculates the output amount of tokens for a swap given input and pool reserves.
///
/// This function implements a constant product AMM–style formula with fees applied externally (using PoolSwapFee).
/// The calculation follows:
///
/// ```text
/// out = in * reserve_buy / (reserve_sell + in) - fee
/// ```
///
/// The `fee` portion is not handled inside this function—it must be deducted by the caller
/// after receiving the raw output value.
///
/// # Arguments
///
/// * `in_amount` – The amount of input tokens being sold into the pool.
/// * `reserve_sell` – The current reserve of the token being sold (the pool’s supply of the input token).
/// * `reserve_buy` – The current reserve of the token being bought (the pool’s supply of the output token).
///
/// # Returns
///
/// A u128 `amount_out`:
/// * `amount_out` – The number of output tokens the user would receive before fee deduction.
///
/// Returns `0` if `in_amount == 0`.
///
/// # Examples
///
/// ```rust
/// // Example reserves
/// let reserve_sell: u128 = 1_000_000;
/// let reserve_buy: u128 = 500_000;
/// let in_amount: u128 = 10_000;
///
/// // Calculate output before fee
/// let out = contract.get_amount_out(in_amount, reserve_sell, reserve_buy);
///
/// assert!(out > 0);
/// ```
pub fn get_amount_out(
    e: &Env,
    in_amount: u128,    // dx  – exact tokens the trader wants to sell
    reserve_sell: u128, // x
    reserve_buy: u128,  // y
) -> (u128, u128) {
    if in_amount == 0 {
        return (0, 0);
    }

    let fee_fraction = get_fee_fraction(e) as u128; // e.g. 30 => 0.3 %
    let in_after_fee = (in_amount * (FEE_MULTIPLIER - fee_fraction)) / FEE_MULTIPLIER;
    let raw_out = in_after_fee.fixed_mul_floor(e, &reserve_buy, &(reserve_sell + in_after_fee));
    (raw_out, in_amount - in_after_fee) // fee is taken on input
}

/// Calculates the required input amount for a **strict receive** swap given
/// a desired output amount and current pool reserves.
///
/// This function computes how much of the *sell token* must be provided in
/// order to guarantee receiving exactly `out_amount` of the *buy token*,
/// assuming a constant product AMM invariant.
///
/// The calculation uses a floor division (`fixed_mul_floor`) and then adds
/// `+1` to the result to guard against rounding errors, ensuring the pool
/// always receives enough input tokens to cover the desired output.
///
/// # Arguments
///
/// * `out_amount` – The amount of output tokens the trader wishes to receive.
/// * `reserve_sell` – The current reserve of the token being sold
///   (the pool’s input-side liquidity).
/// * `reserve_buy` – The current reserve of the token being bought
///   (the pool’s output-side liquidity).
///
/// # Returns
///
/// * `u128` – The minimum required input amount of the sell token
///   needed to guarantee the `out_amount` is fulfilled.
///   Returns `0` if `out_amount == 0`.
///
/// # Notes
///
/// * The extra `+1` ensures rounding errors never result in underpayment.
/// * The function does **not** apply protocol or trading fees — callers
///   must account for those separately if required.
///
/// # Examples
///
/// ```rust
/// let reserve_sell: u128 = 1_000_000;
/// let reserve_buy: u128 = 500_000;
/// let desired_out: u128 = 10_000;
///
/// // Compute required input for a strict receive
/// let required_in = contract::get_amount_out_strict_receive(desired_out, reserve_sell, reserve_buy);
///
/// assert!(required_in > 0);
/// ```
pub fn get_amount_out_strict_receive(
    e: &Env,
    out_amount: u128,   // dy  – exact tokens the trader wants to receive
    reserve_sell: u128, // x
    reserve_buy: u128,  // y
) -> (u128, u128) {
    if out_amount == 0 {
        return (0, 0);
    }
    if out_amount >= reserve_buy {
        panic_with_error!(e, PoolValidationError::InsufficientBalance);
    }

    let fee_fraction = get_fee_fraction(&e) as u128;

    // ----------  Step 1: dx_after_fee = ceil(x·dy / (y-dy))  ----------
    let dx_after_fee = reserve_sell.fixed_mul_ceil(e, &out_amount, &(reserve_buy - out_amount));

    // ----------  Step 2: gross-up for fee on *input* side  -------------
    // dx_before_fee = ceil( dx_after_fee / (1-f) )
    let dx_before_fee =
        dx_after_fee.fixed_mul_ceil(e, &FEE_MULTIPLIER, &(FEE_MULTIPLIER - fee_fraction));

    // ----------  Step 3: fee = dx_before_fee - dx_after_fee -----------
    let fee = dx_before_fee - dx_after_fee;

    (dx_before_fee, fee)
}

// Validates that the share calculation prevents value extraction after synthetic minting
//
// This function ensures that new depositors cannot exploit synthetic Token A minting
// by receiving shares disproportionate to their actual contribution to the pool's total value.
//
// # Arguments
// * `e` - Soroban environment reference.
// * `token_b_amount` - Amount of Token B being deposited.
// * `shares_to_mint` - Number of shares calculated to be minted.
// * `total_shares` - Total shares before this deposit.
// * `reserve_a` - Current Token A reserves (including synthetic tokens).
// * `reserve_b_before_deposit` - Token B reserves before this deposit.
// * `base_oracle_price` - Oracle price for base asset.
// * `quote_oracle_price` - Oracle price for quote asset.
pub fn validate_fair_share_calculation(
    e: &Env,
    token_b_amount: u128,
    shares_to_mint: u128,
    total_shares: u128,
    reserve_a: u128,
    reserve_b_before_deposit: u128,
    base_oracle_price: u128,
    quote_oracle_price: u128,
) {
    if total_shares > 0 && reserve_a > 0 {
        // Calculate the minimum fair shares based on pool's total value
        let token_a_value_in_token_b =
            reserve_a.fixed_mul_floor(e, &base_oracle_price, &quote_oracle_price);

        let total_pool_value = reserve_b_before_deposit + token_a_value_in_token_b;

        // Minimum shares should be proportional to contribution vs total pool value
        let expected_min_shares =
            token_b_amount.fixed_mul_floor(e, &total_shares, &total_pool_value);

        // Allow for small rounding differences
        let tolerance = expected_min_shares / 10000;
        let min_acceptable_shares = expected_min_shares.saturating_sub(tolerance);

        if shares_to_mint < min_acceptable_shares {
            panic_with_error!(e, PoolError::UnfairShareCalculation);
        }
    }
}

pub fn update_volume(e: &Env, amount: u128, current_time: u64) {
    let volume_30d = get_volume_30d(e);
    let since_last = max(1_u64, current_time.saturating_sub(get_last_trade_ts(&e)));

    let updated_volume_30d = calculate_rolling_sum(&e, volume_30d, amount, since_last, THIRTY_DAY);

    set_volume_30d(&e, &updated_volume_30d);
    set_last_trade_ts(&e, &current_time);
}

pub fn get_sanitize_clamp_denominator(tier: &PoolTier) -> Option<i64> {
    match tier {
        PoolTier::A => Some(10_i64),         // 10%
        PoolTier::B => Some(5_i64),          // 20%
        PoolTier::C => Some(2_i64),          // 50%
        PoolTier::Speculative => None,       // DEFAULT_MAX_TWAP_UPDATE_PRICE_BAND_DENOMINATOR
        PoolTier::HighlySpeculative => None, // DEFAULT_MAX_TWAP_UPDATE_PRICE_BAND_DENOMINATOR
        PoolTier::Isolated => None,          // DEFAULT_MAX_TWAP_UPDATE_PRICE_BAND_DENOMINATOR
    }
}

pub fn get_insurance_coverage_multiplier(tier: &PoolTier) -> u64 {
    match tier {
        PoolTier::A => 10_u64, // 10%
        PoolTier::B => 5_u64,  // 20%
        PoolTier::C => 2_u64,  // 50%
        PoolTier::Speculative => 10_u64,
        PoolTier::HighlySpeculative => 10_u64,
        PoolTier::Isolated => 10_u64,
    }
}
