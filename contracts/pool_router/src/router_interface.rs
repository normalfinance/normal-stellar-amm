use soroban_sdk::{Address, BytesN, Env, Vec};

pub trait AdminInterface {
    // Initialize admin user. Will panic if called twice
    fn init_admin(e: Env, account: Address);

    fn set_privileged_addrs(
        e: Env,
        admin: Address,
        rewards_admin: Address,
        operations_admin: Address,
        pause_admin: Address,
        emergency_pause_admins: Vec<Address>,
    );

    // Set liquidity pool token wasm hash
    fn set_token_hash(e: Env, admin: Address, new_hash: BytesN<32>);

    // Set standard pool wasm hash
    fn set_pool_hash(e: Env, admin: Address, new_hash: BytesN<32>);

    // Set reward token address
    fn set_reward_token(e: Env, admin: Address, reward_token: Address);

    fn set_liquidity_calculator(e: Env, admin: Address, calculator: Address);
}
