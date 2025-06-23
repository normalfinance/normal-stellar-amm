use soroban_sdk::{ Address, BytesN, Env, Map, Symbol, Vec };
use utils::state::pool::{ InitializeAllParams, InitializeParams, PoolInfo, PoolStatus, PoolTier };

pub trait PoolCrunch {
    // Initialize pool completely to reduce calculations cost
    fn initialize_all(e: Env, params: InitializeAllParams);
}

pub trait PoolTrait {
    fn initialize(e: Env, params: InitializeParams);

    //  ___      ___       __        __    _____  ___
    // |"  \    /"  |     /""\      |" \  (\"   \|"  \
    //  \   \  //   |    /    \     ||  | |.\\   \    |
    //  /\\  \/.    |   /' /\  \    |:  | |: \.   \\  |
    // |: \.        |  //  __'  \   |.  | |.  \    \. |
    // |.  \    /:  | /   /  \\  \  /\  |\|    \    \ |
    // |___|\__/|___|(___/    \___)(__\_|_)\___|\____\)

    // Add liquidity
    fn deposit(e: Env, user: Address, token_b_amount: u128) -> (u128, u128);

    // Perform an exchange between two coins.
    // in_idx: Index value for the coin to send
    // out_idx: Index value of the coin to receive
    // in_amount: Amount of in_idx being exchanged
    // out_min: Minimum amount of out_idx to receive
    // Returns the actual amount of coin out_idx received. Index values can be found via the get_tokens public getter method.
    fn swap(
        e: Env,
        user: Address,
        in_idx: u32,
        out_idx: u32,
        in_amount: u128,
        out_min: u128
    ) -> u128;

    // Estimate amount of coins to retrieve using swap function
    fn estimate_swap(e: Env, in_idx: u32, out_idx: u32, in_amount: u128) -> (u128, i128);

    // Perform an exchange between two coins with strict amount to receive.
    // in_idx: Index value for the coin to send
    // out_idx: Index value of the coin to receive
    // out_amount: Amount of out_idx being exchanged
    // in_max: Maximum amount of in_idx to send
    fn swap_strict_receive(
        e: Env,
        user: Address,
        in_idx: u32,
        out_idx: u32,
        out_amount: u128,
        in_max: u128
    ) -> u128;

    // Estimate amount of coins to retrieve using swap_strict_receive function
    fn estimate_swap_strict_receive(
        e: Env,
        in_idx: u32,
        out_idx: u32,
        out_amount: u128
    ) -> (u128, i128);

    // Remove liquidity
    fn withdraw(e: Env, user: Address, share_amount: u128) -> u128;

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

    fn get_insurance_coverage(e: Env) -> u128;

    // Get dictionary of basic pool information: type, fee, special parameters if any.
    fn get_info(e: Env) -> PoolInfo;

    fn get_privileged_addrs(e: Env) -> Map<Symbol, Vec<Address>>;
}

pub trait AdminInterfaceTrait {
    //  ___      ___       __        __    _____  ___
    // |"  \    /"  |     /""\      |" \  (\"   \|"  \
    //  \   \  //   |    /    \     ||  | |.\\   \    |
    //  /\\  \/.    |   /' /\  \    |:  | |: \.   \\  |
    // |: \.        |  //  __'  \   |.  | |.  \    \. |
    // |.  \    /:  | /   /  \\  \  /\  |\|    \    \ |
    // |___|\__/|___|(___/    \___)(__\_|_)\___|\____\)

    fn rebalance(e: Env, admin: Address);

