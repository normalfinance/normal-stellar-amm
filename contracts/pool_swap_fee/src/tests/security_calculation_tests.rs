// Deep security and edge case tests for Pool Swap Fee calculations
#![cfg(test)]

use soroban_sdk::Env;
use soroban_fixed_point_math::FixedPoint;
use utils::constant::{FEE_DENOMINATOR, PRICE_PRECISION, THIRTY_DAY};
use utils::math::stats::calculate_rolling_sum;

mod fee_distribution_security_tests {
    use super::*;

    #[test]
    fn test_fee_split_manipulation_attack() {
        let e = Env::default();
        
        // Attack: manipulate lp_revenue_fraction to extract more fees
        let total_fee = 1000_0000000u128;
        
        // Normal 50% split
        let normal_lp_fraction = 5000u32; // 50%
        let normal_lp_fee = (total_fee * normal_lp_fraction as u128) / FEE_DENOMINATOR as u128;
        let normal_protocol_fee = total_fee - normal_lp_fee;
        
        // Attacker tries to set 100% to LPs
        let malicious_lp_fraction = 10000u32; // 100%
        let malicious_lp_fee = (total_fee * malicious_lp_fraction as u128) / FEE_DENOMINATOR as u128;
        let malicious_protocol_fee = total_fee - malicious_lp_fee;
        
        // Verify math works correctly
        assert_eq!(normal_lp_fee, 500_0000000);
        assert_eq!(normal_protocol_fee, 500_0000000);
        assert_eq!(malicious_lp_fee, 1000_0000000);
        assert_eq!(malicious_protocol_fee, 0);
        
        // Total should always equal original fee
        assert_eq!(normal_lp_fee + normal_protocol_fee, total_fee);
        assert_eq!(malicious_lp_fee + malicious_protocol_fee, total_fee);
    }

    #[test]
    fn test_fee_distribution_rounding_attacks() {
        let e = Env::default();
        
        // Attack: use odd fee amounts to exploit rounding
        let odd_fee = 999u128; // Odd number that doesn't divide evenly
        let lp_fraction = 3333u32; // 33.33%
        
        let lp_fee = (odd_fee * lp_fraction as u128) / FEE_DENOMINATOR as u128;
        let protocol_fee = odd_fee - lp_fee;
        
        // Verify no tokens are lost to rounding
        assert_eq!(lp_fee + protocol_fee, odd_fee);
        
        // LP fee should be rounded down
        let expected_lp_fee = (999u128 * 3333u128) / 10000u128; // = 332
        assert_eq!(lp_fee, expected_lp_fee);
        
        // Protocol gets the remainder
        assert_eq!(protocol_fee, odd_fee - expected_lp_fee);
    }

    #[test]
    fn test_buffer_fraction_edge_cases() {
        let e = Env::default();
        
        // Test with very small protocol fee
        let tiny_protocol_fee = 1u128;
        let buffer_fraction = 1000u32; // 10%
        
        let buffer_amount = (tiny_protocol_fee * buffer_fraction as u128) / FEE_DENOMINATOR as u128;
        let remaining = tiny_protocol_fee - buffer_amount;
        
        // Should round down to 0
        assert_eq!(buffer_amount, 0);
        assert_eq!(remaining, tiny_protocol_fee);
        
        // Test with exact buffer fraction
        let protocol_fee = 100_0000000u128;
        let buffer_amount_exact = (protocol_fee * buffer_fraction as u128) / FEE_DENOMINATOR as u128;
        
        assert_eq!(buffer_amount_exact, 10_0000000); // Exactly 10%
    }

    #[test]
    fn test_fee_calculation_overflow_protection() {
        // Test with maximum safe values
        let max_safe_amount = u128::MAX / FEE_DENOMINATOR as u128;
        let max_fee_fraction = FEE_DENOMINATOR; // 100%
        
        // This should not overflow
        let fee = (max_safe_amount * max_fee_fraction as u128) / FEE_DENOMINATOR as u128;
        assert_eq!(fee, max_safe_amount);
        
        // Test near-overflow scenario
        let near_max = u128::MAX / 2;
        let small_fraction = 1u32; // 0.01%
        
        let small_fee = (near_max * small_fraction as u128) / FEE_DENOMINATOR as u128;
        assert!(small_fee > 0);
        assert!(small_fee < near_max);
    }

