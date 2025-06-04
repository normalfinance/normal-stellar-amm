use soroban_sdk::{ Address, Env };

pub trait InsuranceFundTrait {
    fn initialize(e: Env, deposit_token: Address, unstaking_period: u64, max_shares: u128);

    fn deposit(e: Env, user: Address, amount: u128);

    fn request_withdraw(env: Env, user: Address, amount: u128);

    fn cancel_request_withdraw(env: Env, user: Address);

    fn withdraw(env: Env, user: Address);

    fn collect_reward(e: Env, user: Address);
}

pub trait AdminInterface {
    // Initialize admin user. Will panic if called twice
    fn init_admin(e: Env, account: Address);

    // Set unstaking period
    fn set_unstaking_period(e: Env, admin: Address, unstaking_period: u64);

    // Set max insurance
    fn set_max_shares(e: Env, admin: Address, max_shares: u128);

    //
    fn resolve_liquidity_deficit(e: Env, admin: Address, pool_address: Address);

    // Stop staking instantly
    fn kill_deposit(e: Env, admin: Address);
    fn kill_request_withdraw(e: Env, admin: Address);
    fn kill_withdraw(e: Env, admin: Address);

    // Resume staking
    fn unkill_deposit(e: Env, admin: Address);
    fn unkill_request_withdraw(e: Env, admin: Address);
    fn unkill_withdraw(e: Env, admin: Address);

    // Get killswitch status
    fn get_is_killed_deposit(e: Env) -> bool;
    fn get_is_killed_request_withdraw(e: Env) -> bool;
    fn get_is_killed_withdraw(e: Env) -> bool;
}
