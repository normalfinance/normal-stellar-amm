use core::cmp::max;

use crate::errors::PoolError;
use crate::events::Events as LiquidityPoolEvents;
use crate::events::PoolEvents;
use crate::storage::get_last_trade_ts;
use crate::storage::get_mint_cap_fraction;
use crate::storage::get_oracle_registry;
use crate::storage::get_volume_30d;
use crate::storage::set_last_trade_ts;
use crate::storage::set_volume_30d;
use crate::storage::{get_reserve_a, get_reserve_b, set_reserve_a};
use soroban_fixed_point_math::FixedPoint;
use token_synthetic::{burn_synthetic_tokens, get_total_synthetic_tokens, mint_synthetic_tokens};

use soroban_sdk::Symbol;
use soroban_sdk::Vec;
use soroban_sdk::{panic_with_error, Env};

use utils::constant::PRICE_PRECISION;
use utils::constant::PRICE_PRECISION_I64;
use utils::constant::PRICE_TIMES_AMM_TO_QUOTE_PRECISION_RATIO_I128;
use utils::constant::TWENTY_FOUR_HOUR;
use utils::math::safe_math::SafeMath;
use utils::math::stats::calculate_rolling_sum;
use normal_rust_types::HistoricalOracleData;
use normal_rust_types::NormalAction;
use normal_rust_types::OracleGuardRails;
use normal_rust_types::OracleValidity;

// Calculates the net liquidity imbalance between base and quote assets in the pool.
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
pub fn get_net_liquidity_imbalance(
    e: &Env,
    base_oracle_price: u128,
    quote_oracle_price: u128,
) -> i128 {
    let base_token_supply = get_total_synthetic_tokens(&e);
    let reserve_b = get_reserve_b(e);

    let net_base_asset_value = (base_token_supply as i128)
        .safe_mul(e, base_oracle_price as i128)
        .safe_div(e, PRICE_TIMES_AMM_TO_QUOTE_PRECISION_RATIO_I128);

    let net_quote_asset_value = (reserve_b as i128)
        .safe_mul(e, quote_oracle_price as i128)
        .safe_div(e, PRICE_TIMES_AMM_TO_QUOTE_PRECISION_RATIO_I128);

    net_quote_asset_value.safe_sub(e, net_base_asset_value)
}

