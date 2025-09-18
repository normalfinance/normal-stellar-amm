#[cfg(test)]
mod tests {
    use soroban_sdk::Env;
    use crate::oracle::{calculate_oracle_twap_price_spread_pct, block_operation};
    use utils::temporal::Delay;
    use utils::state::oracle_registry::{NormalAction, OraclePriceData};

    #[test]
    #[should_panic(expected = "Error(Contract, #511)")]
    fn or01_price_uptick_causes_panic() {
        let e = Env::default();
        
        // Scenario: Market price increases from 1.00 to 1.02
        let twap = 1_000_000_u128;      // Historical TWAP: 1.00
        let live_price = 1_020_000_u128; // Current live price: 1.02
                
        let oracle_price_data = OraclePriceData {
            price: live_price,
            delay: Delay::from_timestamp_diff_expect(
                e.ledger().timestamp(),
                twap_ts,
                "Historical TWAP timestamp exceeds allowed clock drift tolerance",
            ),
        };
        
        // This is how block_operation is called in production (from get_price):
        // Note the parameter order - TWAP is passed as reserve_price (3rd param)
        // and live price is passed as last_oracle_price_twap (4th param)
        block_operation(
            &e,
            &oracle_price_data,
            twap,       // 3rd param: passed as reserve_price but it's actually TWAP
            live_price, // 4th param: passed as last_twap but it's actually live price
            NormalAction::Swap
        );
        
        // This panics because inside block_operation -> get_oracle_status -> 
        // calculate_oracle_twap_price_spread_pct, it tries to compute:
        // (twap as u64) - (live_price as u64) = 1.00 - 1.02 = UNDERFLOW!
    }

    #[test] 
    #[should_panic(expected = "Error(Contract, #511)")]
    fn direct_calculation_shows_underflow() {
        let e = Env::default();
        
        // Direct demonstration of the arithmetic underflow
        let twap = 1_000_000_u128;
        let live_price = 1_020_000_u128;
        
        // This is what happens inside the function with swapped parameters:
        // other_price = twap, last_oracle_price_twap = live_price
        calculate_oracle_twap_price_spread_pct(&e, twap, live_price);
        
        // The function tries: (twap as u64).safe_sub(e, live_price as u64)
        // Which is: 1_000_000 - 1_020_000 = PANIC!
    }

    #[test]
    fn normal_operation_when_price_decreases() {
        let e = Env::default();
        
        // When price decreases, no panic occurs
        let twap = 1_000_000_u128;      // TWAP: 1.00
        let live_price = 980_000_u128;   // Live: 0.98
        
        // With the buggy parameter order, this calculates:
        // twap - live = 1.00 - 0.98 = 0.02 (positive, no underflow)
        let spread = calculate_oracle_twap_price_spread_pct(&e, twap, live_price);
        
        // The calculation succeeds (even though the logic is backwards)
        assert!(spread > 0);
    }
}