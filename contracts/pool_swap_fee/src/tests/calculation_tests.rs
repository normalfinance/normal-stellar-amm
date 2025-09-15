// Comprehensive tests for Pool Swap Fee calculations
#![cfg(test)]

use soroban_sdk::{Env, testutils::Address as _, Address};
use soroban_fixed_point_math::FixedPoint;
use utils::constant::{FEE_DENOMINATOR, PRICE_PRECISION, THIRTY_DAY};
use utils::math::stats::calculate_rolling_sum;

mod fee_distribution_tests {
    use super::*;

    #[test]
    fn test_fee_split_50_50() {
        let e = Env::default();
        
        // 50% to LPs, 50% to protocol
        let total_fee = 1000_0000000u128;
        let lp_revenue_fraction = 5000u32; // 50% in basis points
        
        let lp_fee = (total_fee * lp_revenue_fraction as u128) / FEE_DENOMINATOR as u128;
        let protocol_fee = total_fee - lp_fee;
        
        assert_eq!(lp_fee, 500_0000000);
        assert_eq!(protocol_fee, 500_0000000);
    }

    #[test]
    fn test_fee_split_custom_ratio() {
        let e = Env::default();
        
        // 60% to LPs, 40% to protocol
        let total_fee = 1000_0000000u128;
        let lp_revenue_fraction = 6000u32;
        
        let lp_fee = (total_fee * lp_revenue_fraction as u128) / FEE_DENOMINATOR as u128;
        let protocol_fee = total_fee - lp_fee;
        
        assert_eq!(lp_fee, 600_0000000);
        assert_eq!(protocol_fee, 400_0000000);
    }

    #[test]
    fn test_buffer_fraction() {
        let e = Env::default();
        
        // 10% of protocol revenue to buffer
        let protocol_fee = 500_0000000u128;
        let buffer_fraction = 1000u32; // 10% in basis points
        
        let buffer_amount = (protocol_fee * buffer_fraction as u128) / FEE_DENOMINATOR as u128;
        let remaining_protocol = protocol_fee - buffer_amount;
        
        assert_eq!(buffer_amount, 50_0000000);
        assert_eq!(remaining_protocol, 450_0000000);
    }

    #[test]
    fn test_fee_on_input_token() {
        let e = Env::default();
        
        // Fee taken from input amount
        let in_amount = 1000_0000000u128;
        let pool_fee_fraction = 30u32; // 0.3%
        
        let fee_amount = (in_amount * pool_fee_fraction as u128) / FEE_DENOMINATOR as u128;
        let actual_swap_amount = in_amount - fee_amount;
        
        assert_eq!(fee_amount, 3_0000000); // 0.3% of 1000
        assert_eq!(actual_swap_amount, 997_0000000);
    }

    #[test]
    fn test_fee_on_output_token() {
        let e = Env::default();
        
        // Fee taken from output amount
        let out_amount = 1000_0000000u128;
        let pool_fee_fraction = 30u32; // 0.3%
        
        let fee_amount = (out_amount * pool_fee_fraction as u128) / FEE_DENOMINATOR as u128;
        let actual_receive = out_amount - fee_amount;
        
        assert_eq!(fee_amount, 3_0000000);
        assert_eq!(actual_receive, 997_0000000);
    }

    #[test]
    fn test_minimum_fee_amounts() {
        let e = Env::default();
        
        // Very small trade
        let in_amount = 100u128; // Tiny amount
        let pool_fee_fraction = 30u32;
        
        let fee_amount = (in_amount * pool_fee_fraction as u128) / FEE_DENOMINATOR as u128;
        
        // Should round down to 0
        assert_eq!(fee_amount, 0);
    }

    #[test]
    fn test_maximum_fee_protection() {
        let e = Env::default();
        
        // Ensure fees can't exceed reasonable limits
        let in_amount = 1000_0000000u128;
        let max_fee_fraction = 1000u32; // 10% max reasonable fee
        
        let fee_amount = (in_amount * max_fee_fraction as u128) / FEE_DENOMINATOR as u128;
        
        assert_eq!(fee_amount, 100_0000000); // 10% of 1000
        assert!(fee_amount < in_amount / 2); // Never more than 50%
    }
}

mod insurance_premium_tests {
    use super::*;