    #[test]
    fn test_dynamic_fee_split_calculation() {
        let e = Env::default();
        
        // Test dynamic LP fee calculation based on liquidity ownership
        let reserve_a = 2000_0000000u128; // Protocol synthetic
        let reserve_b = 1000_0000000u128; // LP deposits
        let base_price = 1_0000000u128;
        let quote_price = 1_0000000u128;
        
        // Calculate actual liquidity values
        let lp_liquidity_value = reserve_b * quote_price;
        let protocol_liquidity_value = reserve_a * base_price;
        let total_liquidity = lp_liquidity_value + protocol_liquidity_value;
        
        // Dynamic LP fraction based on actual contribution
        let dynamic_lp_fraction = (lp_liquidity_value * FEE_DENOMINATOR as u128) / total_liquidity;
        
        // In this case: 1000 / (1000 + 2000) = 33.33%
        assert_eq!(dynamic_lp_fraction, 3333); // 33.33%
        
        // Apply to fee distribution
        let total_fee = 300_0000000u128;
        let dynamic_lp_fee = (total_fee * dynamic_lp_fraction) / FEE_DENOMINATOR as u128;
        let dynamic_protocol_fee = total_fee - dynamic_lp_fee;
        
        assert_eq!(dynamic_lp_fee, 99_9900000); // ~33.33%
        assert_eq!(dynamic_protocol_fee, 200_0100000); // ~66.67%
        
        // Verify this is more fair than static 50/50 split
        let static_lp_fee = total_fee / 2; // 150
        assert!(dynamic_lp_fee < static_lp_fee); // LPs get less because they own less
    }

    #[test]
    fn test_fee_distribution_gaming_scenarios() {
        let e = Env::default();
        
        // Scenario 1: Attacker tries to game timing of fee distribution
        let total_fee = 1000_0000000u128;
        let lp_fraction = 5000u32;
        
        // Multiple small distributions vs one large
        let mut total_lp_fees = 0u128;
        let mut total_protocol_fees = 0u128;
        
        // 10 distributions of 100 each
        for _ in 0..10 {
            let small_fee = total_fee / 10;
            let lp_fee = (small_fee * lp_fraction as u128) / FEE_DENOMINATOR as u128;
            let protocol_fee = small_fee - lp_fee;
            
            total_lp_fees += lp_fee;
            total_protocol_fees += protocol_fee;
        }
        
        // Single large distribution
        let single_lp_fee = (total_fee * lp_fraction as u128) / FEE_DENOMINATOR as u128;
        let single_protocol_fee = total_fee - single_lp_fee;
        
        // Should be equivalent (no gaming advantage)
        assert_eq!(total_lp_fees, single_lp_fee);
        assert_eq!(total_protocol_fees, single_protocol_fee);
    }
}

mod insurance_premium_security_tests {
    use super::*;

    #[test]
    fn test_volume_manipulation_attack() {
        let e = Env::default();
        
        // Scenario: Attacker manipulates 30-day volume to reduce premiums
        let normal_volume_30d = 1_000_000_0000000u128; // $1M normal volume
        let insurance_coverage = 100_000_0000000u128; // $100k coverage
        let insurance_rate = 500i32; // 5% annual rate
        
        // Normal premium calculation
        let normal_annual_volume = normal_volume_30d.fixed_mul_floor(&e, &365, &30).unwrap();
        let annual_premium = insurance_coverage.fixed_mul_floor(&e, &(insurance_rate as u128), &PRICE_PRECISION).unwrap();
        let normal_premium_per_dollar = annual_premium / normal_annual_volume;
        
        // Attack: Inflate volume temporarily to reduce premium rate
        let inflated_volume_30d = normal_volume_30d * 100; // 100x volume spike
        let inflated_annual_volume = inflated_volume_30d.fixed_mul_floor(&e, &365, &30).unwrap();
        let inflated_premium_per_dollar = annual_premium / inflated_annual_volume;
        
        // Attacker benefits from much lower premium rate
        assert_eq!(inflated_premium_per_dollar, normal_premium_per_dollar / 100);
        
        // For a $1000 swap:
        let swap_amount = 1000_0000000u128;
        let normal_premium = swap_amount * normal_premium_per_dollar;
        let inflated_premium = swap_amount * inflated_premium_per_dollar;
        
        assert_eq!(inflated_premium, normal_premium / 100); // 100x less premium
    }

