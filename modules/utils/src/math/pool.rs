use soroban_fixed_point_math::FixedPoint;
use soroban_sdk::Env;

use crate::constant::{DEFAULT_MAX_TWAP_UPDATE_PRICE_BAND_DENOMINATOR, FEE_MULTIPLIER};

use super::safe_math::SafeMath;

/// Sanitizes a new oracle price update by clamping it within a band around the TWAP.
///
/// This function guards against abrupt oracle spikes by limiting `new_price` to
/// a maximum delta from `last_price_twap`. If `sanitize_clamp_denominator` is
/// non-zero, the allowed price band is:
///
/// ```text
/// band = last_price_twap / sanitize_clamp_denominator
/// ```
///
/// The sanitized price is then:
///
/// ```text
/// if abs(new_price - last_price_twap) > band:
///     // clamp toward the TWAP edge
///     capped = last_price_twap ± band
/// else:
///     capped = new_price
/// ```
///
/// If `sanitize_clamp_denominator == 0`, no clamping is applied and `new_price` is returned.
/// If `last_price_twap == 0`, normalization isn’t attempted and `new_price` is returned as-is.
///
/// # Arguments
///
/// * `e` — Soroban [`Env`] for safe math helpers (e.g., `safe_add`, `safe_sub`, `safe_div`).
/// * `new_price` — The latest oracle price reading (u128, must be > 0).
/// * `last_price_twap` — The previous time-weighted average price used as an anchor (u128).
/// * `sanitize_clamp_denominator` — Denominator controlling the allowed deviation.
///   - If 0, disables clamping.
///   - If non-zero, the max deviation is `last_price_twap / sanitize_clamp_denominator`.
///   - If omitted/0, a default (`DEFAULT_MAX_TWAP_UPDATE_PRICE_BAND_DENOMINATOR`) is used.
///
/// # Returns
///
/// * `u128` — The sanitized price, clamped to the TWAP band if applicable.
///
/// # Panics
///
/// * Panics if `new_price == 0`.
/// * Asserts that `last_price_twap` is non-negative (redundant for `u128`, but retained for clarity).
///
/// # Notes
///
/// * Uses `safe_*` helpers to avoid overflow/underflow on intermediate arithmetic.
/// * When clamping downward, if `band >= last_price_twap`, the function returns `0`
///   to avoid underflow and represent a floor at zero.
///
/// # Examples
///
/// ```rust
/// // Allow at most ±1% move from TWAP (denominator 100)
/// let twap = 100_000u128;
/// let new = 105_000u128; // +5%
/// let clamped = sanitize_new_price(&env, new, twap, 100);
/// assert_eq!(clamped, 101_000); // TWAP + 1% band
/// ```
pub fn sanitize_new_price(
    e: &Env,
    new_price: u128,
    last_price_twap: u128,
    sanitize_clamp_denominator: u64,
) -> u128 {
    assert!(new_price > 0, "new_price must be positive");
    assert!(last_price_twap >= 0, "last_price_twap must be non-negative");

    // when/if twap is 0, dont try to normalize new_price
    if last_price_twap == 0 {
        return new_price;
    }

    let (new_price_spread, price_is_increasing) = if new_price >= last_price_twap {
        (new_price.safe_sub(e, last_price_twap), true)
    } else {
        (last_price_twap.safe_sub(e, new_price), false)
    };

    // cap new oracle update to 100/MAX_TWAP_UPDATE_PRICE_BAND_DENOMINATOR% delta from twap
    let sanitize_clamp_denominator = if sanitize_clamp_denominator != 0 {
        sanitize_clamp_denominator
    } else {
        DEFAULT_MAX_TWAP_UPDATE_PRICE_BAND_DENOMINATOR
    };

    if sanitize_clamp_denominator == 0 {
        // no need to use price band check
        return new_price;
    }

    let price_twap_price_band = last_price_twap.safe_div(e, sanitize_clamp_denominator as u128);

    let capped_update_price = if new_price_spread > price_twap_price_band {
        if price_is_increasing {
            last_price_twap.safe_add(e, price_twap_price_band)
        } else {
            if price_twap_price_band >= last_price_twap {
                0
            } else {
                last_price_twap.safe_sub(e, price_twap_price_band)
            }
        }
    } else {
        new_price
    };

    capped_update_price
}

/// Computes a fee using ceiling rounding in fixed-point arithmetic.
///
/// The fee is calculated as:
///
/// ```text
/// fee = ceil(amount * fee_fraction / FEE_MULTIPLIER)
/// ```
///
/// where `fee_fraction` and `FEE_MULTIPLIER` share the same scale
/// (e.g., `fee_fraction = 30` and `FEE_MULTIPLIER = 10_000` for 30 bps).
/// Ceiling rounding ensures the protocol never under-collects due to truncation.
///
/// # Arguments
///
/// * `amount` — The gross amount to which the fee applies.
/// * `fee_fraction` — The fee numerator in the same scale as `FEE_MULTIPLIER` (e.g., bps).
///
/// # Returns
///
/// * `u128` — The fee, rounded up (ceiling).
///   Returns `0` if an overflow occurs inside `fixed_mul_ceil` (current behavior).
///
/// # Notes
///
/// * This uses `fixed_mul_ceil` and propagates its overflow handling by returning `0`
///   on `None` via `unwrap_or(0)`. Consider replacing with explicit error handling
///   if you need to distinguish overflow from a legitimate zero fee.
///
/// # Examples
///
/// ```rust
/// // 30 bps on 1_000_000 with ceiling rounding
/// // fee = ceil(1_000_000 * 30 / 10_000) = ceil(3_000 / 10) = 300
/// let fee = calculate_fee(1_000_000, 30);
/// assert_eq!(fee, 300);
/// ```
pub fn calculate_fee(amount: u128, fee_fraction: u32) -> u128 {
    let result = amount
        .fixed_mul_ceil(fee_fraction as u128, FEE_MULTIPLIER)
        .unwrap_or(0);

    result
}
