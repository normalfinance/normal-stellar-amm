#[cfg(test)]
mod tests {
    use soroban_sdk::{Env, Symbol, testutils::Address as _, Address, vec};
    use crate::{OracleRegistryClient, oracle::calculate_oracle_twap_price_spread_pct};
    use utils::state::oracle_registry::NormalAction;
    use crate::storage::put_historical_oracle_data;
    use crate::storage_types::HistoricalOracleData;
    use sep_40_oracle::testutils::{Asset, MockPriceOracleClient, MockPriceOracleWASM};

    #[test]
    #[should_panic(expected = "Error(Contract, #1)")]
    fn oracle_panics_on_price_uptick() {
        let e = Env::default();
        e.mock_all_auths();
        
        // Setup
        let admin = Address::generate(&e);
        let emergency_admin = Address::generate(&e);
        let registry = OracleRegistryClient::new(&e, &e.register(crate::OracleRegistry, ()));
        registry.initialize(&admin, &emergency_admin);
        
        // Create mock oracle
        let mock_oracle_id = e.register(MockPriceOracleWASM, ());
        let mock_oracle_client = MockPriceOracleClient::new(&e, &mock_oracle_id);
        
        // Set up asset
        let asset_symbol = Symbol::new(&e, "XLM");
        let asset_address = Address::generate(&e);
        
        // Register oracle with initial TWAP of 1.00
        let initial_twap = 1_000_000_u128; // 1.00 with 6 decimals
        registry.register_oracle(
            &admin,
            &asset_symbol,
            &asset_address,
            &mock_oracle_id,
            &18,
            &100 // sanitize_clamp_denominator
        );
        
        // Set historical data with TWAP = 1.00
        let historical_data = HistoricalOracleData {
            last_oracle_price_twap: initial_twap,
            last_oracle_price: initial_twap,
            last_oracle_delay: 0,
            last_oracle_price_twap_ts: e.ledger().timestamp(),
        };
        put_historical_oracle_data(&e, &asset_symbol, &historical_data);
        
        // Configure mock oracle to return higher price (1.02)
        let higher_price = 1_020_000_u128; // 1.02 with 6 decimals
        mock_oracle_client.set_data(
            &admin,
            &Asset::Stellar(asset_address.clone()),
            &vec![&e, Asset::Stellar(asset_address.clone())],
            &7,
            &300
        );
        mock_oracle_client.set_price_stable(&vec![&e, higher_price as i128]);
        
        // This call will panic because:
        // 1. Live price (1.02) is higher than TWAP (1.00)
        // 2. Due to parameter confusion, it calculates: TWAP - Live = 1.00 - 1.02
        // 3. safe_sub panics on underflow
        registry.get_price(&asset_symbol, &false, &NormalAction::Swap);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1)")]
    fn calculate_spread_panics_when_twap_less_than_live() {
        let e = Env::default();
        
        // Direct test of the vulnerable function
        // When called with swapped parameters (as happens in production):
        // - other_price = TWAP (1.00)
        // - last_oracle_price_twap = Live price (1.02)
        let twap_price = 1_000_000_u128;
        let live_price = 1_020_000_u128;
        
        // This will panic because twap_price < live_price
        calculate_oracle_twap_price_spread_pct(&e, twap_price, live_price);
    }
} 