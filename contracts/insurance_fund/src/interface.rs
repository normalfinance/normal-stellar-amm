use soroban_sdk::{ Address, Env };

use crate::stake::Stake;

pub trait InsuranceFundTrait {
    //
    fn initialize(
        e: Env,
        admin: Address,
        emergency_admin: Address,
        token: Address,
        coverage_buffer: u128,
        optimal_utilization: u32,
        base_rate: i32,
        rate_slopes: (i32, i32)
    );

    fn deposit(e: Env, user: Address, amount: u128);

    fn request_withdraw(e: Env, user: Address, amount: u128);

    fn cancel_request_withdraw(e: Env, user: Address);

    fn withdraw(e: Env, user: Address);

    fn pay_premium(e: Env, sender: Address, amount: u128);

    //   _______    _______  ___________  ___________  _______   _______    ________
    //  /" _   "|  /"     "|("     _   ")("     _   ")/"     "| /"      \  /"       )
    // (: ( \___) (: ______) )__/  \\__/  )__/  \\__/(: ______)|:        |(:   \___/
    //  \/ \       \/    |      \\_ /        \\_ /    \/    |  |_____/   ) \___  \
    //  //  \ ___  // ___)_     |.  |        |.  |    // ___)_  //      /   __/  \\
    // (:   _(  _|(:      "|    \:  |        \:  |   (:      "||:  __   \  /" \   :)
    //  \_______)  \_______)     \__|         \__|    \_______)|__|  \___)(_______/

    fn get_token(e: Env) -> Address;

    fn get_unstaking_period(e: Env) -> u64;

    fn get_optimal_coverage(e: Env) -> u128;

    fn get_coverage_buffer(e: Env) -> u128;

    fn get_total_shares(e: Env) -> u128;

    fn get_share_base(e: Env) -> u128;

    fn get_stake(e: Env, user: Address) -> Stake;

    fn get_optimal_utilization(e: Env) -> u32;

    fn get_utilization(e: Env) -> u32;

    fn get_rate(e: Env) -> i32;

    fn get_base_rate(e: Env) -> i32;

    fn get_rate_slopes(e: Env) -> (i32, i32);
}

pub trait AdminInterface {
    // Set unstaking period
    fn set_unstaking_period(e: Env, admin: Address, unstaking_period: u64);

    //
    fn set_optimal_coverage(e: Env, admin: Address, optimal_coverage: u128);

    fn set_coverage_buffer(e: Env, admin: Address, coverage_buffer: u128);

    //
    fn set_rate_config(
        e: Env,
        admin: Address,
        optimal_utilization: u32,
        base_rate: i32,
        rate_slope_a: i32,
        rate_slope_b: i32
    );

    //
    fn resolve_liquidity_deficit(e: Env, admin: Address, pool_address: Address);

    //    _______     __       ____  ____   ________  _______  ________
    //   |   __ "\   /""\     ("  _||_ " | /"       )/"     "||"      "\
    //   (. |__) :) /    \    |   (  ) : |(:   \___/(: ______)(.  ___  :)
    //   |:  ____/ /' /\  \   (:  |  | . ) \___  \   \/    |  |: \   ) ||
    //   (|  /    //  __'  \   \\ \__/ //   __/  \\  // ___)_ (| (___\ ||
    //  /|__/ \  /   /  \\  \  /\\ __ //\  /" \   :)(:      "||:       :)
    // (_______)(___/    \___)(__________)(_______/  \_______)(________/

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