// Invokes the external Oracle Registry contract to fetch the current price for a given asset.
//
// This performs a cross-contract call to the `get_price` method on the Oracle Registry,
// passing the asset and action context.
//
// # Arguments
// * `e` - Soroban environment reference.
// * `asset` - Symbol representing the asset to price.
// * `action` - The context in which the price is being fetched (e.g. Swap, Rebalance).
//
// # Returns
// * `HistoricalOracleData` — The current oracle price and delay since publication.
pub fn get_oracle_price(e: &Env, asset: &Symbol, action: NormalAction) -> HistoricalOracleData {
    let (historical_oracle_data, oracle_validity): (HistoricalOracleData, OracleValidity) = e
        .invoke_contract(
            &get_oracle_registry(e),
            &Symbol::new(e, "get_price"),
            Vec::from_array(e, [asset.to_val()]),
        );

    let oracle_valid_for_action = is_oracle_valid_for_action(oracle_validity, Some(action));

    if !oracle_valid_for_action {
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
        reserve_b
            .fixed_div_floor(reserve_a, PRICE_PRECISION)
            .unwrap()
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
    let price_spread: i64 =
        (pool_price as i128).saturating_sub(last_oracle_price_twap as i128) as i64;

    // price_spread_pct
    price_spread
        .fixed_div_floor(pool_price as i64, PRICE_PRECISION_I64)
        .unwrap() as i64
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

/// Determines whether the oracle data is valid for a specific contract action.
///
/// The oracle validity is evaluated based on the context of the action:
///
/// - `AddLiquidity` / `RemoveLiquidity`: Allowed if oracle data is valid or mildly stale.
/// - `Swap`: Requires strictly valid oracle data.
/// - `UpdateTwap`: Allowed if oracle price is positive.
/// - `Rebalance`: Requires strictly valid oracle data.
/// - `ClaimInsurance`: Disallowed if oracle is too volatile or non-positive.
///
/// If no action is provided, it defaults to requiring strictly valid oracle data.
///
/// # Arguments
/// * `oracle_validity` - Classification of the current oracle state.
/// * `action` - The protocol action being evaluated (optional).
///
/// # Returns
/// - `true` if the oracle data meets the criteria for the specified action.
/// - `false` if the action should be blocked due to stale, volatile, or invalid data.
pub fn is_oracle_valid_for_action(
    oracle_validity: OracleValidity,
    action: Option<NormalAction>,
) -> bool {
    let is_ok = match action {
        Some(action) => match action {
            NormalAction::PoolInit => matches!(oracle_validity, OracleValidity::Valid),
            NormalAction::AddLiquidity => matches!(
                oracle_validity,
                OracleValidity::Valid | OracleValidity::StaleForPool
            ),
            NormalAction::RemoveLiquidity => matches!(
                oracle_validity,
                OracleValidity::Valid | OracleValidity::StaleForPool
            ),
            NormalAction::Swap => matches!(oracle_validity, OracleValidity::Valid),
            NormalAction::UpdateTwap => !matches!(oracle_validity, OracleValidity::NonPositive),
            NormalAction::Rebalance => {
                matches!(oracle_validity, OracleValidity::Valid)
            }
            NormalAction::ClaimInsurance => !matches!(
                oracle_validity,
                OracleValidity::NonPositive | OracleValidity::TooVolatile
            ),
        },
        None => {
            matches!(oracle_validity, OracleValidity::Valid)
        }
    };

    is_ok
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

    base_oracle_price
        .fixed_div_floor(quote_oracle_price, PRICE_PRECISION)
        .unwrap()
    // quote_oracle_price.checked_div(base_oracle_price).unwrap_or(0)
    // quote_oracle_price.safe_div(e, base_oracle_price)
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
    let target_reserve_a = reserve_b
        .fixed_div_floor(peg_price, PRICE_PRECISION)
        .unwrap();
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
// * `pool_status` - Current pool status to determine if minting should be restricted.
pub fn rebalance(e: &Env, base_oracle_price: u128, quote_oracle_price: u128, reduce_only: bool) {
    let (reserve_a, reserve_b) = (get_reserve_a(&e), get_reserve_b(&e));

    // Find the ideal reserve_a amount such that the pool's price is equal to the oracle price
    let delta_a = get_delta_a(
        &e,
        reserve_a,
        reserve_b,
        base_oracle_price,
        quote_oracle_price,
    );
    if delta_a != 0 {
        if delta_a > 0 {
            if reduce_only {
                LiquidityPoolEvents::new(&e).capped_mint(
                    base_oracle_price,
                    quote_oracle_price,
                    delta_a,
                );

                // allow minting up to 0.1 % of current supply per ledger
                let mint_cap =
                    (get_total_synthetic_tokens(&e) / (get_mint_cap_fraction(&e) as u128)) as i128;

                if delta_a > mint_cap {
                    panic_with_error!(&e, PoolError::SwapReduceOnly);
                }
            } else {
                mint_synthetic_tokens(&e, &e.current_contract_address(), delta_a);
                set_reserve_a(&e, &(reserve_a + (delta_a as u128)));
            }
        }
        if delta_a < 0 {
            burn_synthetic_tokens(&e, &e.current_contract_address(), delta_a.abs() as u128);
            set_reserve_a(&e, &(reserve_a - (delta_a.abs() as u128)));
        }

        let (new_reserve_a, new_reserve_b) = (get_reserve_a(&e), get_reserve_b(&e));

        LiquidityPoolEvents::new(&e).rebalance(
            reserve_a,
            reserve_b,
            new_reserve_a,
            new_reserve_b,
            delta_a,
        );
    }
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
pub fn get_amount_out(in_amount: u128, reserve_sell: u128, reserve_buy: u128) -> u128 {
    if in_amount == 0 {
        return 0;
    }

    // +1 just in case there were some rounding errors & convert to real units in place
    let result = in_amount
        .fixed_mul_floor(reserve_buy, reserve_sell.saturating_add(in_amount))
        .unwrap()
        + 1;

    result
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
    out_amount: u128,
    reserve_sell: u128,
    reserve_buy: u128,
) -> u128 {
    if out_amount == 0 {
        return 0;
    }

    // +1 just in case there were some rounding errors & convert to real units in place
    let result = reserve_buy
        .fixed_mul_floor(reserve_sell, reserve_buy)
        .unwrap()
        - reserve_sell
        + 1;

    result
}
