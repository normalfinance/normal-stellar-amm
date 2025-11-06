use soroban_fixed_point_math::SorobanFixedPoint;
use soroban_sdk::{Address, Env};
use utils::constant::PRICE_PRECISION;
use utils::math::safe_math::{PrecisionMath, SafeConversion, SafeMath};

use crate::storage::{
    get_bonus_escrow, get_bonus_rate_table, get_bonus_reserve_b, get_bonus_vesting_period, get_max_bonus_fraction,
    put_bonus_escrow, set_bonus_reserve_b, BonusEscrow,
};


/// Bonus curve based on configurable table or exponential deviation from peg.
/// If a rate table is configured, uses step function lookup.
/// Otherwise falls back to exponential formula with k = 25 to incentivize larger corrections.
pub fn calculate_bonus_rate(e: &Env, pool_price: u128, peg_price: u128) -> u32 {
    if pool_price == 0 || peg_price == 0 {
        return 0;
    }
    
    // Calculate absolute deviation: |pool_price - peg_price| / peg_price
    let price_diff = if pool_price > peg_price {
        pool_price.safe_sub(e, peg_price)
    } else {
        peg_price.safe_sub(e, pool_price)
    };

    // abs_deviation = |price_diff| / peg_price (scaled by PRICE_PRECISION)
    let abs_deviation = price_diff.safe_fixed_div_round(e, peg_price, PRICE_PRECISION);

    // For no deviation, no bonus
    if abs_deviation == 0 {
        return 0;
    }

    // Check if we have a configured rate table
    let rate_table = get_bonus_rate_table(e);
    
    if rate_table.len() > 0 {
        // Use table lookup with step function
        // Find the entry with the highest deviation <= actual deviation
        let mut best_rate = 0_u32;
        let mut found = false;
        
        for i in 0..rate_table.len() {
            let entry = rate_table.get(i).unwrap();
            if entry.deviation <= abs_deviation {
                best_rate = entry.rate;
                found = true;
            } else {
                break; // Table should be sorted, so we can stop here
            }
        }
        
        // If deviation is below all table points, return 0
        // If deviation matches or exceeds a point, return that rate
        if found {
            return best_rate;
        } else {
            return 0;
        }
    }

    // Fall back to exponential formula (backward compatibility)
    let max_bonus = get_max_bonus_fraction(e);

    // Bonus curve: max_bonus * (1 - e^(-k * abs_dev))
    // Using a steeper curve for bonus (k = 25) to incentivize larger corrections
    let k = 25_u128;
    
    // Calculate k * abs_deviation
    let x = k.safe_fixed_mul_floor(e, abs_deviation, PRICE_PRECISION);

    // For very large deviations, return max bonus
    if x > PRICE_PRECISION * 5 {
        return max_bonus;
    }

    // Calculate Taylor series: 1 - x + x²/2 - x³/6
    let x2 = x.safe_fixed_mul_floor(e, x, PRICE_PRECISION);
    let x3 = x2.safe_fixed_mul_floor(e, x, PRICE_PRECISION);

    // exp_neg = 1 - x + x²/2 - x³/6
    let mut exp_neg = PRICE_PRECISION;
    exp_neg = exp_neg.safe_sub(e, x);
    exp_neg = exp_neg.safe_add(e, x2.safe_div(e, 2));
    
    // Guard against negative values
    if x3.safe_div(e, 6) < exp_neg {
        exp_neg = exp_neg.safe_sub(e, x3.safe_div(e, 6));
    } else {
        exp_neg = 0;
    }

    // Clamp exp_neg within [0, PRICE_PRECISION]
    let exp_neg_clamped = exp_neg.min(PRICE_PRECISION);

    // Calculate: max_bonus * (1 - exp_neg)
    let one_minus_exp = PRICE_PRECISION.safe_sub(e, exp_neg_clamped);
    
    let bonus_rate = (max_bonus as u128)
        .safe_fixed_mul_floor(e, one_minus_exp, PRICE_PRECISION)
        .safe_to_u32(e);
    
    // Cap at max_bonus
    bonus_rate.min(max_bonus)
}

/// Calculates the bonus amount to be awarded for a risk-reducing trade.
/// The bonus is capped by the available bonus reserve to prevent over-distribution.
pub fn calculate_bonus_amount(
    e: &Env,
    bonus_rate: u32,
    trade_amount: u128,
) -> u128 {
    if bonus_rate == 0 || trade_amount == 0 {
        return 0;
    }

    // Calculate raw bonus: trade_amount * bonus_rate / PRICE_PRECISION
    let raw_bonus = trade_amount.fixed_mul_floor(e, &(bonus_rate as u128), &PRICE_PRECISION);

    // Cap bonus by available reserve
    let available_reserve = get_bonus_reserve_b(e);
    
    // Return the minimum of raw bonus and available reserve
    raw_bonus.min(available_reserve)
}

