use soroban_fixed_point_math::SorobanFixedPoint;
use soroban_sdk::Env;
use utils::constant::PRICE_PRECISION;
use utils::math::safe_math::{PrecisionMath, SafeConversion, SafeMath};

use crate::storage::{get_base_tax_fraction, get_max_tax_fraction, get_tax_incline};


//TODO: Make this table configurable in storage
/// Tax curve based on exponential deviation from peg.
/// Example reference points:
/// | Deviation | Tax rate |
/// |-----------|----------|
/// | 0%        | 0.10%    |
/// | 2%        | 1.1%     |
/// | 5%        | 5.5%     |
/// | 10%       | 27%      |
/// | 20%       | 49%      |
pub fn calculate_tax_rate(e: &Env, pool_price: u128, peg_price: u128) -> u32 {
    // Guard against zero division
    if peg_price == 0 || pool_price == 0 {
        return get_base_tax_fraction(e);
    }

    let base_tax = get_base_tax_fraction(e);
    let max_tax = get_max_tax_fraction(e);
    let k = get_tax_incline(e);

    // Calculate absolute deviation: |pool_price - peg_price| / peg_price
    let price_diff = if pool_price > peg_price {
        pool_price.safe_sub(e, peg_price)
    } else {
        peg_price.safe_sub(e, pool_price)
    };

    // abs_deviation = |price_diff| / peg_price (scaled by PRICE_PRECISION)
    let abs_deviation = price_diff.safe_fixed_div_round(e, peg_price, PRICE_PRECISION);

    // For small deviations, return base tax
    if abs_deviation == 0 {
        return base_tax;
    }

    // Calculate exponential curve: base_tax + (max_tax - base_tax) * (1 - e^(-k * abs_dev))
    // Using Taylor series approximation for e^(-x): e^(-x) ≈ 1 - x + x²/2 - x³/6
    // where x = k * abs_deviation / PRICE_PRECISION

    // Calculate k * abs_deviation (already scaled)
    let x = (k as u128).safe_fixed_mul_floor(e, abs_deviation, PRICE_PRECISION);

    // For very large deviations, return max tax
    if x > PRICE_PRECISION * 5 {
        return max_tax;
    }

    // Calculate Taylor series: 1 - x + x²/2 - x³/6
    let x2 = x.safe_fixed_mul_floor(e, x, PRICE_PRECISION);
    let x3 = x2.safe_fixed_mul_floor(e, x, PRICE_PRECISION);

    // exp_neg = 1 - x + x²/2 - x³/6
    let mut exp_neg = PRICE_PRECISION;
    exp_neg = exp_neg.safe_sub(e, x);
    exp_neg = exp_neg.safe_add(e, x2.safe_div(e, 2));
    
    // Guard against negative values from approximation
    if x3.safe_div(e, 6) < exp_neg {
        exp_neg = exp_neg.safe_sub(e, x3.safe_div(e, 6));
    } else {
        exp_neg = 0;
    }

    // Clamp exp_neg within [0, PRICE_PRECISION]
    let exp_neg_clamped = exp_neg.min(PRICE_PRECISION);

    // Calculate: base_tax + (max_tax - base_tax) * (1 - exp_neg)
    let diff = max_tax.safe_sub(e, base_tax);
    let one_minus_exp = PRICE_PRECISION.safe_sub(e, exp_neg_clamped);
    
    let tax_increase = (diff as u128)
        .safe_fixed_mul_floor(e, one_minus_exp, PRICE_PRECISION)
        .safe_to_u32(e);
    
    let mut tax_rate = base_tax.safe_add(e, tax_increase);

    // Cap at max_tax to be safe from rounding
    if tax_rate > max_tax {
        tax_rate = max_tax;
    }

    tax_rate
}

