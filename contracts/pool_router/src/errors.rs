use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone)]
#[repr(u32)]
pub enum PoolRouterError {
    #[doc = "PoolRouterError: PoolNotFound"]
    PoolNotFound = 301,
    BadFee = 302,
    PathIsEmpty = 307,
    TokensAreNotForReward = 308, // unable to find tokens in reward map
    LiquidityNotFilled = 309,    // liquidity info not available yet. run `fill_liquidity` first
    LiquidityAlreadyFilled = 310,
    LiquidityCalculationError = 312,
    RewardsNotConfigured = 313, // unable to find rewards tokens. please run `config_rewards` first
    RewardsAlreadyConfigured = 314,
    DuplicatesNotAllowed = 315,

    TokensNotSorted = 2002,
    InMaxNotSatisfied = 2020,
}
