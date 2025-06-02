use soroban_sdk::{ Address, Env };

pub trait InsuranceFundTrait {
    fn initialize(e: Env, deposit_token: Address, max_shares: u128);

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

    // Deposit collected swap fees
    fn settle_revenue(e: Env, admin: Address, amount: u128) -> u128;

    // Stop pool instantly
    fn kill_deposit(e: Env, admin: Address);
    fn kill_request_withdraw(e: Env, admin: Address);
    fn kill_withdraw(e: Env, admin: Address);

    // Resume pool
    fn unkill_deposit(e: Env, admin: Address);
    fn unkill_request_withdraw(e: Env, admin: Address);
    fn unkill_withdraw(e: Env, admin: Address);

    // Get killswitch status
    fn get_is_killed_deposit(e: Env) -> bool;
    fn get_is_killed_request_withdraw(e: Env) -> bool;
    fn get_is_killed_withdraw(e: Env) -> bool;
}
