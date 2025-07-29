use soroban_fixed_point_math::FixedPoint;
use utils::constant::{ PERCENTAGE_PRECISION, PERCENTAGE_PRECISION_I64 };

// Calculates the utilization percentage of the insurance fund.
//
// # Arguments
//
// * `insurance_vault_amount` - The current balance in the insurance vault, in fixed-point units.
// * `optimal_insurance` - The target or optimal coverage amount, in fixed-point units.
//
// # Returns
//
// * The utilization percentage as a fixed-point `u32` (e.g. 10_000_000 = 100%).
//   Returns 0 if either input is zero. The result is scaled by `PERCENTAGE_PRECISION`.
pub fn calculate_utilization(insurance_vault_amount: u128, optimal_insurance: u128) -> u32 {
    if insurance_vault_amount == 0 || optimal_insurance == 0 {
        return 0;
    }

    // is this safe to cast u128 down to i32?
    insurance_vault_amount
        .fixed_div_floor(optimal_insurance, PERCENTAGE_PRECISION)
        .unwrap_or(0) as u32
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
    utilization: u32,
    optimal_utilization: u32,
    base_rate: i32,
    slope1: u32,
    slope2: u32
) -> i32 {
    if utilization == 0 {
        return base_rate;
    }

    let utilization = utilization as i64;
    let optimal_utilization = optimal_utilization as i64;
    let base_rate = base_rate as i64;
    let slope1 = slope1 as i64;
    let slope2 = slope2 as i64;

    let rate = if utilization <= optimal_utilization {
        // rate = base + (utilization * slope1 / optimal_utilization)
        let variable_rate = utilization.fixed_mul_floor(slope1, optimal_utilization).unwrap();
        base_rate + variable_rate
    } else {
        // rate = base + slope1 + ((utilization - optimal_utilization) * slope2 / (10_000_000 - optimal_utilization))
        let excess_util = utilization - optimal_utilization;
        let remaining = PERCENTAGE_PRECISION_I64 - optimal_utilization;

        let slope2_part = excess_util.fixed_mul_floor(slope2, remaining).unwrap();

        base_rate + slope1 + slope2_part
    };

    rate as i32
}

#[cfg(test)]
mod tests {
    use utils::constant::{ PERCENTAGE_PRECISION_U32, PRICE_PRECISION };

    use super::*;

    // utilization

    #[test]
    fn test_utilization_100_percent() {
        let utilization = calculate_utilization(
            1_000_000 * PRICE_PRECISION,
            1_000_000 * PRICE_PRECISION
        );
        assert_eq!(utilization, 1 * PERCENTAGE_PRECISION_U32);
    }

    #[test]
    fn test_utilization_50_percent() {
        let utilization = calculate_utilization(
            500_000 * PRICE_PRECISION,
            1_000_000 * PRICE_PRECISION
        );
        assert_eq!(utilization, 5_000_000); // 0.5%
    }

    #[test]
    fn test_utilization_above_100_percent() {
        let utilization = calculate_utilization(
            2_000_000 * PRICE_PRECISION,
            1_000_000 * PRICE_PRECISION
        );
        assert_eq!(utilization, 2 * PERCENTAGE_PRECISION_U32);
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

    #[test]
    #[should_panic(expected = "Error(Contract, #21)")]
    fn test_zero_optimal_utilization() {
        // optimal_utilization = 0 could panic on division unless handled
        calculate_rate(5000, 0, 100, 400, 1500);
    }

    #[test]
    fn test_utilization_above_100_percent_in_rate() {
        let rate = calculate_rate(11_000, 8000, 100, 400, 1500);
        // excess = 3000, remaining = 2000
        // rate = 100 + 400 + (3000 / 2000) * 1500 = 100 + 400 + 2250 = 2750
        assert_eq!(rate, 2750);
    }

    // interest rate

    #[test]
    fn test_zero_utilization() {
        let rate = calculate_rate(0, 8000, 100, 400, 1500);
        assert_eq!(rate, 100); // Only base rate should apply
    }

    #[test]
    fn test_negative_base_rate() {
        let rate = calculate_rate(5000, 8000, -100, 400, 1500);
        // -100 + (5000 / 8000 * 400) = -100 + 250 = 150
        assert_eq!(rate, 150);
    }

    #[test]
    fn test_negative_rate() {
        let rate = calculate_rate(11000, 8000, -1000, 200, 1000);
        // -100 + (5000 / 8000 * 400) = -100 + 250 = 150
        assert_eq!(rate, -975);
    }

    #[test]
    fn test_low_utilization_rate() {
        let rate = calculate_rate(5000, 8000, 100, 400, 1500);
        // 100 + (5000 / 8000 * 400) = 100 + 250 = 350
        assert_eq!(rate, 350);
    }

    #[test]
    fn test_utilization_at_optimal() {
        let rate = calculate_rate(8000, 8000, 100, 400, 1500);
        // base + slope_a = 100 + 400 = 500
        assert_eq!(rate, 500);
    }

    #[test]
    fn test_high_utilization_rate() {
        let rate = calculate_rate(9500, 8000, 100, 400, 1500);
        // base + slope_a + ((1500 / 2000) * 1500) = 100 + 400 + 1125 = 1625
        assert_eq!(rate, 1625);
    }

    #[test]
    fn test_max_utilization() {
        let rate = calculate_rate(10_000, 8000, 100, 400, 1500);
        // base + slope_a + slope_b = 100 + 400 + 1500 = 2000
        assert_eq!(rate, 2000);
    }
}
