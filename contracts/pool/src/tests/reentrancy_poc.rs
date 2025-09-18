#[cfg(test)]
mod reentrancy_tests {
    use soroban_sdk::{contract, contractimpl, contracttype, testutils::Address as _, Address, Env, IntoVal, Symbol, Vec};

    /********************** MiniPool *************************/
    #[contract]
    pub struct MiniPool;

    #[derive(Clone)]
    #[contracttype]
    enum PoolKey {
        TokenB,   // Address
        ReserveB, // u128
    }

    fn set_token_b(e: &Env, addr: &Address) {
        e.storage().instance().set(&PoolKey::TokenB, addr);
    }

    fn get_token_b(e: &Env) -> Address {
        e.storage()
            .instance()
            .get::<_, Address>(&PoolKey::TokenB)
            .expect("token not set")
    }

    fn get_reserve_b(e: &Env) -> u128 {
        e.storage()
            .instance()
            .get::<_, u128>(&PoolKey::ReserveB)
            .unwrap_or(0)
    }

    fn set_reserve_b(e: &Env, v: u128) {
        e.storage().instance().set(&PoolKey::ReserveB, &v);
    }

    #[contractimpl]
    impl MiniPool {
        /// Initialize with the malicious token B address.
        pub fn init(e: Env, token_b: Address) {
            set_token_b(&e, &token_b);
            set_reserve_b(&e, 0u128);
        }

        /// Vulnerable deposit: reads reserve, performs external call, then writes
        pub fn deposit(e: Env, user: Address, amount: u128) {
            // (auth omitted for brevity)
            let prior_reserve = get_reserve_b(&e);

            // External call to token.transfer(user -> pool)
            let token_b = get_token_b(&e);
            // Build args: from, to, amount
            let args: Vec<soroban_sdk::Val> = Vec::from_array(
                &e,
                [
                    user.clone().into_val(&e),
                    e.current_contract_address().into_val(&e),
                    amount.into_val(&e),
                ],
            );
            e.invoke_contract::<()>(&token_b, &Symbol::new(&e, "transfer"), args);

            // Effect: update reserve with stale prior_reserve
            set_reserve_b(&e, prior_reserve + amount);
        }

        pub fn get_reserve(e: Env) -> u128 {
            get_reserve_b(&e)
        }
    }

    /********************** Malicious Token from dev-dependency *************************/
    use mal_token::MalToken;

    /********************** PoC Test *************************/
    #[test]
    #[should_panic]
    fn reentrancy_deposit_blocked_by_host() {
        let e = Env::default();
        // Deploy contracts
        let pool_addr = Address::generate(&e);
        e.register_contract(&pool_addr, MiniPool);
        let token_addr = Address::generate(&e);
        e.register_contract(&token_addr, MalToken);

        // Initialize contracts
        e.invoke_contract::<()>(&pool_addr, &Symbol::new(&e, "init"), Vec::from_array(&e, [token_addr.clone().into_val(&e)]));
        e.invoke_contract::<()>(&token_addr, &Symbol::new(&e, "init"), Vec::from_array(&e, [pool_addr.clone().into_val(&e)]));

        // User address
        let user = Address::generate(&e);

        // Perform deposit of 10; malicious token will attempt re-entry and the host should panic
        e.invoke_contract::<()>(&pool_addr, &Symbol::new(&e, "deposit"), Vec::from_array(&e, [user.clone().into_val(&e), (10u128).into_val(&e)]));
    }
}