    #[test]
    fn test_premium_calculation_edge_cases() {
        let e = Env::default();
        
        // Edge case 1: Zero volume
        let zero_volume = 0u128;
        let insurance_coverage = 100_000_0000000u128;
        let insurance_rate = 500i32;
        
        let annual_premium = insurance_coverage.fixed_mul_floor(&e, &(insurance_rate as u128), &PRICE_PRECISION).unwrap();
        
        // Division by zero protection
        let premium_per_dollar = if zero_volume == 0 {
            u128::MAX // Infinite premium rate
        } else {
            annual_premium / zero_volume
        };
        
        assert_eq!(premium_per_dollar, u128::MAX);
        
        // Edge case 2: Tiny volume
        let tiny_volume = 1u128;
        let tiny_annual = tiny_volume.fixed_mul_floor(&e, &365, &30).unwrap();
        let tiny_premium_per_dollar = annual_premium / tiny_annual.max(1);
        
        // Should be very high premium rate
        assert!(tiny_premium_per_dollar > 1000_0000000);
        
        // Edge case 3: Massive volume
        let massive_volume = u128::MAX / 1000; // Avoid overflow
        let massive_annual = massive_volume / 365 * 30; // Simplified calculation
        let massive_premium_per_dollar = annual_premium / massive_annual.max(1);
        
        // Should be tiny premium rate
        assert!(massive_premium_per_dollar < 1000);
    }

    #[test]
    fn test_premium_capping_mechanism() {
        let e = Env::default();
        
        // Test premium capped by protocol fee
        let protocol_fee = 10_0000000u128; // Small protocol fee
        let calculated_premium = 50_0000000u128; // Large calculated premium
        
        let actual_premium = calculated_premium.min(protocol_fee);
        assert_eq!(actual_premium, protocol_fee); // Should be capped
        
        // Test when premium is less than protocol fee
        let small_premium = 5_0000000u128;
        let uncapped_premium = small_premium.min(protocol_fee);
        assert_eq!(uncapped_premium, small_premium); // Should not be capped
    }

    #[test]
    fn test_time_based_premium_alternative() {
        let e = Env::default();
        
        // Alternative: Time-based premium accrual instead of volume-based
        let insurance_coverage = 100_000_0000000u128;
        let annual_rate = 500u128; // 5%
        let seconds_per_year = 365 * 24 * 60 * 60u64;
        
        // Premium accrued per second
        let annual_premium = insurance_coverage * annual_rate / PRICE_PRECISION;
        let premium_per_second = annual_premium / (seconds_per_year as u128);
        
        // For a 1-hour period
        let one_hour = 3600u64;
        let hourly_premium = premium_per_second * (one_hour as u128);
        
        // Should be much more stable than volume-based
        assert_eq!(hourly_premium, annual_premium / (seconds_per_year as u128) * (one_hour as u128));
        
        // Test different time periods
        let daily_premium = premium_per_second * (24 * 3600u128);
        let monthly_premium = premium_per_second * (30 * 24 * 3600u128);
        
        assert_eq!(daily_premium * 30, monthly_premium);
        assert_eq!(monthly_premium * 12, annual_premium);
    }