/// Calculates the tax amount to be collected from a trade.
/// Returns the tax amount in token units.
pub fn calculate_tax_amount(
    e: &Env,
    trade_amount: u128,
    pool_price: u128,
    peg_price: u128,
) -> u128 {
    if pool_price == 0 || peg_price == 0 || trade_amount == 0 {
        return 0;
    }

    let tax_rate = calculate_tax_rate(e, pool_price, peg_price);

    // Apply tax rate to trade amount: trade_amount * tax_rate / PRICE_PRECISION
    trade_amount.fixed_mul_floor(e, &(tax_rate as u128), &PRICE_PRECISION)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::ElasticPool;

    fn test_env_with_contract() -> (Env, soroban_sdk::Address) {
        let e = Env::default();
        e.mock_all_auths();
        let contract_id = e.register_contract(None, ElasticPool);
        (e, contract_id)
    }

    #[test]
    fn test_calculate_tax_rate_zero_prices() {
        let (e, contract_id) = test_env_with_contract();

        let (base_tax_fraction, tax_rate_0, tax_rate_1, tax_rate_2) = e.as_contract(&contract_id, || {
            let base_tax_fraction = get_base_tax_fraction(&e);
            let tax_rate_0 = calculate_tax_rate(&e, 0, 0);
            let tax_rate_1 = calculate_tax_rate(&e, 1_0000000, 0);
            let tax_rate_2 = calculate_tax_rate(&e, 0, 1_0000000);
            (base_tax_fraction, tax_rate_0, tax_rate_1, tax_rate_2)
        });

        assert_eq!(tax_rate_0, base_tax_fraction);
        assert_eq!(tax_rate_1, base_tax_fraction);
        assert_eq!(tax_rate_2, base_tax_fraction);
    }

    #[test]
    fn test_calculate_tax_rate_at_peg() {
        let (e, contract_id) = test_env_with_contract();

        let (base_tax_fraction, tax_rate) = e.as_contract(&contract_id, || {
            let base_tax_fraction = get_base_tax_fraction(&e);
            let tax_rate = calculate_tax_rate(&e, 1_0000000, 1_0000000);
            (base_tax_fraction, tax_rate)
        });

        assert_eq!(tax_rate, base_tax_fraction);
    }

    #[test]
    fn test_calculate_tax_rate_small_deviation() {
        let (e, contract_id) = test_env_with_contract();

        let (base_tax_fraction, tax_rate) = e.as_contract(&contract_id, || {
            let base_tax_fraction = get_base_tax_fraction(&e);
            let tax_rate = calculate_tax_rate(&e, 1_0100000, 1_0000000);
            (base_tax_fraction, tax_rate)
        });

        // With small deviation (1%), tax rate should be close to base rate
        // The actual increase depends on the tax curve parameters
        assert!(tax_rate >= base_tax_fraction);
        assert!(tax_rate < 10000); // Should be reasonable
    }

    #[test]
    fn test_calculate_tax_rate_large_deviation() {
        let (e, contract_id) = test_env_with_contract();

        let (base_tax_fraction, max_tax_fraction, tax_rate) = e.as_contract(&contract_id, || {
            let base_tax_fraction = get_base_tax_fraction(&e);
            let max_tax_fraction = get_max_tax_fraction(&e);
            let tax_rate = calculate_tax_rate(&e, 1_2000000, 1_0000000);
            (base_tax_fraction, max_tax_fraction, tax_rate)
        });

        // Verify tax rate is within valid bounds
        assert!(tax_rate >= base_tax_fraction);
        assert!(tax_rate <= max_tax_fraction);
    }

    #[test]
    fn test_calculate_tax_amount_zero_prices() {
        let (e, contract_id) = test_env_with_contract();
        let trade_amount = 100_0000000;

        let (tax_amount_0, tax_amount_1, tax_amount_2) = e.as_contract(&contract_id, || {
            let tax_amount_0 = calculate_tax_amount(&e, trade_amount, 0, 0);
            let tax_amount_1 = calculate_tax_amount(&e, trade_amount, 1_0000000, 0);
            let tax_amount_2 = calculate_tax_amount(&e, trade_amount, 0, 1_0000000);
            (tax_amount_0, tax_amount_1, tax_amount_2)
        });

        assert_eq!(tax_amount_0, 0);
        assert_eq!(tax_amount_1, 0);
        assert_eq!(tax_amount_2, 0);
    }

    #[test]
    fn test_calculate_tax_amount_at_peg() {
        let (e, contract_id) = test_env_with_contract();
        let trade_amount = 100_0000000;

        let tax_amount = e.as_contract(&contract_id, || {
            calculate_tax_amount(&e, trade_amount, 1_0000000, 1_0000000)
        });

        // At peg, only base tax applies
        // With base_tax_fraction = 100 and PRICE_PRECISION = 10_000_000:
        // 100_0000000 * 100 / 10_000_000 = 10000
        assert_eq!(tax_amount, 10000);
    }

    #[test]
    fn test_calculate_tax_amount_with_deviation() {
        let (e, contract_id) = test_env_with_contract();
        let trade_amount = 100_0000000;

        let (base_tax_amount, tax_amount_with_dev) = e.as_contract(&contract_id, || {
            let base_tax_amount = calculate_tax_amount(&e, trade_amount, 1_0000000, 1_0000000);
            // 10% deviation
            let tax_amount_with_dev = calculate_tax_amount(&e, trade_amount, 1_1000000, 1_0000000);
            (base_tax_amount, tax_amount_with_dev)
        });

        // Verify tax amount is within valid bounds (at least base tax, less than half of trade)
        assert!(tax_amount_with_dev >= base_tax_amount);
        assert!(tax_amount_with_dev < trade_amount / 2); // Less than 50% of trade amount
    }
}
