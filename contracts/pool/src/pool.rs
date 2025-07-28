use core::cmp::max;

use crate::errors::PoolValidationError;
use crate::events::Events as LiquidityPoolEvents;
use crate::events::PoolEvents;
use crate::storage::get_last_oracle_valid;
use crate::storage::get_last_trade_ts;
use crate::storage::get_last_update_ts;
use crate::storage::get_oracle_registry;
use crate::storage::get_volume_30d;
use crate::storage::set_last_trade_ts;
use crate::storage::set_volume_30d;
use crate::storage::{ get_reserve_a, get_reserve_b, set_reserve_a };
use pool_tokens::{ burn_synthetic_tokens, get_total_synthetic_tokens, mint_synthetic_tokens };
use soroban_fixed_point_math::SorobanFixedPoint;

use soroban_sdk::IntoVal;
use soroban_sdk::Symbol;
use soroban_sdk::Vec;
use soroban_sdk::{ panic_with_error, Env };

use utils::constant::PRICE_TIMES_AMM_TO_QUOTE_PRECISION_RATIO_I128;
use utils::constant::TWENTY_FOUR_HOUR;
use utils::constant::{ FEE_MULTIPLIER, PRICE_PRECISION };
use utils::math::safe_math::SafeMath;
use utils::math::stats::calculate_rolling_sum;
use utils::state::oracle_registry::NormalAction;
use utils::state::oracle_registry::OraclePriceData;

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
    quote_oracle_price: u128
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
// passing the calling contract, asset, caching preference, and action context.
//
// # Arguments
// * `e` - Soroban environment reference.
// * `asset` - Symbol representing the asset to price.
// * `cached` - Whether to use cached data from the oracle.
// * `action` - The context in which the price is being fetched (e.g. Swap, Rebalance).
//
// # Returns
// * `OraclePriceData` — The current oracle price and delay since publication.
pub fn get_oracle_price(
    e: &Env,
    asset: &Symbol,
    cached: bool,
    action: NormalAction
) -> OraclePriceData {
    let skip = true;
    e.invoke_contract(
        &get_oracle_registry(e),
        &Symbol::new(e, "get_price"),
        Vec::from_array(e, [
            asset.to_val(),
            cached.into_val(e),
            action.into_val(e),
            skip.into_val(e),
        ])
    )
}

pub fn get_last_oracle_price(e: &Env, asset: &Symbol) -> HistoricalOracleData {
    e.invoke_contract(
        &get_oracle_registry(e),
        &Symbol::new(e, "get_last_price"),
        Vec::from_array(e, [asset.to_val()])
    )
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
    let since_last = max(1_u64, now.safe_sub(e, get_last_trade_ts(e)));
    let volume_30d = get_volume_30d(e);
    set_volume_30d(
        e,
        &calculate_rolling_sum(e, volume_30d, quote_asset_amount, since_last, TWENTY_FOUR_HOUR)
    );
    set_last_trade_ts(e, &now);
}

// Checks whether the most recent oracle update is still valid for use.
//
// Compares the current timestamp to the last update timestamp and returns `true`
// only if they match and the last oracle update was marked valid.
//
// # Arguments
// * `e` - Soroban environment reference.
// * `current_ts` - The current ledger timestamp.
//
// # Returns
// * `bool` — `true` if the oracle data is recent and marked valid.
pub fn is_recent_oracle_valid(e: &Env, current_ts: u64) -> bool {
    get_last_oracle_valid(e) && current_ts == get_last_update_ts(e)
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
pub fn get_delta_a(e: &Env, base_oracle_price: u128, quote_oracle_price: u128) -> i128 {
    let (reserve_a, reserve_b) = (get_reserve_a(e), get_reserve_b(e));

    let peg_price = peg_price(e, base_oracle_price, quote_oracle_price);
    let target_reserve_a = reserve_b.fixed_div_floor(e, &peg_price, &PRICE_PRECISION);
    let delta_a = (target_reserve_a as i128).checked_sub(reserve_a as i128).unwrap();

    delta_a
}

// Mints or burns synthetic tokens (reserve A) to restore the peg between base and quote assets.
//
// Uses oracle prices to calculate the required change in synthetic token supply to match
// the peg. Adjusts the pool's reserve A accordingly and emits a rebalance event.
//
// # Arguments
// * `e` - Soroban environment reference.
// * `base_oracle_price` - Oracle price of the synthetic base asset.
// * `quote_oracle_price` - Oracle price of the quote asset.
// * `now` - Current ledger timestamp used in the emitted event.
pub fn rebalance(e: &Env, base_oracle_price: u128, quote_oracle_price: u128) {
    let (reserve_a, reserve_b) = (get_reserve_a(&e), get_reserve_b(&e));

    // Find the ideal reserve_a amount such that the pool's price is equal to the oracle price
    let delta_a = get_delta_a(&e, base_oracle_price, quote_oracle_price);

    if delta_a > 0 {
        mint_synthetic_tokens(&e, &e.current_contract_address(), delta_a);
        set_reserve_a(&e, &(reserve_a + (delta_a as u128)));
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
        delta_a
    );
}

// Calculates the input amount required to receive a fixed output amount in a swap,
// factoring in the trading fee.
//
// This is used in strict-receive swaps where the output is guaranteed and the input
// is determined. If the total value with fee exceeds the reserve, the function panics.
//
// # Arguments
// * `e` - Soroban environment reference.
// * `out_amount` - Desired amount of output token to receive.
// * `reserve_sell` - Reserve of the token being sold.
// * `reserve_buy` - Reserve of the token being bought.
// * `fee_fraction` - Trading fee as a fraction (e.g., 30 = 0.3%).
//
// # Returns
// * `(u128, u128)` — Tuple:
//   - Input amount required to receive `out_amount`
//   - Fee charged as the difference between input and output value.
//
// # Panics
// - If the fee-adjusted input exceeds available reserves.
pub fn get_amount_out_strict_receive(
    e: &Env,
    out_amount: u128,
    reserve_sell: u128,
    reserve_buy: u128,
    fee_fraction: u32
) -> (u128, u128) {
    if out_amount == 0 {
        return (0, 0);
    }

    let dy_w_fee = out_amount.fixed_mul_ceil(
        &e,
        &FEE_MULTIPLIER,
        &(FEE_MULTIPLIER - (fee_fraction as u128))
    );
    // if total value including fee is more than the reserve, math can't be done properly
    if dy_w_fee >= reserve_buy {
        panic_with_error!(e, PoolValidationError::InsufficientBalance);
    }
    // +1 just in case there were some rounding errors & convert to real units in place
    let result =
        reserve_buy.fixed_mul_floor(&e, &reserve_sell, &(reserve_buy - dy_w_fee)) -
        reserve_sell +
        1;
    (result, dy_w_fee - out_amount)
}
