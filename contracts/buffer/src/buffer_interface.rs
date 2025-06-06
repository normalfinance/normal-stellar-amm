use soroban_sdk::{ Address, Env };

use crate::reserve::Reserve;

pub trait BufferTrait {
    // Deposit swap fees into Buffer
    fn deposit(e: Env, sender: Address, token: Address, amount: u128);

    // Resolve pool liquidity deficit using reserves
    fn request_payout(e: Env, sender: Address, oken: Address, amount: u128);

    // Getter for a buffer reserve.
    fn get_reserve(e: Env, token: Address) -> Reserve;

    // Get the minimum reserve ratio
    fn get_min_reserve_ratio(e: Env) -> u128;

    // Get the last payout timestamp
    fn get_last_payout_timestamp(e: Env) -> u64;
}

pub trait AdminInterface {
    // Set the Router
    fn set_router(e: Env, admin: Address, router: Address);

    // Set the Fee Collector
    fn set_fee_collector(e: Env, admin: Address, fee_collector: Address);

    // Set reserve max balance
    fn set_reserve_max_balance(e: Env, admin: Address, token: Address, max_balance: u128);

    // Set min time between payouts
    fn set_min_time_between_payouts(e: Env, admin: Address, min_time: u64);

    // Withdraw surplus reserves
    fn withdraw_surplus(e: Env, admin: Address, token: Address, amount: u128);

    // Sync token balances with reserves
    fn sync(e: Env, admin: Address, token: Address);

    // Skim excess token balances
    fn skim(e: Env, admin: Address, token: Address) -> u128;

    // Stop buffer instantly
    fn kill_deposit(e: Env, admin: Address);
    fn kill_request_payout(e: Env, admin: Address);

    // Resume buffer
    fn unkill_deposit(e: Env, admin: Address);
    fn unkill_request_payout(e: Env, admin: Address);

    // Get killswitch status
    fn get_is_killed_deposit(e: Env) -> bool;
    fn get_is_killed_request_payout(e: Env) -> bool;
}
