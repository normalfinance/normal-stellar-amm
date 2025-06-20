use soroban_sdk::{ Address, Env };

use crate::reserve::Reserve;

pub trait BufferTrait {
    fn initialize(
        e: Env,
        admin: Address,
        emergency_admin: Address,
        time_bt_payouts: u64,
        min_reserve_ratio: u32
    );

    //  ___      ___       __        __    _____  ___
    // |"  \    /"  |     /""\      |" \  (\"   \|"  \
    //  \   \  //   |    /    \     ||  | |.\\   \    |
    //  /\\  \/.    |   /' /\  \    |:  | |: \.   \\  |
    // |: \.        |  //  __'  \   |.  | |.  \    \. |
    // |.  \    /:  | /   /  \\  \  /\  |\|    \    \ |
    // |___|\__/|___|(___/    \___)(__\_|_)\___|\____\)

    // Deposit swap fees into the Buffer
    fn deposit(e: Env, sender: Address, token: Address, amount: u128);

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

    fn get_min_time_between_payouts(e: Env) -> u64;

    fn get_reserve(e: Env, token: Address) -> Reserve;

    fn get_min_reserve_ratio(e: Env) -> u32;

    fn get_last_payout_timestamp(e: Env) -> u64;
}

pub trait AdminInterface {
    //   ________  _______  ___________  ___________  _______   _______    ________
    //  /"       )/"     "|("     _   ")("     _   ")/"     "| /"      \  /"       )
    // (:   \___/(: ______) )__/  \\__/  )__/  \\__/(: ______)|:        |(:   \___/
    //  \___  \   \/    |      \\_ /        \\_ /    \/    |  |_____/   ) \___  \
    //   __/  \\  // ___)_     |.  |        |.  |    // ___)_  //      /   __/  \\
    //  /" \   :)(:      "|    \:  |        \:  |   (:      "||:  __   \  /" \   :)
    // (_______/  \_______)     \__|         \__|    \_______)|__|  \___)(_______/

    fn set_min_time_between_payouts(e: Env, admin: Address, min_time: u64);

    fn set_min_reserve_ratio(e: Env, admin: Address, min_ratio: u32);

    fn set_reserve_max_balance(e: Env, admin: Address, token: Address, max_balance: u128);

    //  ___      ___       __        __    _____  ___
    // |"  \    /"  |     /""\      |" \  (\"   \|"  \
    //  \   \  //   |    /    \     ||  | |.\\   \    |
    //  /\\  \/.    |   /' /\  \    |:  | |: \.   \\  |
    // |: \.        |  //  __'  \   |.  | |.  \    \. |
    // |.  \    /:  | /   /  \\  \  /\  |\|    \    \ |
    // |___|\__/|___|(___/    \___)(__\_|_)\___|\____\)

    // Resolve pool liquidity deficit using reserves
    fn resolve_liquidity_deficit(
        e: Env,
        admin: Address,
        token: Address,
        amount: u128,
        pool_address: Address
    ) -> u128;

    // Withdraw surplus reserves
    fn withdraw_surplus(e: Env, admin: Address, token: Address, amount: u128);

    //    _______     __       ____  ____   ________  _______  ________
    //   |   __ "\   /""\     ("  _||_ " | /"       )/"     "||"      "\
    //   (. |__) :) /    \    |   (  ) : |(:   \___/(: ______)(.  ___  :)
    //   |:  ____/ /' /\  \   (:  |  | . ) \___  \   \/    |  |: \   ) ||
    //   (|  /    //  __'  \   \\ \__/ //   __/  \\  // ___)_ (| (___\ ||
    //  /|__/ \  /   /  \\  \  /\\ __ //\  /" \   :)(:      "||:       :)
    // (_______)(___/    \___)(__________)(_______/  \_______)(________/

    // Stop buffer instantly
    fn kill_deposit(e: Env, admin: Address);
    fn kill_resolve_liquidity_deficit(e: Env, admin: Address);

    // Resume buffer
    fn unkill_deposit(e: Env, admin: Address);
    fn unkill_resolve_liquidity_deficit(e: Env, admin: Address);

    // Get killswitch status
    fn get_is_killed_deposit(e: Env) -> bool;
    fn get_is_killed_resolve_deficit(e: Env) -> bool;
}