    fn pay_insurance_claim(e: Env, sender: Address, insurance_vault_amount: u128) -> u128;

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
        emergency_pause_admins: Vec<Address>
    );

    fn set_fee(e: Env, admin: Address, fee_fraction: u32);

    fn set_tier(e: Env, admin: Address, tier: PoolTier);

    fn set_status(e: Env, admin: Address, status: PoolStatus);

    fn set_max_imbalances(
        e: Env,
        admin: Address,
        liquidity_max_imbalance: u128,
        quote_max_insurance: u128
    );

    fn set_expiry(e: Env, admin: Address, expiry_ts: u64);

    //    _______     __       ____  ____   ________  _______  ________
    //   |   __ "\   /""\     ("  _||_ " | /"       )/"     "||"      "\
    //   (. |__) :) /    \    |   (  ) : |(:   \___/(: ______)(.  ___  :)
    //   |:  ____/ /' /\  \   (:  |  | . ) \___  \   \/    |  |: \   ) ||
    //   (|  /    //  __'  \   \\ \__/ //   __/  \\  // ___)_ (| (___\ ||
    //  /|__/ \  /   /  \\  \  /\\ __ //\  /" \   :)(:      "||:       :)
    // (_______)(___/    \___)(__________)(_______/  \_______)(________/

    // Stop pool instantly
    fn kill_deposit(e: Env, admin: Address);
    fn kill_withdraw(e: Env, admin: Address);
    fn kill_swap(e: Env, admin: Address);
    fn kill_claim(e: Env, admin: Address);

    // Resume pool
    fn unkill_deposit(e: Env, admin: Address);
    fn unkill_withdraw(e: Env, admin: Address);
    fn unkill_swap(e: Env, admin: Address);
    fn unkill_claim(e: Env, admin: Address);

    // Get killswitch status
    fn get_is_killed_deposit(e: Env) -> bool;
    fn get_is_killed_withdraw(e: Env) -> bool;
    fn get_is_killed_swap(e: Env) -> bool;
    fn get_is_killed_claim(e: Env) -> bool;
}

pub trait UpgradeableContract {
    // Get contract version
    fn version() -> u32;

    // Upgrade contract with new wasm code
    fn commit_upgrade(
        e: Env,
        admin: Address,
        new_wasm_hash: BytesN<32>,
        new_token_wasm_hash: BytesN<32>
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

pub trait IncentivesTrait {
    // Initialize incentives token address
    fn initialize_incentives_config(e: Env, reward_token: Address);

    // Configure incentives for pool. Every second tps of coins
    // being distributed across all liquidity providers
    // after expired_at timestamp distribution ends
    fn set_incentives_config(e: Env, admin: Address, expired_at: u64, tps: u128);

    // Calculate reward token surplus
    fn get_unused_reward(e: Env) -> u128;

    // Return reward token above the configured amount back to the router
    fn return_unused_reward(e: Env, admin: Address) -> u128;

    // Get incentives status for the pool,
    // including amount available for the user
    fn get_incentives_info(e: Env, user: Address) -> Map<Symbol, i128>;

    // Get amount of reward tokens available for the user to claim.
    fn get_user_reward(e: Env, user: Address) -> u128;

    //
    fn get_user_fees(e: Env, user: Address) -> (u128, u128);

    // Checkpoints the LP fees and reward for the user.
    // Useful when user moves funds by itself to avoid re-entrancy issue.
    // Can be called only by the token contract to notify pool external changes happened.
    fn checkpoint_incentive(e: Env, token_contract: Address, user: Address, user_shares: u128);

    // Checkpoints total working balance and the working balance for the user.
    // Useful when user moves funds by itself to avoid re-entrancy issue.
    // Can be called only by the token contract to notify pool external changes happened.
    fn checkpoint_working_balance(
        e: Env,
        token_contract: Address,
        user: Address,
        user_shares: u128
    );

    // Get total amount of accumulated reward for the pool
    fn get_total_accumulated_reward(e: Env) -> u128;

    // Get total amount of generated plus configured reward for the pool
    fn get_total_configured_reward(e: Env) -> u128;

    // Get total amount of claimed reward for the pool
    fn get_total_claimed_reward(e: Env) -> u128;

    // Claim LP fees and reward as a user.
    // returns amount of tokens rewarded to the user
    fn claim(e: Env, user: Address) -> (u128, u128, u128);
}
