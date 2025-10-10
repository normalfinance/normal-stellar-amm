use core::cmp::max;

use pool_validation_errors::PoolValidationError;
use soroban_sdk::{contracttype, panic_with_error, Env};
use token_share::get_total_shares;
use utils::{
    constant::{FEE_MULTIPLIER, PRICE_PRECISION_I64, TWENTY_FOUR_HOUR},
    math::{
        safe_math::{SafeConversion, SafeMath},
        stats::calculate_rolling_sum,
    },
};

use crate::storage::{get_base_tax, get_last_liquidity_withdrawal_ts, get_min_tax_price_deviation};

/**
 * Circuit Breaker Rules
 *
 * - Deviation b/t the pool and peg price larger than some threshold
 * - Too much liquidity is withdrawn quickly
 * - A trade exceeds slippage limit (i.e. 20%)
 */

pub fn check_withdrawal(e: &Env, share_amount: u128, current_time: u64) {
    let total_shares = get_total_shares(e);

    let since_last = max(
        1_u64,
        current_time.safe_sub(e, get_last_liquidity_withdrawal_ts(e)),
    );

    // let sum = calculate_rolling_sum(
    //         e,
    //         volume_30d,
    //         quote_asset_amount,
    //         since_last,
    //         TWENTY_FOUR_HOUR,
    //     );
    let lp_withdrawal_ema = calculate_rolling_sum(e, 0, share_amount, since_last, TWENTY_FOUR_HOUR);
    let max_perc = 0;

    if share_amount / total_shares > max_perc {
        panic_with_error!(e, PoolValidationError::CircuitBreaker)
    }
}

pub fn is_flipped(e: &Env, pool_price: u128, oracle_price: u128) -> bool {
    false

    // TODO: add circuit breakers

    // // Find % difference b/t pool price and oracle price
    // // let oracle_pool_price_spread_pct =
    // //     calculate_oracle_twap_price_spread_pct(e, pool_price, peg_price);

    // // let is_oracle_price_too_divergent =
    // //     is_oracle_price_too_divergent(oracle_pool_price_spread_pct, oracle_guard_rails);

    // // if is_oracle_price_too_divergent {
    // //     panic_with_error!(e, PoolError::InvalidOracle);
    // // }
}
