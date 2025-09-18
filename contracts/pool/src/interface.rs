use soroban_sdk::{Address, BytesN, Env, Map, Symbol, Vec};
use utils::state::{
    oracle_registry::NormalAction,
    pool::{
        InitializeAllParams, InsuranceClaim, PoolConfig, PoolInfo, PoolStatus, PoolTier,
        SwapDirection,
    },
};

pub trait PoolCrunch {
    // Initialize pool completely to reduce calculations cost
    fn initialize_all(e: Env, params: InitializeAllParams);
}

pub trait PoolTrait {
    fn initialize(e: Env, params: PoolConfig);

    //  ___      ___       __        __    _____  ___
    // |"  \    /"  |     /""\      |" \  (\"   \|"  \
    //  \   \  //   |    /    \     ||  | |.\\   \    |
    //  /\\  \/.    |   /' /\  \    |:  | |: \.   \\  |
    // |: \.        |  //  __'  \   |.  | |.  \    \. |
    // |.  \    /:  | /   /  \\  \  /\  |\|    \    \ |
    // |___|\__/|___|(___/    \___)(__\_|_)\___|\____\)

    // Add liquidity
    fn deposit(e: Env, user: Address, amount: u128, min_shares: u128) -> (u128, u128, i128);

    // Perform an exchange between two coins.
    fn swap(
        e: Env,
        user: Address,
        direction: SwapDirection,
        in_amount: u128,
        out_min: u128,
    ) -> (u128, i128, i128);

    // Estimate amount of coins to retrieve using swap function
    fn estimate_swap(e: Env, direction: SwapDirection, in_amount: u128) -> (u128, i128);

    // Perform an exchange between two coins with strict amount to receive.
    fn swap_strict_receive(
        e: Env,
        user: Address,
        direction: SwapDirection,
        out_amount: u128,
        in_max: u128,
    ) -> (u128, i128, i128);

    // Estimate amount of coins to retrieve using swap_strict_receive function
    fn estimate_swap_strict_receive(
        e: Env,
        direction: SwapDirection,
        out_amount: u128,
    ) -> (u128, i128);

    // Remove liquidity
    fn withdraw(e: Env, user: Address, share_amount: u128, min_amounts: Vec<u128>) -> (u128, i128);

    //   _______    _______  ___________  ___________  _______   _______    ________
    //  /" _   "|  /"     "|("     _   ")("     _   ")/"     "| /"      \  /"       )
    // (: ( \___) (: ______) )__/  \\__/  )__/  \\__/(: ______)|:        |(:   \___/
    //  \/ \       \/    |      \\_ /        \\_ /    \/    |  |_____/   ) \___  \
    //  //  \ ___  // ___)_     |.  |        |.  |    // ___)_  //      /   __/  \\
    // (:   _(  _|(:      "|    \:  |        \:  |   (:      "||:  __   \  /" \   :)
    //  \_______)  \_______)     \__|         \__|    \_______)|__|  \___)(_______/

    // Returns the token contract address for the pool share token
    fn share_id(e: Env) -> Address;

    // Returns the total amount of shares
    fn get_total_shares(e: Env) -> u128;

    fn get_tokens(e: Env) -> Vec<Address>;

    fn get_reserves(e: Env) -> Vec<u128>;

    fn get_fee_fraction(e: Env) -> u32;

    fn get_mint_cap_fraction(e: Env) -> u32;

    fn get_insurance_claim(e: Env) -> InsuranceClaim;

    fn get_info(e: Env) -> PoolInfo;

    fn get_privileged_addrs(e: Env) -> Map<Symbol, Vec<Address>>;

    fn get_liquidity_imbalance(e: Env) -> i128;

    // Returns the protocol fees accumulated in the pool.
    fn get_protocol_fees(e: Env) -> Vec<u128>;
}

pub trait AdminInterfaceTrait {
    //  ___      ___       __        __    _____  ___
    // |"  \    /"  |     /""\      |" \  (\"   \|"  \
    //  \   \  //   |    /    \     ||  | |.\\   \    |
    //  /\\  \/.    |   /' /\  \    |:  | |: \.   \\  |
    // |: \.        |  //  __'  \   |.  | |.  \    \. |
    // |.  \    /:  | /   /  \\  \  /\  |\|    \    \ |
    // |___|\__/|___|(___/    \___)(__\_|_)\___|\____\)

    fn rebalance(e: Env, admin: Address, action: NormalAction) -> i128;

    fn claim_protocol_fees(e: Env, admin: Address, destination: Address) -> Vec<u128>;

