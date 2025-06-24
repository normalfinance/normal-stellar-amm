use soroban_sdk::{ Address, Env, Map, String, Symbol, Val, Vec, U256 };
use utils::state::pool::{ PoolInfo, PoolTier };

pub trait PoolInterfaceTrait {
    //  ___      ___       __        __    _____  ___
    // |"  \    /"  |     /""\      |" \  (\"   \|"  \
    //  \   \  //   |    /    \     ||  | |.\\   \    |
    //  /\\  \/.    |   /' /\  \    |:  | |: \.   \\  |
    // |: \.        |  //  __'  \   |.  | |.  \    \. |
    // |.  \    /:  | /   /  \\  \  /\  |\|    \    \ |
    // |___|\__/|___|(___/    \___)(__\_|_)\___|\____\)

    fn deposit(e: Env, user: Address, asset: Symbol, token_b_amount: u128) -> (u128, u128);

    fn swap(
        e: Env,
        user: Address,
        tokens: Vec<Address>,
        token_in: Address,
        token_out: Address,
        asset: Symbol,
        in_amount: u128,
        out_min: u128
    ) -> u128;

    fn estimate_swap(
        e: Env,
        tokens: Vec<Address>,
        token_in: Address,
        token_out: Address,
        asset: Symbol,
        in_amount: u128
    ) -> (u128, i128);

    fn withdraw(e: Env, user: Address, asset: Symbol, share_amount: u128) -> u128;

    //   _______    _______  ___________  ___________  _______   _______    ________
    //  /" _   "|  /"     "|("     _   ")("     _   ")/"     "| /"      \  /"       )
    // (: ( \___) (: ______) )__/  \\__/  )__/  \\__/(: ______)|:        |(:   \___/
    //  \/ \       \/    |      \\_ /        \\_ /    \/    |  |_____/   ) \___  \
    //  //  \ ___  // ___)_     |.  |        |.  |    // ___)_  //      /   __/  \\
    // (:   _(  _|(:      "|    \:  |        \:  |   (:      "||:  __   \  /" \   :)
    //  \_______)  \_______)     \__|         \__|    \_______)|__|  \___)(_______/

    // Get dictionary of basic pool information: type, fee, special parameters if any.
    fn get_info(e: Env, asset: Symbol) -> Map<Symbol, Val>;

    // Get address for specified pool index.
    fn get_pool(e: Env, asset: Symbol) -> Address;

    // Returns the token contract address for the pool share token.
    fn share_id(e: Env, asset: Symbol) -> Address;

    // Returns the total amount of shares
    fn get_total_shares(e: Env, asset: Symbol) -> u128;

    // Getter for the pool balances array.
    fn get_reserves(e: Env, asset: Symbol) -> Vec<u128>;

    // Fee fraction getter. 1 = 0.01%
    fn get_fee_fraction(e: Env, asset: Symbol) -> u32;

    // Insurance Claim - Max quote insurance getter.
    fn get_insurance_coverage(e: Env, asset: Symbol) -> u128;

    fn get_liquidity(e: Env, asset: Symbol) -> U256;

    fn get_liquidity_calculator(e: Env) -> Address;

    // Get map of privileged roles
    fn get_privileged_addrs(e: Env) -> Map<Symbol, Vec<Address>>;
}

pub trait IncentivesInterfaceTrait {
    // Retrieves the global rewards configuration and returns it as a `Map`.
    //
    // This function fetches the global rewards configuration from the contract's state.
    // The configuration includes the rewards per second (`tps`) and the expiration timestamp (`expired_at`)
    //
    // # Returns
    //
    // A `Map` where each key is a `Symbol` representing a configuration parameter, and the value is the corresponding value.
    // The keys are "tps" and "expired_at".
    fn get_incentives_config(e: Env) -> Map<Symbol, i128>;

    // Returns a mapping of token addresses to their respective reward information.
    //
    // # Returns
    //
    // A `Map` where each key is a `Symbol` representing an oracle id, and the value is a tuple
    // `(bool, U256)`. The tuple elements represent the processed status, and total liquidity
    // of the tokens respectively.
    fn get_tokens_for_reward(e: Env) -> Map<Symbol, (bool, U256)>;

    // Sums up the liquidity of all pools for given tokens set and returns the total liquidity
    //
    // # Returns
    //
    // A `U256` value representing the total liquidity for the given set of tokens.
    fn get_total_liquidity(e: Env, asset: Symbol) -> U256;

