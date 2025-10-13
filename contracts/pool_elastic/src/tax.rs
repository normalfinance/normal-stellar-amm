use soroban_fixed_point_math::SorobanFixedPoint;
use soroban_sdk::{contracttype, Env};
use utils::{
    constant::{PRICE_PRECISION, PRICE_PRECISION_I64},
    math::safe_math::{SafeConversion, SafeMath},
};

use crate::storage::{get_base_tax, get_min_tax_price_deviation};

// #[contracttype]
// #[derive(Default, Clone, Copy, Debug)]
// pub struct TaxConfig {
//     pub min_tax_price_deviation: u128,
//     pub base_tax: u32,
//     pub tax_scaling_factor: u32,
//     pub tax_scaling_rate: u32,
// }

pub fn is_trade_taxable(e: &Env, pool_price: u128, oracle_price: u128) -> bool {
    let min_price_deviation = get_min_tax_price_deviation(e);

    let price_spread_pct = calculate_price_spread_pct(e, pool_price, oracle_price);

    if price_spread_pct.abs() >= (min_price_deviation as i64) {
        true
    } else {
        false
    }
}

pub fn calculate_price_spread_pct(e: &Env, price_a: u128, price_b: u128) -> i64 {
    // Use safe conversions to prevent overflow
    let price_a_i128 = price_a.safe_to_i128(e);
    let price_b_i128 = price_b.safe_to_i128(e);

    let price_spread_i128 = price_a_i128.safe_sub(e, price_b_i128);

    // Safe conversion to i64 with overflow protection
    let price_spread = price_spread_i128.safe_to_i64(e);
    let price_a_i64 = price_a.safe_to_i64(e);

    // Calculate (price_spread * PRICE_PRECISION_I64) / price_a_i64 using safe arithmetic
    let numerator = price_spread.safe_mul(e, PRICE_PRECISION_I64);
    numerator.safe_div(e, price_a_i64)
}