    #[test]
    fn test_premium_volatility_smoothing() {
        let e = Env::default();
        
        // Test smoothing mechanism for volume spikes
        let base_volume = 1_000_000_0000000u128;
        let insurance_coverage = 100_000_0000000u128;
        let insurance_rate = 500i32;
        
        // Simulate volume history with spikes
        let volumes = [
            base_volume,      // Day 1: Normal
            base_volume * 10, // Day 2: 10x spike
            base_volume,      // Day 3: Back to normal
            base_volume / 2,  // Day 4: Low volume
            base_volume * 5,  // Day 5: 5x spike
        ];
        
        // Calculate smoothed volume (simple moving average)
        let smoothed_volume = volumes.iter().sum::<u128>() / volumes.len() as u128;
        
        // Compare with raw volume approaches
        let annual_premium = insurance_coverage.fixed_mul_floor(&e, &(insurance_rate as u128), &PRICE_PRECISION).unwrap();
        
        // Using smoothed volume
        let smoothed_annual = smoothed_volume.fixed_mul_floor(&e, &365, &30).unwrap();
        let smoothed_premium_per_dollar = annual_premium / smoothed_annual;
        
        // Using spike volume (worst case for protocol)
        let spike_annual = (base_volume * 10).fixed_mul_floor(&e, &365, &30).unwrap();
        let spike_premium_per_dollar = annual_premium / spike_annual;
        
        // Smoothed should be between normal and spike rates
        let normal_annual = base_volume.fixed_mul_floor(&e, &365, &30).unwrap();
        let normal_premium_per_dollar = annual_premium / normal_annual;
        
        assert!(smoothed_premium_per_dollar < normal_premium_per_dollar);
        assert!(smoothed_premium_per_dollar > spike_premium_per_dollar);
    }

    #[test]
    fn test_premium_economic_attacks() {
        let e = Env::default();
        
        // Attack 1: Sandwich attack on volume calculation
        let insurance_coverage = 100_000_0000000u128;
        let insurance_rate = 500i32;
        let annual_premium = insurance_coverage.fixed_mul_floor(&e, &(insurance_rate as u128), &PRICE_PRECISION).unwrap();
        
        // Before attack: Normal volume
        let normal_volume = 1_000_000_0000000u128;
        let normal_annual = normal_volume.fixed_mul_floor(&e, &365, &30).unwrap();
        let normal_rate = annual_premium / normal_annual;
        
        // During attack: Inflate volume
        let attack_volume = normal_volume * 1000; // 1000x inflation
        let attack_annual = attack_volume.fixed_mul_floor(&e, &365, &30).unwrap();
        let attack_rate = annual_premium / attack_annual;
        
        // Attacker gets 1000x lower premium rate
        assert_eq!(attack_rate, normal_rate / 1000);
        
        // Attack 2: Coordinated low-volume periods
        let low_volume = normal_volume / 100; // 1% of normal
        let low_annual = low_volume.fixed_mul_floor(&e, &365, &30).unwrap();
        let low_rate = annual_premium / low_annual;
        
        // Premium rate becomes 100x higher during low volume
        assert_eq!(low_rate, normal_rate * 100);
        
        // This creates unfair burden on users during low-volume periods
    }
}

mod rolling_volume_security_tests {
    use super::*;

    #[test]
    fn test_rolling_sum_manipulation() {
        let e = Env::default();
        
        // Test manipulation of rolling sum calculation
        let old_sum = 1_000_000_0000000u128; // $1M accumulated
        let period = THIRTY_DAY;
        
        // Normal update
        let normal_value = 100_000_0000000u128; // $100k new volume
        let normal_since_last = 86400u64; // 1 day
        let normal_new_sum = calculate_rolling_sum(&e, old_sum, normal_value, normal_since_last, period);
        
        // Attack: Add huge volume right before period expires
        let attack_value = 10_000_000_0000000u128; // $10M attack volume
        let attack_since_last = period - 1; // 1 second before expiry
        let attack_new_sum = calculate_rolling_sum(&e, old_sum, attack_value, attack_since_last, period);
        
        // Attacker's volume gets almost no decay
        let decay_factor = 1u128 * PRICE_PRECISION / period as u128; // Almost 0 decay
        let expected_attack_sum = attack_value + (old_sum * decay_factor / PRICE_PRECISION);
        
        // Attack sum should be much higher
        assert!(attack_new_sum > normal_new_sum * 5);
        
        // But after full period, attack volume should decay away
        let post_attack_sum = calculate_rolling_sum(&e, attack_new_sum, 0, period, period);
        assert_eq!(post_attack_sum, 0); // All volume decayed
    }