    // Configures the global rewards for the liquidity pool.
    //
    // # Arguments
    //
    // * `user` - This user must be authenticated and have admin or operator privileges.
    // * `reward_tps` - The rewards per second. This value is scaled by 1e7 for precision.
    // * `expired_at` - The timestamp at which the rewards configuration will expire.
    // * `tokens_votes` - A vector of tuples, where each tuple contains a vector of token addresses and a voting share.
    //   The voting share is a value between 0 and 1, scaled by 1e7 for precision.
    fn config_global_rewards(
        e: Env,
        user: Address,
        reward_tps: u128,
        expired_at: u64,
        assets: Vec<Symbol>
    );

    // Fills the aggregated liquidity information for a given set of tokens.
    fn fill_liquidity(e: Env, asset: Symbol);

    // Configures the rewards for a specific pool.
    //
    // This function is used to set up the rewards configuration for a specific pool.
    // It calculates the pool's share of the total rewards based on its liquidity and sets the pool's rewards configuration.
    //
    // # Arguments
    //
    // * `tokens` - A vector of token addresses that the pool consists of.
    // * `pool_index` - The index of the pool.
    //
    // # Returns
    //
    // * `pool_tps` - The total reward tokens per second (TPS) to be distributed to the pool.
    //
    // # Errors
    //
    // This function will panic if:
    //
    // * The pool does not exist.
    // * The tokens are not found in the current rewards configuration.
    // * The liquidity for the tokens has not been filled.
    fn config_pool_rewards(e: Env, asset: Symbol) -> u128;

    // Get rewards status for the pool,
    // including amount available for the user
    fn get_incentives_info(e: Env, user: Address, asset: Symbol) -> Map<Symbol, i128>;

    // Get amount of reward tokens available for the user to claim.
    fn get_user_reward(e: Env, user: Address, asset: Symbol) -> u128;

    // Get amount of LP fees available for the user to claim.
    fn get_user_fees(e: Env, user: Address, asset: Symbol) -> u128;

    // Get total amount of accumulated reward for the pool
    fn get_total_accumulated_reward(e: Env, asset: Symbol) -> u128;

    // Get total amount of generated plus configured reward for the pool
    fn get_total_configured_reward(e: Env, asset: Symbol) -> u128;

    // Get total amount of claimed reward for the pool
    fn get_total_claimed_reward(e: Env, asset: Symbol) -> u128;

    // Calculate difference between total configured reward and total claimed reward.
    // Helps to estimate the amount of missing reward tokens pool has configured to distribute
    fn get_total_outstanding_reward(e: Env, asset: Symbol) -> u128;

    // Transfer outstanding reward to the pool
    fn distribute_outstanding_reward(
        e: Env,
        user: Address,
        from: Address,
        asset: Symbol
    ) -> u128;

    // Claim reward as a user.
    // returns amount of tokens rewarded to the user
    fn claim(e: Env, user: Address, asset: Symbol) -> u128;
}

pub trait PoolsManagementTrait {
    //  ___      ___       __        __    _____  ___
    // |"  \    /"  |     /""\      |" \  (\"   \|"  \
    //  \   \  //   |    /    \     ||  | |.\\   \    |
    //  /\\  \/.    |   /' /\  \    |:  | |: \.   \\  |
    // |: \.        |  //  __'  \   |.  | |.  \    \. |
    // |.  \    /:  | /   /  \\  \  /\  |\|    \    \ |
    // |___|\__/|___|(___/    \___)(__\_|_)\___|\____\)

    fn init_pool(
        e: Env,
        admin: Address,
        assets: (Symbol, Symbol),
        tokens: Vec<Address>,
        lp_token_info: (String, String),
        fee_fraction: u32,
        tier: PoolTier,
        quote_max_insurance: u128
    ) -> Address;

    fn remove_pool(e: Env, user: Address, asset: Symbol);

    //   _______    _______  ___________  ___________  _______   _______    ________
    //  /" _   "|  /"     "|("     _   ")("     _   ")/"     "| /"      \  /"       )
    // (: ( \___) (: ______) )__/  \\__/  )__/  \\__/(: ______)|:        |(:   \___/
    //  \/ \       \/    |      \\_ /        \\_ /    \/    |  |_____/   ) \___  \
    //  //  \ ___  // ___)_     |.  |        |.  |    // ___)_  //      /   __/  \\
    // (:   _(  _|(:      "|    \:  |        \:  |   (:      "||:  __   \  /" \   :)
    //  \_______)  \_______)     \__|         \__|    \_______)|__|  \___)(_______/

    fn query_pool_details(env: Env, pool_address: Address) -> PoolInfo;

    fn query_all_pools_details(env: Env) -> Vec<PoolInfo>;

    fn get_pools(e: Env) -> Vec<Address>;
}

pub trait PoolPlaneInterface {
    // configure pools plane address to be used as lightweight proxy to optimize instructions & batch operations
    fn set_pools_plane(e: Env, admin: Address, plane: Address);

    // get pools plane address
    fn get_plane(e: Env) -> Address;
}
