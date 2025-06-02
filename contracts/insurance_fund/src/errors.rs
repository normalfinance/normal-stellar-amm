use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone)]
#[repr(u32)]
pub enum InsuranceFundError {
    #[doc = "InsuranceFundError: MaxIFWithdrawReached"]
    MaxIFWithdrawReached = 0,
    NoIFWithdrawAvailable = 1,
    InvalidIFUnstake = 2,
    InvalidIFUnstakeSize = 3,
    InvalidIFUnstakeCancel = 4,
    InvalidIFForNewStakes = 5,
    InvalidIFRebase = 6,
    InvalidInsuranceUnstakeSize = 7,
    InsuranceFundOperationPaused = 8,
    IFWithdrawRequestInProgress = 9,
    NoIFWithdrawRequestInProgress = 10,
    IFWithdrawRequestTooSmall = 11,
    InvalidIFSharesDetected = 12,
    InsufficientIFShares = 13,
    #[doc = "Trying to remove liqudity too fast after adding it"]
    TryingToRemoveLiquidityTooFast = 14,
    AlreadyInitialized = 15,
    NotAuthorized = 16,
    AdminNotSet = 17,

    FundDepositKilled = 18,
    FundRequestWithdrawKilled = 19,
    FundWithdrawKilled = 20,
}
