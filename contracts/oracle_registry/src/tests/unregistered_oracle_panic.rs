#[cfg(test)]
mod tests {
    use soroban_sdk::{Env, Symbol, testutils::Address as _, Address};
    use crate::OracleRegistryClient;
    use utils::state::oracle_registry::NormalAction;

    #[test]
    #[should_panic(expected = "Error(Contract, #501)")]
    fn unregistered_oracle_panics() {
        let e = Env::default();
        e.mock_all_auths();
        
        // Minimal setup - just create registry and initialize
        let admin = Address::generate(&e);
        let emergency_admin = Address::generate(&e);
        let registry = OracleRegistryClient::new(&e, &e.register(crate::OracleRegistry, ()));
        registry.initialize(&admin, &emergency_admin);
        
        // Try to get price for unregistered asset - should panic
        let unregistered_asset = Symbol::new(&e, "FAKE");
        
        // This should panic with error 501 because oracle is not registered
        registry.get_price(&unregistered_asset, &false, &NormalAction::Swap);
    }
} 