use soroban_sdk::{ Address, Env };

pub trait BufferTrait {
    fn initialize(e: Env, token: Address, max_balance: u128);

    // Deposit collected swap fees
    fn settle_revenue(e: Env, user: Address, amount: u128);

    // Use funds to resolve liquidity deficit
    fn claim_funds(env: Env, user: Address, amount: u128);
}

pub trait AdminInterface {
    // Initialize admin user. Will panic if called twice
    fn init_admin(e: Env, account: Address);

    // Set max balance
    fn set_max_balance(e: Env, admin: Address, max_balance: u128);
}
