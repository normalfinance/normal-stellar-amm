use soroban_fixed_point_math::FixedPoint;
use soroban_sdk::{ Env };
use utils::{ constant::PRICE_PRECISION, math::safe_math::SafeMath };

// Calculates the utilization percentage of the insurance fund.
//
// # Arguments
//
// * `insurance_vault_amount` - The current balance in the insurance vault, in fixed-point units.
// * `optimal_coverage` - The target or optimal coverage amount, in fixed-point units.
//
// # Returns
//
// * The utilization percentage as a fixed-point `u32` (e.g. 1_000_000 = 100%).
//   Returns 0 if either input is zero. The result is scaled by `PRICE_PRECISION`.
pub fn calculate_utilization(insurance_vault_amount: u128, optimal_coverage: u128) -> u32 {
    if insurance_vault_amount == 0 || optimal_coverage == 0 {
        return 0;
    }

    // is this safe to cast u128 down to i32?
    insurance_vault_amount.fixed_div_floor(optimal_coverage, PRICE_PRECISION).unwrap_or(0) as u32
    //  result.min(u32::MAX as u128) as u32
}

// Calculates the interest rate based on utilization using a two-slope model.
//
// # Arguments
//
// * `e` - The Soroban environment.
// * `utilization` - The current utilization as a percentage in basis points (e.g. 8000 = 80%).
// * `optimal_utilization` - The target utilization percentage in basis points where the slope changes.
// * `base_rate` - The base interest rate in basis points (can be negative).
// * `slope1` - The interest rate slope below or at the optimal utilization.
// * `slope2` - The interest rate slope above the optimal utilization.
//
// # Returns
//
// * The calculated interest rate in basis points as an `i32`.
pub fn calculate_rate(
    e: &Env,
    utilization: u32,
    optimal_utilization: u32,
    base_rate: i32,
    slope1: i32,
    slope2: i32
) -> i32 {
    let utilization = utilization as i32;
    let optimal_utilization = optimal_utilization as i32;

    if utilization <= optimal_utilization {
        let utilization_ratio = utilization.safe_div(e, optimal_utilization);
        let variable_rate = utilization_ratio.safe_mul(e, slope1);
        base_rate.safe_add(e, variable_rate)
    } else {
        let excess_utilization = utilization.safe_sub(e, optimal_utilization);
        let remaining = (10_000_i32).safe_sub(e, optimal_utilization); // Assuming 100% = 10_000
        let excess_ratio = excess_utilization.safe_div(e, remaining);
        let slope2_part = excess_ratio.safe_mul(e, slope2);
        base_rate.safe_add(e, slope1).safe_add(e, slope2_part)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // utilization

    #[test]
    fn test_utilization_100_percent() {
        let utilization = calculate_utilization(
            1_000_000 * PRICE_PRECISION,
            1_000_000 * PRICE_PRECISION
        );
        assert_eq!(utilization, 1_000_000); // 1.0 in fixed-point
    }

    #[test]
    fn test_utilization_50_percent() {
        let utilization = calculate_utilization(
            500_000 * PRICE_PRECISION,
            1_000_000 * PRICE_PRECISION
        );
        assert_eq!(utilization, 500_000); // 0.5 in fixed-point
    }

    #[test]
    fn test_utilization_above_100_percent() {
        let utilization = calculate_utilization(
            2_000_000 * PRICE_PRECISION,
            1_000_000 * PRICE_PRECISION
        );
        assert_eq!(utilization, 2_000_000); // 2.0
    }

    #[test]
    fn test_zero_coverage_returns_zero() {
        let utilization = calculate_utilization(1_000_000 * PRICE_PRECISION, 0);
        assert_eq!(utilization, 0); // Prevent division by zero
    }

    #[test]
    fn test_utilization_clamps_to_u32_max() {
        let huge_value = u128::MAX;
        let utilization = calculate_utilization(huge_value, 1); // absurdly high ratio
        assert_eq!(utilization, u32::MAX); // Clamped
    }

    #[test]
    fn test_zero_vault_amount() {
        let utilization = calculate_utilization(0, 1_000_000 * PRICE_PRECISION);
        assert_eq!(utilization, 0); // 0% utilization
    }

    // interest rate

    #[test]
    fn test_zero_utilization() {
        let e = Env::default();
        let rate = calculate_rate(&e, 0, 8000, 100, 400, 1500);
        assert_eq!(rate, 100); // Only base rate should apply
    }

    #[test]
    fn test_negative_utilization() {
        let e = Env::default();
        let rate = calculate_rate(&e, -1000, 8000, 100, 400, 1500);
        // This should still compute: 100 + (-1000 * 400 / 8000) = 100 - 50 = 50
        assert_eq!(rate, 50);
    }

    #[test]
    fn test_negative_base_rate_and_slopes() {
        let e = Env::default();
        let rate = calculate_rate(&e, 5000, 8000, -100, -400, -1500);
        // -100 + (5000 / 8000 * -400) = -100 - 250 = -350
        assert_eq!(rate, -350);
    }

    #[test]
    fn test_low_utilization_rate() {
        let e = Env::default();

        let utilization = 5000; // 50%
        let base_rate = 100; // 1.00%
        let slope_a = 400; // 4.00% max at optimal
        let slope_b = 1500; // 15.00% for high usage
        let optimal_util = 8000; // 80%

        let rate = calculate_rate(&e, utilization, optimal_util, base_rate, slope_a, slope_b);
        // Expected: base + (util / optimal) * slope_a
        // 100 + (5000 / 8000 * 400) = 100 + 250 = 350
        assert_eq!(rate, 350);
    }

    #[test]
    fn test_utilization_at_optimal() {
        let e = Env::default();

        let utilization = 8000; // 80%
        let base_rate = 100;
        let slope_a = 400;
        let slope_b = 1500;
        let optimal_util = 8000;

        let rate = calculate_rate(&e, utilization, optimal_util, base_rate, slope_a, slope_b);
        // Should be base + slope_a
        assert_eq!(rate, base_rate + slope_a);
    }

    #[test]
    fn test_high_utilization_rate() {
        let e = Env::default();

        let utilization = 9500; // 95%
        let base_rate = 100;
        let slope_a = 400;
        let slope_b = 1500;
        let optimal_util = 8000;

        let rate = calculate_rate(&e, utilization, optimal_util, base_rate, slope_a, slope_b);
        // base + slope_a + ((util - opt) / (1 - opt)) * slope_b
        // 100 + 400 + ((1500 / 2000) * 1500) = 100 + 400 + 1125 = 1625
        assert_eq!(rate, 1625);
    }

    #[test]
    fn test_max_utilization() {
        let e = Env::default();

        let utilization = 10_000; // 100%
        let base_rate = 100;
        let slope_a = 400;
        let slope_b = 1500;
        let optimal_util = 8000;

        let rate = calculate_rate(&e, utilization, optimal_util, base_rate, slope_a, slope_b);
        // base + slope_a + (2000 / 2000 * slope_b)
        assert_eq!(rate, base_rate + slope_a + slope_b);
    }
}
