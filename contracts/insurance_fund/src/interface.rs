use soroban_sdk::{Address, Env, Symbol, Vec};

use crate::stake::Stake;

pub trait InsuranceFundTrait {
    fn initialize(
        e: Env,
        admin: Address,
        emergency_admin: Address,
        oracle_registry: Address,
        pool_router: Address,
        premium_token: Address,
        whitelisted_tokens: Vec<Address>,
        unstaking_period: u64,
        optimal_utilization: u32,
        base_rate: i32,
        rate_slopes: (u32, u32),
    );

    //  ___      ___       __        __    _____  ___
    // |"  \    /"  |     /""\      |" \  (\"   \|"  \
    //  \   \  //   |    /    \     ||  | |.\\   \    |
    //  /\\  \/.    |   /' /\  \    |:  | |: \.   \\  |
    // |: \.        |  //  __'  \   |.  | |.  \    \. |
    // |.  \    /:  | /   /  \\  \  /\  |\|    \    \ |
    // |___|\__/|___|(___/    \___)(__\_|_)\___|\____\)

    fn deposit(e: Env, user: Address, token: Address, amount: u128);

    fn request_withdraw(e: Env, user: Address, token: Address, amount: u128);

    fn cancel_request_withdraw(e: Env, user: Address, token: Address);

    fn withdraw(e: Env, user: Address, token: Address);

    fn pay_premium(e: Env, sender: Address, amount: u128);

    // Sync token balances with reserves
    fn sync(e: Env, sender: Address, token: Address);

    // Skim excess token balances
    fn skim(e: Env, sender: Address, token: Address);

    //   _______    _______  ___________  ___________  _______   _______    ________
    //  /" _   "|  /"     "|("     _   ")("     _   ")/"     "| /"      \  /"       )
    // (: ( \___) (: ______) )__/  \\__/  )__/  \\__/(: ______)|:        |(:   \___/
    //  \/ \       \/    |      \\_ /        \\_ /    \/    |  |_____/   ) \___  \
    //  //  \ ___  // ___)_     |.  |        |.  |    // ___)_  //      /   __/  \\
    // (:   _(  _|(:      "|    \:  |        \:  |   (:      "||:  __   \  /" \   :)
    //  \_______)  \_______)     \__|         \__|    \_______)|__|  \___)(_______/

    fn get_whitelist_tokens(e: Env) -> Vec<Address>;

    fn get_deletion_queue_tokens(e: Env) -> Vec<Address>;

    fn get_pool_router(e: Env) -> Address;

    fn get_unstaking_period(e: Env) -> u64;

    fn get_optimal_insurance(e: Env) -> u128;

    fn get_total_shares(e: Env) -> u128;

    fn get_share_base(e: Env) -> u128;

    fn get_stake(e: Env, user: Address) -> Stake;

    fn get_optimal_utilization(e: Env) -> u32;

    fn get_utilization(e: Env) -> u32;

    fn get_rate(e: Env) -> i32;

    fn get_base_rate(e: Env) -> i32;

    fn get_rate_slopes(e: Env) -> (u32, u32);

    fn get_premium_whitelist_status(e: Env, address: Address) -> bool;
}

pub trait AdminInterface {
    //  ___      ___       __        __    _____  ___
    // |"  \    /"  |     /""\      |" \  (\"   \|"  \
    //  \   \  //   |    /    \     ||  | |.\\   \    |
    //  /\\  \/.    |   /' /\  \    |:  | |: \.   \\  |
    // |: \.        |  //  __'  \   |.  | |.  \    \. |
    // |.  \    /:  | /   /  \\  \  /\  |\|    \    \ |
    // |___|\__/|___|(___/    \___)(__\_|_)\___|\____\)

    fn sync_optimal_insurance(e: Env, admin: Address);

    fn resolve_liquidity_deficit(e: Env, admin: Address, asset: Symbol);

    //   ________  _______  ___________  ___________  _______   _______    ________
    //  /"       )/"     "|("     _   ")("     _   ")/"     "| /"      \  /"       )
    // (:   \___/(: ______) )__/  \\__/  )__/  \\__/(: ______)|:        |(:   \___/
    //  \___  \   \/    |      \\_ /        \\_ /    \/    |  |_____/   ) \___  \
    //   __/  \\  // ___)_     |.  |        |.  |    // ___)_  //      /   __/  \\
    //  /" \   :)(:      "|    \:  |        \:  |   (:      "||:  __   \  /" \   :)
    // (_______/  \_______)     \__|         \__|    \_______)|__|  \___)(_______/

    fn set_pool_router(e: Env, admin: Address, pool_router: Address);
    fn add_whitelist_token(e: Env, admin: Address, token: Address);

    fn set_unstaking_period(e: Env, admin: Address, unstaking_period: u64);

    fn set_rate_config(
        e: Env,
        admin: Address,
        optimal_utilization: u32,
        base_rate: i32,
        rate_slope_a: u32,
        rate_slope_b: u32,
    );

    fn set_whitelist_status(e: Env, admin: Address, address: Address, status: bool);

    fn remove_whitelist_token(e: Env, admin: Address, token: Address);

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
