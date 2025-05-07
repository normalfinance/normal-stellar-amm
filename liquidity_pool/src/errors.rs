use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone)]
#[repr(u32)]
pub enum LiquidityPoolError {
    #[doc = "LiquidityPoolError: AlreadyInitialized"]
    AlreadyInitialized = 201,
    PlaneAlreadyInitialized = 202,
    RewardsAlreadyInitialized = 203,
    InvariantDoesNotHold = 204,
    PoolDepositKilled = 205,
    PoolSwapKilled = 206,
    PoolClaimKilled = 207,
    FutureShareIdNotSet = 208,
    // pool specific validation errors
}

#[contracterror]
#[derive(Copy, Clone)]
#[repr(u32)]
pub enum LiquidityPoolValidationError {
    #[doc = "LiquidityPoolValidationError: WrongInputVecSize"]
    WrongInputVecSize = 2001,
    FeeOutOfBounds = 2003,
    AllCoinsRequired = 2004,
    InMinNotSatisfied = 2005,
    OutMinNotSatisfied = 2006,
    CannotSwapSameToken = 2007,
    InTokenOutOfBounds = 2008,
    OutTokenOutOfBounds = 2009,
    EmptyPool = 2010,
    InvalidDepositAmount = 2011,
    AdminFeeOutOfBounds = 2012,
    UnknownPoolType = 2013,
    ZeroSharesBurned = 2014,
    TooManySharesBurned = 2015,
    CannotComparePools = 2017,
    ZeroAmount = 2018,
    InsufficientBalance = 2019,
    InMaxNotSatisfied = 2020,
}