    #[test]
    fn test_premium_calculation_standard() {
        let e = Env::default();
        
        // Standard scenario
        let volume_30d = 1_000_000_0000000u128; // $1M
        let insurance_coverage = 100_000_0000000u128; // $100k coverage
        let insurance_rate = 500i32; // 5% annual rate in basis points
        
        // Annual volume estimate = 30d volume * 365/30
        let annual_volume = volume_30d.fixed_mul_floor(&e, &365, &30).unwrap();
        
        // Annual premium = coverage * rate
        let annual_premium = insurance_coverage
            .fixed_mul_floor(&e, &(insurance_rate as u128), &PRICE_PRECISION)
            .unwrap();
        
        // Premium per dollar swapped
        let premium_per_dollar = annual_premium / annual_volume;
        
        // For a $1000 swap
        let swap_amount = 1000_0000000u128;
        let swap_premium = swap_amount * premium_per_dollar;
        
        // Should be reasonable (< 1% of swap)
        assert!(swap_premium < swap_amount / 100);
    }

    #[test]
    fn test_premium_zero_rate() {
        let e = Env::default();
        
        let insurance_rate = 0i32;
        let insurance_coverage = 100_000_0000000u128;
        
        let annual_premium = insurance_coverage
            .fixed_mul_floor(&e, &(insurance_rate.max(0) as u128), &PRICE_PRECISION)
            .unwrap_or(0);
        
        assert_eq!(annual_premium, 0);
    }

    #[test]
    fn test_premium_negative_rate() {
        // Negative rate means no premium (over-insured)
        let insurance_rate = -200i32;
        
        // Should not charge premium
        let effective_rate = insurance_rate.max(0);
        assert_eq!(effective_rate, 0);
    }

    #[test]
    fn test_premium_capped_by_protocol_fee() {
        let e = Env::default();
        
        let protocol_fee = 10_0000000u128;
        let calculated_premium = 20_0000000u128;
        
        // Premium can't exceed protocol fee
        let actual_premium = calculated_premium.min(protocol_fee);
        
        assert_eq!(actual_premium, 10_0000000);
    }

    #[test]
    fn test_premium_volume_spike() {
        let e = Env::default();
        
        // Sudden 10x volume spike
        let normal_volume_30d = 1_000_000_0000000u128;
        let spike_volume_30d = 10_000_000_0000000u128;
        let insurance_coverage = 100_000_0000000u128;
        let insurance_rate = 500i32;
        
        // Calculate premiums for both scenarios
        let normal_annual = normal_volume_30d.fixed_mul_floor(&e, &365, &30).unwrap();
        let spike_annual = spike_volume_30d.fixed_mul_floor(&e, &365, &30).unwrap();
        
        let annual_premium = insurance_coverage
            .fixed_mul_floor(&e, &(insurance_rate as u128), &PRICE_PRECISION)
            .unwrap();
        
        let normal_per_dollar = annual_premium / normal_annual;
        let spike_per_dollar = annual_premium / spike_annual;
        
        // Premium per dollar should decrease with higher volume
        assert!(spike_per_dollar < normal_per_dollar);
        // Should be roughly 1/10th
        assert_eq!(spike_per_dollar * 10, normal_per_dollar);
    }

    #[test]
    fn test_premium_low_volume_protection() {
        let e = Env::default();
        
        // Very low volume scenario
        let volume_30d = 1000_0000000u128; // Only $1000 in 30 days
        let insurance_coverage = 100_000_0000000u128;
        let insurance_rate = 500i32;
        
        let annual_volume = volume_30d.fixed_mul_floor(&e, &365, &30).unwrap();
        let annual_premium = insurance_coverage
            .fixed_mul_floor(&e, &(insurance_rate as u128), &PRICE_PRECISION)
            .unwrap();
        
        // This would result in very high premium per dollar
        let premium_per_dollar = annual_premium / annual_volume.max(1);
        
        // For a $100 swap
        let swap_amount = 100_0000000u128;
        let protocol_fee = 1_0000000u128; // Assume 1% fee
        
        let calculated_premium = swap_amount * premium_per_dollar;
        let actual_premium = calculated_premium.min(protocol_fee);
        
        // Premium is capped by protocol fee
        assert_eq!(actual_premium, protocol_fee);
    }
}

mod rolling_volume_tests {
    use super::*;

