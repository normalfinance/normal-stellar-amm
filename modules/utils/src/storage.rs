use soroban_sdk::{ contracttype, Address, BytesN, String, Symbol, Vec };

#[contracttype]
#[derive(Default, Clone, Copy, Debug)]
pub struct OraclePriceData {
    pub price: u128,
    pub delay: u64,
}

#[contracttype]
#[derive(Default, Clone, PartialEq)]
pub enum PoolStatus {
    // warm up period for initialization, fills are paused
    #[default]
    Initialized,
    // all operations allowed
    Active,
    //
    Frozen,
    // fills only able to reduce liability
    ReduceOnly,
    // market has determined settlement price and positions are expired must be settled
    Settlement,
    // market has no remaining participants
    Delisted,
}

#[contracttype]
#[derive(Clone, Copy, PartialEq, Debug, Eq, PartialOrd, Ord, Default)]
pub enum PoolTier {
    // max insurance capped at A level
    A,
    // max insurance capped at B level
    B,
    // max insurance capped at C level
    C,
    // no insurance
    Speculative,
    // no insurance, another tranches below
    #[default]
    HighlySpeculative,
    // no insurance, only single position allowed
    Isolated,
}

impl PoolTier {
    pub fn is_as_safe_as(&self, other: &PoolTier) -> bool {
        // Pool Tier A safest
        self <= other
    }
}

#[contracttype]
#[derive(Clone)]
pub struct TokenInitInfo {
    // The hash of the liquidity pool token contract.
    pub token_wasm_hash: BytesN<32>,
    pub name: String,
    pub symbol: String,
}

#[contracttype]
#[derive(Clone)]
pub struct PrivilegedAddresses {
    pub emergency_admin: Address,
    pub rewards_admin: Address,
    pub operations_admin: Address,
    pub pause_admin: Address,
    pub emergency_pause_admins: Vec<Address>,
}

#[contracttype]
#[derive(Clone)]
pub struct OraclePair {
    pub base_oracle: Address,
    pub quote_oracle: Address,
}

#[contracttype]
#[derive(Clone)]
pub struct RewardConfig {
    // The address of the reward token.
    pub reward_token: Address,
}

#[contracttype]
#[derive(Clone)]
pub struct InitializeParams {
    // The address of the admin user.
    pub admin: Address,
    pub privileged_addrs: PrivilegedAddresses,
    // The address of the Router.
    pub router: Address,
    // The address of the Oracle Registry.
    pub oracle_registry: Address,
    // The
    pub base_asset_id: Symbol,
    //
    pub quote_asset_id: Symbol,
    //
    pub asset: Address,
    pub lp_token_info: TokenInitInfo,
    // A vector of token addresses.
    pub tokens: Vec<Address>,
    // The fee fraction for the pool.
    pub fee_fraction: u32,
    //
    pub tier: PoolTier,
    //
    pub quote_max_insurance: u128,
}

#[contracttype]
#[derive(Clone)]
pub struct InitializeAllParams {
    pub base: InitializeParams,
    pub reward_config: RewardConfig,
}

//  Queries

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AddressAndAmount {
    // Address of the asset
    pub address: Address,
    // The total amount of those tokens in the pool
    pub amount: u128,
}

// This struct is used to return a query result with the total amount of LP tokens and assets in a specific pool.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PoolResponse {
    // The asset A in the pool together with asset amounts
    pub asset_a: AddressAndAmount,
    // The asset B in the pool together with asset amounts
    pub asset_b: AddressAndAmount,
    // The total amount of LP tokens currently issued
    pub asset_lp_share: AddressAndAmount,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PoolInfo {
    pub pool_address: Address,
    pub pool_response: PoolResponse,
    pub total_fee_bps: u32,
}

//     ______     _______        __       ______   ___       _______
//    /    " \   /"      \      /""\     /" _  "\ |"  |     /"     "|
//   // ____  \ |:        |    /    \   (: ( \___)||  |    (: ______)
//  /  /    ) :)|_____/   )   /' /\  \   \/ \     |:  |     \/    |
// (: (____/ //  //      /   //  __'  \  //  \ _   \  |___  // ___)_
//  \        /  |:  __   \  /   /  \\  \(:   _) \ ( \_|:  \(:      "|
//   \"_____/   |__|  \___)(___/    \___)\_______) \_______)\_______)
//   _______    _______   _______   __      ________  ___________  _______   ___  ___
//  /"      \  /"     "| /" _   "| |" \    /"       )("     _   ")/"      \ |"  \/"  |
// |:        |(: ______)(: ( \___) ||  |  (:   \___/  )__/  \\__/|:        | \   \  /
// |_____/   ) \/    |   \/ \      |:  |   \___  \       \\_ /   |_____/   )  \\  \/
//  //      /  // ___)_  //  \ ___ |.  |    __/  \\      |.  |    //      /   /   /
// |:  __   \ (:      "|(:   _(  _|/\  |\  /" \   :)     \:  |   |:  __   \  /   /
// |__|  \___) \_______) \_______)(__\_|_)(_______/       \__|   |__|  \___)|___/

#[derive(Clone, Debug)]
#[contracttype]
pub struct OracleInfo {
    pub oracle_address: Address,
    pub asset: Address,
    pub decimals: u32, // Optional: for price normalization
    pub frozen: bool,
    pub last_updated: u64,
}
