use soroban_sdk::{ Address, BytesN, Env, Map, Symbol, Vec };

use crate::storage::OracleGuardRails;

pub trait AdminInterface {
    // Initialize admin user. Will panic if called twice
    fn init_admin(e: Env, account: Address);

    // Set privileged addresses
    fn set_privileged_addrs(
        e: Env,
        admin: Address,
        rewards_admin: Address,
        operations_admin: Address,
        pause_admin: Address,
        emergency_pause_admins: Vec<Address>
    );

    // Get map of privileged roles
    fn get_privileged_addrs(e: Env) -> Map<Symbol, Vec<Address>>;

    // Set liquidity pool token wasm hash
    fn set_token_hash(e: Env, admin: Address, new_hash: BytesN<32>);

    // Set standard pool wasm hash
    fn set_pool_hash(e: Env, admin: Address, new_hash: BytesN<32>);

    // Set reward token address
    fn set_reward_token(e: Env, admin: Address, reward_token: Address);

    // Set rewards boost config: token and feed
    fn set_reward_boost_config(
        e: Env,
        admin: Address,
        reward_boost_token: Address,
        reward_boost_feed: Address
    );

    // Set oracle guardrails
    fn set_oracle_guardrails(e: Env, admin: Address, oracle_guard_rails: OracleGuardRails);

    //
    fn set_supported_quote_tokens(e: Env, tokens: Vec<Address>);
}
