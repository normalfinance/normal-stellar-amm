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

    fn set_insurance_fund(e: Env, admin: Address, insurance_fund: Address);

    fn set_liquidity_calculator(e: Env, admin: Address, calculator: Address);

    fn set_oracle_registry(e: Env, admin: Address, oracle_registry: Address);

    fn set_token_share_hash(e: Env, admin: Address, new_hash: BytesN<32>);

    fn set_pool_hash(e: Env, admin: Address, new_hash: BytesN<32>);

    fn set_reward_token(e: Env, admin: Address, reward_token: Address);

    //   _______    _______  ___________  ___________  _______   _______    ________
    //  /" _   "|  /"     "|("     _   ")("     _   ")/"     "| /"      \  /"       )
    // (: ( \___) (: ______) )__/  \\__/  )__/  \\__/(: ______)|:        |(:   \___/
    //  \/ \       \/    |      \\_ /        \\_ /    \/    |  |_____/   ) \___  \
    //  //  \ ___  // ___)_     |.  |        |.  |    // ___)_  //      /   __/  \\
    // (:   _(  _|(:      "|    \:  |        \:  |   (:      "||:  __   \  /" \   :)
    //  \_______)  \_______)     \__|         \__|    \_______)|__|  \___)(_______/

    fn get_insurance_fund(e: Env) -> Address;
}