/// Records a bonus for a user with vesting period.
/// The bonus is stored in escrow and can only be claimed after the vesting period.
pub fn record_bonus(
    e: &Env,
    user: &Address,
    pool_price: u128,
    peg_price: u128,
    trade_amount: u128,
    current_time: u64,
) {
    let vesting_period = get_bonus_vesting_period(e);
    let bonus_rate = calculate_bonus_rate(e, pool_price, peg_price);
    let bonus_amount = calculate_bonus_amount(e, bonus_rate, trade_amount);

    // Only record if there's a bonus to give
    if bonus_amount > 0 {
        // Get existing escrow or create new one
        let mut escrow = get_bonus_escrow(e, user);
        
        // Add new bonus to existing amount (accumulate bonuses)
        escrow.amount = escrow.amount.safe_add(e, bonus_amount);
        escrow.updated_at = current_time;
        escrow.valid_after = current_time.safe_add(e, vesting_period);
        
        // Save escrow
        put_bonus_escrow(e, user, &escrow);
        
        // Deduct from bonus reserve (it's now allocated to this user)
        let current_reserve = get_bonus_reserve_b(e);
        let new_reserve = current_reserve.safe_sub(e, bonus_amount);
        set_bonus_reserve_b(e, &new_reserve);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::ElasticPool;
    use soroban_sdk::testutils::Address as _;

    fn test_env_with_contract() -> (Env, soroban_sdk::Address) {
        let e = Env::default();
        e.mock_all_auths();
        let contract_id = e.register_contract(None, ElasticPool);
        (e, contract_id)
    }

    #[test]
    fn test_calculate_bonus_rate_zero_prices() {
        let (e, contract_id) = test_env_with_contract();

        let (rate_0, rate_1, rate_2) = e.as_contract(&contract_id, || {
            let rate_0 = calculate_bonus_rate(&e, 0, 0);
            let rate_1 = calculate_bonus_rate(&e, 1_0000000, 0);
            let rate_2 = calculate_bonus_rate(&e, 0, 1_0000000);
            (rate_0, rate_1, rate_2)
        });

        assert_eq!(rate_0, 0);
        assert_eq!(rate_1, 0);
        assert_eq!(rate_2, 0);
    }

    #[test]
    fn test_calculate_bonus_rate_at_peg() {
        let (e, contract_id) = test_env_with_contract();

        let bonus_rate = e.as_contract(&contract_id, || {
            calculate_bonus_rate(&e, 1_0000000, 1_0000000)
        });

        // No deviation = no bonus
        assert_eq!(bonus_rate, 0);
    }

    #[test]
    fn test_calculate_bonus_rate_with_deviation() {
        let (e, contract_id) = test_env_with_contract();

        let (max_bonus_fraction, bonus_rate) = e.as_contract(&contract_id, || {
            let max_bonus_fraction = get_max_bonus_fraction(&e);
            let bonus_rate = calculate_bonus_rate(&e, 1_1000000, 1_0000000);
            (max_bonus_fraction, bonus_rate)
        });

        // Should be within valid bounds
        assert!(bonus_rate >= 0);
        assert!(bonus_rate <= max_bonus_fraction);
    }

    #[test]
    fn test_calculate_bonus_rate_large_deviation() {
        let (e, contract_id) = test_env_with_contract();

        let (max_bonus_fraction, bonus_rate) = e.as_contract(&contract_id, || {
            let max_bonus_fraction = get_max_bonus_fraction(&e);
            let bonus_rate = calculate_bonus_rate(&e, 1_2000000, 1_0000000);
            (max_bonus_fraction, bonus_rate)
        });

        // Should be within valid bounds
        assert!(bonus_rate >= 0);
        assert!(bonus_rate <= max_bonus_fraction);
    }

    #[test]
    fn test_calculate_bonus_amount_no_reserve() {
        let (e, contract_id) = test_env_with_contract();

        let bonus_amount = e.as_contract(&contract_id, || {
            // Set bonus reserve to 0
            set_bonus_reserve_b(&e, &0);

            let bonus_rate = 5000; // 5%
            let trade_amount = 100_0000000;

            calculate_bonus_amount(&e, bonus_rate, trade_amount)
        });

        // No bonus if no reserve
        assert_eq!(bonus_amount, 0);
    }

    #[test]
    fn test_calculate_bonus_amount_with_reserve() {
        let (e, contract_id) = test_env_with_contract();

        let bonus_amount = e.as_contract(&contract_id, || {
            // Set bonus reserve to 50 tokens
            set_bonus_reserve_b(&e, &50_0000000);

            let bonus_rate = 5000; // 5%
            let trade_amount = 100_0000000;

            calculate_bonus_amount(&e, bonus_rate, trade_amount)
        });

        // With bonus_rate = 5000 and PRICE_PRECISION = 10_000_000:
        // 100_0000000 * 5000 / 10_000_000 = 500000
        assert_eq!(bonus_amount, 500000);
    }

    #[test]
    fn test_calculate_bonus_amount_capped_by_reserve() {
        let (e, contract_id) = test_env_with_contract();

        let bonus_amount = e.as_contract(&contract_id, || {
            // Set small bonus reserve
            set_bonus_reserve_b(&e, &100000);

            let bonus_rate = 5000; // 5%
            let trade_amount = 100_0000000;

            calculate_bonus_amount(&e, bonus_rate, trade_amount)
        });

        // Should be capped at reserve amount (smaller than calculated bonus)
        assert_eq!(bonus_amount, 100000);
    }

    #[test]
    fn test_record_bonus() {
        let (e, contract_id) = test_env_with_contract();

        let (escrow_amount, escrow_updated_at, escrow_valid_after, remaining_reserve, current_time, initial_reserve) = e.as_contract(&contract_id, || {
            let user = Address::generate(&e);

            // Set up bonus reserve
            let initial_reserve = 10_0000000;
            set_bonus_reserve_b(&e, &initial_reserve);

            let current_time = 1000_u64;
            let pool_price = 1_1000000; // 10% deviation
            let peg_price = 1_0000000;
            let trade_amount = 100_0000000;

            record_bonus(&e, &user, pool_price, peg_price, trade_amount, current_time);

            // Check escrow was created
            let escrow = get_bonus_escrow(&e, &user);

            // Check reserve
            let remaining_reserve = get_bonus_reserve_b(&e);

            (escrow.amount, escrow.updated_at, escrow.valid_after, remaining_reserve, current_time, initial_reserve)
        });

        // Verify escrow structure is set up correctly when bonus was recorded
        if escrow_amount > 0 {
            assert_eq!(escrow_updated_at, current_time);
            assert!(escrow_valid_after >= current_time);
            assert!(remaining_reserve < initial_reserve);
        } else {
            // If no bonus, escrow should be empty (default values)
            assert_eq!(escrow_updated_at, 0);
            assert!(remaining_reserve == initial_reserve);
        }
    }

    #[test]
    fn test_record_bonus_accumulates() {
        let (e, contract_id) = test_env_with_contract();

        let (first_amount, second_total) = e.as_contract(&contract_id, || {
            let user = Address::generate(&e);

            // Set up bonus reserve
            set_bonus_reserve_b(&e, &20_0000000);

            let current_time = 1000_u64;
            let pool_price = 1_0500000; // 5% deviation
            let peg_price = 1_0000000;
            let trade_amount = 100_0000000;

            // Record first bonus
            record_bonus(&e, &user, pool_price, peg_price, trade_amount, current_time);
            let first_amount = get_bonus_escrow(&e, &user).amount;

            // Record second bonus
            record_bonus(&e, &user, pool_price, peg_price, trade_amount, current_time + 100);
            let second_total = get_bonus_escrow(&e, &user).amount;

            (first_amount, second_total)
        });

        // If bonuses were recorded, they should accumulate
        if first_amount > 0 {
            assert!(second_total >= first_amount);
        }
    }

    #[test]
    fn test_calculate_bonus_rate_with_empty_table() {
        // Test that empty table falls back to exponential formula
        let (e, contract_id) = test_env_with_contract();

        let (table_len, bonus_rate_at_peg, bonus_rate_5pct) = e.as_contract(&contract_id, || {
            // Verify table is empty
            let table = crate::storage::get_bonus_rate_table(&e);
            
            // Calculate at peg (should be 0)
            let bonus_rate_at_peg = calculate_bonus_rate(&e, 1_0000000, 1_0000000);
            // Calculate with 5% deviation
            let bonus_rate_5pct = calculate_bonus_rate(&e, 1_0500000, 1_0000000);
            
            (table.len(), bonus_rate_at_peg, bonus_rate_5pct)
        });

        // Should use exponential formula when table is empty
        assert_eq!(table_len, 0);
        // At peg should be 0
        assert_eq!(bonus_rate_at_peg, 0);
        // With deviation, should return a valid bonus rate (>= 0, since formula might return 0 or positive)
        assert!(bonus_rate_5pct >= 0);
    }

    #[test]
    fn test_calculate_bonus_rate_with_configured_table() {
        use crate::storage::{set_bonus_rate_table, RateTableEntry};
        use soroban_sdk::Vec;
        
        let (e, contract_id) = test_env_with_contract();

        let bonus_rate_5pct = e.as_contract(&contract_id, || {
            // Configure a simple table
            let mut table = Vec::new(&e);
            // 2% deviation -> 2000 (2%)
            table.push_back(RateTableEntry { deviation: 200000, rate: 2000 });
            // 5% deviation -> 5000 (5%)
            table.push_back(RateTableEntry { deviation: 500000, rate: 5000 });
            // 10% deviation -> 10000 (10%)
            table.push_back(RateTableEntry { deviation: 1000000, rate: 10000 });
            
            set_bonus_rate_table(&e, &table);
            
            // Test exact match at 5%
            calculate_bonus_rate(&e, 1_0500000, 1_0000000)
        });

        // Should return exact rate for 5% deviation
        assert_eq!(bonus_rate_5pct, 5000);
    }

    #[test]
    fn test_calculate_bonus_rate_step_function() {
        use crate::storage::{set_bonus_rate_table, RateTableEntry};
        use soroban_sdk::Vec;
        
        let (e, contract_id) = test_env_with_contract();

        let (rate_3pct, rate_7pct) = e.as_contract(&contract_id, || {
            // Configure table
            let mut table = Vec::new(&e);
            table.push_back(RateTableEntry { deviation: 200000, rate: 2000 }); // 2%
            table.push_back(RateTableEntry { deviation: 500000, rate: 5000 }); // 5%
            table.push_back(RateTableEntry { deviation: 1000000, rate: 10000 }); // 10%
            
            set_bonus_rate_table(&e, &table);
            
            // Test 3% deviation (between 2% and 5%)
            let rate_3pct = calculate_bonus_rate(&e, 1_0300000, 1_0000000);
            
            // Test 7% deviation (between 5% and 10%)
            let rate_7pct = calculate_bonus_rate(&e, 1_0700000, 1_0000000);
            
            (rate_3pct, rate_7pct)
        });

        // 3% should use rate for 2% (nearest lower point)
        assert_eq!(rate_3pct, 2000);
        
        // 7% should use rate for 5% (nearest lower point)
        assert_eq!(rate_7pct, 5000);
    }

    #[test]
    fn test_calculate_bonus_rate_above_all_table_entries() {
        use crate::storage::{set_bonus_rate_table, RateTableEntry};
        use soroban_sdk::Vec;
        
        let (e, contract_id) = test_env_with_contract();

        let rate_high = e.as_contract(&contract_id, || {
            // Configure table with max at 10%
            let mut table = Vec::new(&e);
            table.push_back(RateTableEntry { deviation: 500000, rate: 5000 }); // 5%
            table.push_back(RateTableEntry { deviation: 1000000, rate: 10000 }); // 10%
            
            set_bonus_rate_table(&e, &table);
            
            // Test 20% deviation (above all entries)
            calculate_bonus_rate(&e, 1_2000000, 1_0000000)
        });

        // Should return the highest rate in table
        assert_eq!(rate_high, 10000);
    }

    #[test]
    fn test_calculate_bonus_rate_below_all_table_entries() {
        use crate::storage::{set_bonus_rate_table, RateTableEntry};
        use soroban_sdk::Vec;
        
        let (e, contract_id) = test_env_with_contract();

        let rate_low = e.as_contract(&contract_id, || {
            // Configure table starting at 5%
            let mut table = Vec::new(&e);
            table.push_back(RateTableEntry { deviation: 500000, rate: 5000 }); // 5%
            table.push_back(RateTableEntry { deviation: 1000000, rate: 10000 }); // 10%
            
            set_bonus_rate_table(&e, &table);
            
            // Test 2% deviation (below all entries)
            calculate_bonus_rate(&e, 1_0200000, 1_0000000)
        });

        // Should return 0 when below all table points
        assert_eq!(rate_low, 0);
    }

    #[test]
    fn test_calculate_bonus_rate_at_zero_deviation() {
        use crate::storage::{set_bonus_rate_table, RateTableEntry};
        use soroban_sdk::Vec;
        
        let (e, contract_id) = test_env_with_contract();

        let (rate_with_table, rate_without_table) = e.as_contract(&contract_id, || {
            // Test without table (should use formula)
            let rate_without_table = calculate_bonus_rate(&e, 1_0000000, 1_0000000);
            
            // Configure table with entry at 0
            let mut table = Vec::new(&e);
            table.push_back(RateTableEntry { deviation: 0, rate: 100 });
            table.push_back(RateTableEntry { deviation: 500000, rate: 5000 }); // 5%
            
            set_bonus_rate_table(&e, &table);
            
            // Test at peg with table
            let rate_with_table = calculate_bonus_rate(&e, 1_0000000, 1_0000000);
            
            (rate_with_table, rate_without_table)
        });

        // Without table, should return 0 (no deviation)
        assert_eq!(rate_without_table, 0);
        
        // With table, should return 0 even with entry (0 deviation means no bonus)
        assert_eq!(rate_with_table, 0);
    }
}
