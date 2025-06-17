use soroban_sdk::{ Address, BytesN, Env, Map, String, Symbol, Val, Vec, U256 };
use utils::{ storage::{ PoolInfo, PoolTier } };

pub trait PoolInterfaceTrait {
    // Get symbolic explanation of pool type.
    fn pool_type(e: Env, tokens: Vec<Address>, pool_index: BytesN<32>) -> Symbol;

    // Get dictionary of basic pool information: type, fee, special parameters if any.
    fn get_info(e: Env, tokens: Vec<Address>, pool_index: BytesN<32>) -> Map<Symbol, Val>;

    // Get address for specified pool index.
    fn get_pool(e: Env, tokens: Vec<Address>, pool_index: BytesN<32>) -> Address;

    // Returns the token contract address for the pool share token.
    fn share_id(e: Env, tokens: Vec<Address>, pool_index: BytesN<32>) -> Address;

    // Returns the total amount of shares
    fn get_total_shares(e: Env, tokens: Vec<Address>, pool_index: BytesN<32>) -> u128;

    // Getter for the pool balances array.
    fn get_reserves(e: Env, tokens: Vec<Address>, pool_index: BytesN<32>) -> Vec<u128>;

    // Fee fraction getter. 1 = 0.01%
    fn get_fee_fraction(e: Env, tokens: Vec<Address>, pool_index: BytesN<32>) -> u32;

    // Deposit coins into the pool.
    // desired_amounts: List of amounts of coins to deposit
    // Returns amounts deposited and the amount of LP tokens received in exchange for the deposited tokens.
    fn deposit(
        e: Env,
        user: Address,
        tokens: Vec<Address>,
        pool_index: BytesN<32>,
        token_b_amount: u128
    ) -> (u128, u128);

    // Perform an exchange between two coins.
    // token_in: token to send
    // token_out: token to receive
    // in_amount: Amount of token_in being exchanged
    // out_min: Minimum amount of token_out to receive
    // Returns the actual amount of coin out received
    fn swap(
        e: Env,
        user: Address,
        tokens: Vec<Address>,
        token_in: Address,
        token_out: Address,
        pool_index: BytesN<32>,
        in_amount: u128,
        out_min: u128
    ) -> u128;

    // Estimate amount of coins to retrieve using swap function
    fn estimate_swap(
        e: Env,
        tokens: Vec<Address>,
        token_in: Address,
        token_out: Address,
        pool_index: BytesN<32>,
        in_amount: u128
    ) -> (u128, i128);

    // Withdraw coins from the pool.
    // share_amount: Quantity of LP tokens to burn in the withdrawal
    // Returns a list of the amounts for each coin that was withdrawn.
    fn withdraw(
        e: Env,
        user: Address,
        tokens: Vec<Address>,
        pool_index: BytesN<32>,
        share_amount: u128
    ) -> u128;

    fn get_liquidity(e: Env, tokens: Vec<Address>, pool_index: BytesN<32>) -> U256;

    // Set liquidity calculator address. it's separate contract optimized to estimate liquidity for multiple pools
    fn set_liquidity_calculator(e: Env, admin: Address, calculator: Address);

    // Get liquidity calculator address
    fn get_liquidity_calculator(e: Env) -> Address;
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
    fn get_rewards_config(e: Env) -> Map<Symbol, i128>;

    // Returns a mapping of token addresses to their respective reward information.
    //
    // # Returns
    //
    // A `Map` where each key is a `Vec<Address>` representing a set of token addresses, and the value is a tuple
    // `(u32, bool, U256)`. The tuple elements represent the voting share, processed status, and total liquidity
    // of the tokens respectively.
    fn get_tokens_for_reward(e: Env) -> Map<Vec<Address>, (u32, bool, U256)>;

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
        tokens_votes: Vec<(Vec<Address>, u32)>
    );

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
    fn config_pool_rewards(e: Env, tokens: Vec<Address>, pool_index: BytesN<32>) -> u128;

    // Get rewards status for the pool,
    // including amount available for the user
    fn get_rewards_info(
        e: Env,
        user: Address,
        tokens: Vec<Address>,
        pool_index: BytesN<32>
    ) -> Map<Symbol, i128>;

    // Get amount of reward tokens available for the user to claim.
    fn get_user_reward(e: Env, user: Address, tokens: Vec<Address>, pool_index: BytesN<32>) -> u128;

    // Get total amount of accumulated reward for the pool
    fn get_total_accumulated_reward(e: Env, tokens: Vec<Address>, pool_index: BytesN<32>) -> u128;

    // Get total amount of generated plus configured reward for the pool
    fn get_total_configured_reward(e: Env, tokens: Vec<Address>, pool_index: BytesN<32>) -> u128;

    // Get total amount of claimed reward for the pool
    fn get_total_claimed_reward(e: Env, tokens: Vec<Address>, pool_index: BytesN<32>) -> u128;

    // Calculate difference between total configured reward and total claimed reward.
    // Helps to estimate the amount of missing reward tokens pool has configured to distribute
    fn get_total_outstanding_reward(e: Env, tokens: Vec<Address>, pool_index: BytesN<32>) -> u128;

    // Transfer outstanding reward to the pool
    fn distribute_outstanding_reward(
        e: Env,
        user: Address,
        from: Address,
        tokens: Vec<Address>,
        pool_index: BytesN<32>
    ) -> u128;

    // Claim reward as a user.
    // returns amount of tokens rewarded to the user
    fn claim(e: Env, user: Address, tokens: Vec<Address>, pool_index: BytesN<32>) -> u128;
}

pub trait PoolsManagementTrait {
    // Initialize standard pool with custom arguments.
    // fee_fraction should match pre-defined set of values: 0.1%, 0.3%, 1%
    // 10 = 0.1%, 30 = 0.3%, 100 = 1%
    fn init_pool(
        e: Env,
        user: Address,
        oracle_registry_ids: (Symbol, Symbol),
        asset: Address,
        tokens: Vec<Address>,
        lp_token_info: (String, String),
        fee_fraction: u32,
        tier: PoolTier,
        quote_max_insurance: u128,
        oracle_registry: Address
    ) -> (BytesN<32>, Address);

    // Get all pools addresses
    fn query_pools(e: Env) -> Vec<Address>;

    //
    fn query_pool_details(env: Env, pool_address: Address) -> PoolInfo;

    //
    fn query_all_pools_details(env: Env) -> Vec<PoolInfo>;

    // Get pools for given pair
    fn get_pools(e: Env, tokens: Vec<Address>) -> Map<BytesN<32>, Address>;

    // Remove pool from the list
    fn remove_pool(e: Env, user: Address, tokens: Vec<Address>, pool_hash: BytesN<32>);

    // Calculates the number of unique token sets.
    fn get_tokens_sets_count(e: Env) -> u128;

    // Retrieves tokens at a specified index
    fn get_tokens(e: Env, index: u128) -> Vec<Address>;

    // Retrieves a lists of pools in batch based on half-open `[..)` range of tokens indexes.
    //
    // # Returns
    //
    // A list containing tuples containing a vector of addresses of the corresponding tokens
    // and a mapping of pool hashes to pool addresses.
    fn get_pools_for_tokens_range(
        e: Env,
        start: u128,
        end: u128
    ) -> Vec<(Vec<Address>, Map<BytesN<32>, Address>)>;
}

pub trait PoolPlaneInterface {
    // configure pools plane address to be used as lightweight proxy to optimize instructions & batch operations
    fn set_pools_plane(e: Env, admin: Address, plane: Address);

    // get pools plane address
    fn get_plane(e: Env) -> Address;
}
