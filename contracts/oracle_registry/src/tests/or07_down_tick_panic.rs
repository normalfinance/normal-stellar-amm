#[cfg(test)]
mod tests {
    use soroban_sdk::Env;
    use utils::math::pool::sanitize_new_price;

    #[test]
    #[should_panic(expected = "Error(Contract, #511)")]
    fn oracle_panics_on_price_down_tick() {
        let e = Env::default();

        let last_twap = 1_000_000_u128;  // 1.00
        let new_price = 980_000_u128;    // 0.98 (price went down)

        // sanitize_clamp_denominator = 100 (any non-zero value works)
        // This will call new_price.safe_sub(last_twap) which under-flows and panics
        let _ = sanitize_new_price(&e, new_price, last_twap, 100);
    }
} 