    #[test]
    fn test_rolling_sum_precision_attacks() {
        let e = Env::default();
        
        // Attack: Use tiny values to exploit precision
        let tiny_old_sum = 1u128;
        let tiny_new_value = 1u128;
        let period = THIRTY_DAY;
        let since_last = period / 2; // Half period
        
        let result = calculate_rolling_sum(&e, tiny_old_sum, tiny_new_value, since_last, period);
        
        // Should handle tiny values gracefully
        assert!(result >= tiny_new_value); // At least the new value
        assert!(result <= tiny_old_sum + tiny_new_value); // Not more than sum
        
        // Test with maximum values
        let max_old_sum = u128::MAX / 2;
        let max_new_value = u128::MAX / 2;
        let max_result = calculate_rolling_sum(&e, max_old_sum, max_new_value, since_last, period);
        
        // Should not overflow
        assert!(max_result < u128::MAX);
        assert!(max_result > max_new_value);
    }

    #[test]
    fn test_rolling_sum_time_manipulation() {
        let e = Env::default();
        
        let old_sum = 1_000_000_0000000u128;
        let new_value = 100_000_0000000u128;
        let period = THIRTY_DAY;
        
        // Test edge cases of time manipulation
        
        // Case 1: No time passed (since_last = 0)
        let no_time_sum = calculate_rolling_sum(&e, old_sum, new_value, 0, period);
        assert_eq!(no_time_sum, old_sum + new_value); // Simple addition
        
        // Case 2: Exactly one period passed
        let full_period_sum = calculate_rolling_sum(&e, old_sum, new_value, period, period);
        assert_eq!(full_period_sum, new_value); // Old sum fully decayed
        
        // Case 3: More than one period passed
        let over_period_sum = calculate_rolling_sum(&e, old_sum, new_value, period * 2, period);
        assert_eq!(over_period_sum, new_value); // Old sum fully decayed
        
        // Case 4: Very long time (potential overflow)
        let very_long_time = u64::MAX / 2;
        let long_time_sum = calculate_rolling_sum(&e, old_sum, new_value, very_long_time, period);
        assert_eq!(long_time_sum, new_value); // Old sum fully decayed
    }

    #[test]
    fn test_volume_extrapolation_attacks() {
        let e = Env::default();
        
        // Test annual volume extrapolation manipulation
        let volume_30d = 1_000_000_0000000u128;
        
        // Normal extrapolation
        let normal_annual = volume_30d.fixed_mul_floor(&e, &365, &30).unwrap();
        let expected_annual = volume_30d * 365 / 30; // ~12.17x
        assert_eq!(normal_annual, expected_annual);
        
        // Attack: Manipulate the 30-day window
        // If attacker can control exactly what counts as "30 days"
        
        // Scenario 1: Front-load volume in the 30-day window
        let frontloaded_30d = volume_30d * 5; // 5x volume in 30 days
        let frontloaded_annual = frontloaded_30d.fixed_mul_floor(&e, &365, &30).unwrap();
        
        // This extrapolates to 5x higher annual volume
        assert_eq!(frontloaded_annual, expected_annual * 5);
        
        // Scenario 2: Use period with artificially low volume
        let low_30d = volume_30d / 10; // 10x lower volume
        let low_annual = low_30d.fixed_mul_floor(&e, &365, &30).unwrap();
        
        // This extrapolates to 10x lower annual volume
        assert_eq!(low_annual, expected_annual / 10);
        
        // This creates 50x difference in premium rates between scenarios
        let coverage = 100_000_0000000u128;
        let rate = 500u128;
        let annual_premium = coverage * rate / PRICE_PRECISION;
        
        let frontloaded_premium_rate = annual_premium / frontloaded_annual;
        let low_premium_rate = annual_premium / low_annual;
        
        assert_eq!(low_premium_rate, frontloaded_premium_rate * 50);
    }
}