    //   ________  _______  ___________  ___________  _______   _______    ________
    //  /"       )/"     "|("     _   ")("     _   ")/"     "| /"      \  /"       )
    // (:   \___/(: ______) )__/  \\__/  )__/  \\__/(: ______)|:        |(:   \___/
    //  \___  \   \/    |      \\_ /        \\_ /    \/    |  |_____/   ) \___  \
    //   __/  \\  // ___)_     |.  |        |.  |    // ___)_  //      /   __/  \\
    //  /" \   :)(:      "|    \:  |        \:  |   (:      "||:  __   \  /" \   :)
    // (_______/  \_______)     \__|         \__|    \_______)|__|  \___)(_______/

    fn set_privileged_addrs(
        e: Env,
        admin: Address,
        rewards_admin: Address,
        operations_admin: Address,
        pause_admin: Address,
        emergency_pause_admins: Vec<Address>,
    );

    fn set_fee(e: Env, admin: Address, fee_fraction: u32);

    fn set_tier(e: Env, admin: Address, tier: PoolTier);

    fn set_status(e: Env, admin: Address, status: PoolStatus);

    fn set_max_imbalances(
        e: Env,
        admin: Address,
        min_collateral_fraction: u32,
        max_insurance: u128,
    );

    fn set_mint_cap_fraction(e: Env, admin: Address, mint_cap_fraction: u32);

    fn set_protocol_fee_fraction(e: Env, admin: Address, new_fraction: u32);

    fn set_oracle_registry(e: Env, admin: Address, oracle_registry: Address);

    fn set_insurance_fund(e: Env, admin: Address, insurance_fund: Address);

    fn set_token_insurance(e: Env, admin: Address, token_insurance: Address);
}

pub trait UpgradeableContract {
    // Get contract version
    fn version() -> u32;

    // Upgrade contract with new wasm code
    fn commit_upgrade(
        e: Env,
        admin: Address,
        new_wasm_hash: BytesN<32>,
        new_token_wasm_hash: BytesN<32>,
    );
    fn apply_upgrade(e: Env, admin: Address) -> (BytesN<32>, BytesN<32>);
    fn revert_upgrade(e: Env, admin: Address);

    // Emergency mode - bypass upgrade deadline
    fn set_emergency_mode(e: Env, admin: Address, value: bool);
    fn get_emergency_mode(e: Env) -> bool;
}

pub trait UpgradeableLPTokenTrait {
    // legacy methods to upgrade token contract up to version 120. future versions will use commit_upgrade
    fn upgrade_token_legacy(e: Env, admin: Address, new_token_wasm: BytesN<32>);
}

pub trait RewardsTrait {
    // Initialize rewards token address
    fn initialize_rewards_config(e: Env, reward_token: Address);

    // Configure rewards for pool. Every second tps of coins
    // being distributed across all liquidity providers
    // after expired_at timestamp distribution ends
    fn set_rewards_config(e: Env, admin: Address, expired_at: u64, tps: u128);

    // Calculate reward token surplus
    fn get_unused_reward(e: Env) -> u128;

    // Return reward token above the configured amount back to the router
    fn return_unused_reward(e: Env, admin: Address) -> u128;

    // Get rewards status for the pool,
    // including amount available for the user
    fn get_rewards_info(e: Env, user: Address) -> Map<Symbol, i128>;

    // Get amount of reward tokens available for the user to claim.
    fn get_user_reward(e: Env, user: Address) -> u128;

    // Checkpoints the reward for the user.
    // Useful when user moves funds by itself to avoid re-entrancy issue.
    // Can be called only by the token contract to notify pool external changes happened.
    fn checkpoint_reward(e: Env, token_contract: Address, user: Address, user_shares: u128);

    // Checkpoints total working balance and the working balance for the user.
    // Useful when user moves funds by itself to avoid re-entrancy issue.
    // Can be called only by the token contract to notify pool external changes happened.
    fn checkpoint_working_balance(
        e: Env,
        token_contract: Address,
        user: Address,
        user_shares: u128,
    );

    // Get total amount of accumulated reward for the pool
    fn get_total_accumulated_reward(e: Env) -> u128;

    // Get total amount of generated plus configured reward for the pool
    fn get_total_configured_reward(e: Env) -> u128;

    // Get total amount of claimed reward for the pool
    fn get_total_claimed_reward(e: Env) -> u128;

    // Claim reward as a user.
    // returns amount of tokens rewarded to the user
    fn claim(e: Env, user: Address) -> u128;
}