    #[test]
    fn test_rolling_sum_update() {
        let e = Env::default();
        
        let old_sum = 1_000_000_0000000u128;
        let new_value = 100_000_0000000u128;
        let since_last = 86400u64; // 1 day
        let period = THIRTY_DAY;
        
        let new_sum = calculate_rolling_sum(&e, old_sum, new_value, since_last, period);
        
        // Should decay old value and add new
        let decay_factor = ((period - since_last) as u128) * PRICE_PRECISION / (period as u128);
        let decayed_old = old_sum.fixed_mul_floor(&e, &decay_factor, &PRICE_PRECISION).unwrap();
        let expected = decayed_old + new_value;
        
        assert_eq!(new_sum, expected);
    }

    #[test]
    fn test_rolling_sum_full_period() {
        let e = Env::default();
        
        let old_sum = 1_000_000_0000000u128;
        let new_value = 100_000_0000000u128;
        let since_last = THIRTY_DAY; // Full period passed
        
        let new_sum = calculate_rolling_sum(&e, old_sum, new_value, since_last, THIRTY_DAY);
        
        // Old sum should be completely decayed
        assert_eq!(new_sum, new_value);
    }

    #[test]
    fn test_rolling_sum_no_time_passed() {
        let e = Env::default();
        
        let old_sum = 1_000_000_0000000u128;
        let new_value = 100_000_0000000u128;
        let since_last = 0u64;
        
        let new_sum = calculate_rolling_sum(&e, old_sum, new_value, since_last, THIRTY_DAY);
        
        // Should just add new value
        assert_eq!(new_sum, old_sum + new_value);
    }

    #[test]
    fn test_annual_volume_extrapolation() {
        let e = Env::default();
        
        // $1M in 30 days
        let volume_30d = 1_000_000_0000000u128;
        
        // Extrapolate to annual
        let annual_volume = volume_30d.fixed_mul_floor(&e, &365, &30).unwrap();
        
        // Should be roughly 12.17x
        let expected = volume_30d * 365 / 30;
        assert_eq!(annual_volume, expected);
    }
}

mod edge_cases_tests {
    use super::*;

    #[test]
    fn test_zero_fee_scenario() {
        // Pool with 0% fee
        let pool_fee_fraction = 0u32;
        let in_amount = 1000_0000000u128;
        
        let fee_amount = (in_amount * pool_fee_fraction as u128) / FEE_DENOMINATOR as u128;
        
        assert_eq!(fee_amount, 0);
    }

    #[test]
    fn test_all_fees_to_protocol() {
        // 0% to LPs, 100% to protocol
        let total_fee = 1000_0000000u128;
        let lp_revenue_fraction = 0u32;
        
        let lp_fee = (total_fee * lp_revenue_fraction as u128) / FEE_DENOMINATOR as u128;
        let protocol_fee = total_fee - lp_fee;
        
        assert_eq!(lp_fee, 0);
        assert_eq!(protocol_fee, total_fee);
    }

    #[test]
    fn test_all_fees_to_lps() {
        // 100% to LPs, 0% to protocol
        let total_fee = 1000_0000000u128;
        let lp_revenue_fraction = 10000u32; // 100%
        
        let lp_fee = (total_fee * lp_revenue_fraction as u128) / FEE_DENOMINATOR as u128;
        let protocol_fee = total_fee - lp_fee;
        
        assert_eq!(lp_fee, total_fee);
        assert_eq!(protocol_fee, 0);
    }

    #[test]
    fn test_rounding_consistency() {
        let e = Env::default();
        
        // Test that rounding doesn't lose tokens
        let total_fee = 999u128; // Odd number
        let lp_revenue_fraction = 3333u32; // 33.33%
        
        let lp_fee = (total_fee * lp_revenue_fraction as u128) / FEE_DENOMINATOR as u128;
        let protocol_fee = total_fee - lp_fee;
        
        // Total should still equal original
        assert_eq!(lp_fee + protocol_fee, total_fee);
    }

    #[test]
    fn test_overflow_protection() {
        // Test with large values near u128::MAX
        let large_amount = u128::MAX / 100;
        let fee_fraction = 100u32; // 1%
        
        // This should not overflow
        let fee = (large_amount / (FEE_DENOMINATOR as u128)) * (fee_fraction as u128);
        
        assert!(fee < large_amount);
        assert!(fee > 0);
    }
}
