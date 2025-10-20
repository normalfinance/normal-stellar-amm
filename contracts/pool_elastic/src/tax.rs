// use soroban_fixed_point_math::SorobanFixedPoint;
// use soroban_sdk::Env;
// use utils::constant::PRICE_PRECISION;

// use crate::storage::get_base_tax_fraction;

// // | Deviation | Tax rate |
// // | --------- | -------- |
// // | 0%        | 0.10%    |
// // | 2%        | 1.1%     |
// // | 5%        | 5.5%     |
// // | 10%       | 27%      |
// // | 20%       | 49%      |
// pub fn calculate_tax_rate(e: &Env, pool_price: u128, peg_price: u128) -> u32 {
//     // Guard against zero division
//     if peg_price == 0 || pool_price == 0 {
//         return get_base_tax_fraction(e);
//     }

//     // let deviation = (pool_price / peg_price) - 1.0;
//     // let abs_dev = deviation.abs();

//     let base_tax = get_base_tax_fraction(e);
//     // let max_tax = get_max_tax_fraction(e);
//     // let k = get_tax_incline(e);

//     // let tax_rate = base_tax + (max_tax - base_tax) * (1.0 - (-k * abs_dev).exp());

//     // Cap at max_tax to be safe from rounding
//     // tax_rate.min(max_tax)

//     base_tax
// }

// /// Tax curve based on exponential deviation from peg.
// /// Example reference points:
// /// | Deviation | Tax rate |
// /// |------------|-----------|
// /// | 0%         | 0.10%     |
// /// | 2%         | 1.1%      |
// /// | 5%         | 5.5%      |
// /// | 10%        | 27%       |
// /// | 20%        | 49%       |
// // pub fn calculate_tax_rate(e: &Env, pool_price: u128, peg_price: u128) -> u32 {
// //     // Guard against zero division
// //     if peg_price == 0 {
// //         return get_max_tax_fraction(e);
// //     }

// //     // ratio = pool_price / peg_price (fixed-point)
// //     let ratio = pool_price.safe_fixed_div_floor(e, peg_price, PRICE_PRECISION);

// //     // deviation = |ratio - 1.0|
// //     let deviation = if ratio > PRICE_PRECISION {
// //         ratio.safe_sub(e, PRICE_PRECISION)
// //     } else {
// //         PRICE_PRECISION.safe_sub(e, ratio)
// //     };

// //     // Load curve parameters (scaled to PRICE_PRECISION)
// //     let base_tax = get_base_tax_fraction(e);
// //     let max_tax = get_max_tax_fraction(e);
// //     let k = get_tax_incline(e); // incline coefficient (scaled)

// //     // Compute exp(-k * abs_dev / PRICE_PRECISION)
// //     // Using truncated Taylor series for fixed-point: e^(-x) ≈ 1 - x + x²/2 - x³/6
// //     let x = k.safe_fixed_mul_floor(e, deviation, PRICE_PRECISION);
// //     let x2 = x.safe_fixed_mul_floor(e, x, PRICE_PRECISION);
// //     let x3 = x2.safe_fixed_mul_floor(e, x, PRICE_PRECISION);

// //     let exp_neg = PRICE_PRECISION
// //         .safe_sub(e, x)
// //         .safe_add(e, x2.safe_div(e, 2))
// //         .safe_sub(e, x3.safe_div(e, 6));

// //     // Clamp exp_neg within [0, 1]
// //     let exp_neg_clamped = exp_neg.min(PRICE_PRECISION);

// //     // tax_rate = base_tax + (max_tax - base_tax) * (1 - exp_neg)
// //     let diff = max_tax.safe_sub(e, base_tax);
// //     let one_minus_exp = PRICE_PRECISION.safe_sub(e, exp_neg_clamped);
// //     let mut tax_rate = base_tax.safe_add(
// //         e,
// //         diff.safe_fixed_mul_floor(e, one_minus_exp, PRICE_PRECISION),
// //     );

// //     // Cap at max_tax
// //     if tax_rate > max_tax {
// //         tax_rate = max_tax;
// //     }

// //     tax_rate
// // }
// pub fn calculate_tax_amount(
//     e: &Env,
//     trade_amount: u128,
//     pool_price: u128,
//     peg_price: u128,
// ) -> u128 {
//     if pool_price == 0 || peg_price == 0 {
//         return 0;
//     }

//     let tax_rate = calculate_tax_rate(e, pool_price, peg_price);

//     trade_amount.fixed_mul_floor(e, &(tax_rate as u128), &PRICE_PRECISION)
// }

// #[cfg(test)]
// mod test {
//     use super::*;

//     #[test]
//     fn test_calculate_tax_rate_zero_prices() {
//         let e = Env::default();
//         e.mock_all_auths();

//         let base_tax_fraction = get_base_tax_fraction(&e);

//         assert_eq!(calculate_tax_rate(&e, 0, 0), base_tax_fraction);
//         assert_eq!(calculate_tax_rate(&e, 1, 0), base_tax_fraction);
//         assert_eq!(calculate_tax_rate(&e, 0, 1), base_tax_fraction);
//     }

//     #[test]
//     fn test_calculate_tax_rate() {
//         let e = Env::default();
//         e.mock_all_auths();

//         let base_tax_fraction = get_base_tax_fraction(&e);

//         let tax_rate = calculate_tax_rate(&e, 1_0000000, 1_0000000);

//         assert_eq!(tax_rate, base_tax_fraction);
//     }

//     #[test]
//     fn test_calculate_tax_amount_zero_prices() {
//         let e = Env::default();
//         e.mock_all_auths();

//         let trade_amount = 100_0000000;

//         assert_eq!(calculate_tax_amount(&e, trade_amount, 0, 0), 0);
//         assert_eq!(calculate_tax_amount(&e, trade_amount, 1_0000000, 0), 0);
//         assert_eq!(calculate_tax_amount(&e, trade_amount, 0, 1_0000000), 0);
//     }

//     #[test]
//     fn test_calculate_tax_amount() {
//         let e = Env::default();
//         e.mock_all_auths();

//         let trade_amount = 100_0000000;

//         let tax_amount = calculate_tax_amount(&e, trade_amount, 1_0000000, 1_0000000);

//         assert_eq!(tax_amount, 0_1000000);
//     }
// }
