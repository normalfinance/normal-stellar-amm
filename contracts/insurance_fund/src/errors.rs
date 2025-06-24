use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone)]
#[repr(u32)]
pub enum InsuranceFundError {
    #[doc = "InsuranceFundError"]
    MaxIFWithdrawReached = 0,
    NoIFWithdrawAvailable = 1,
    InvalidIFUnstake = 2,
    InvalidIFUnstakeSize = 3,
    InvalidIFRebase = 6,
    InvalidInsuranceUnstakeSize = 7,
    IFWithdrawRequestInProgress = 9,
    NoIFWithdrawRequestInProgress = 10,
    IFWithdrawRequestTooSmall = 11,
    InvalidIFSharesDetected = 12,
    InsufficientIFShares = 13,
    TryingToRemoveLiquidityTooFast = 14,
    AlreadyInitialized = 15,
    NotAuthorized = 16,
    AdminNotSet = 17,
    InsufficientCollateral = 18,
    InvalidIFDetected = 19,
    TooMuchInsurance = 20,
    
    InvalidOptimalUtilization = 21,

    // paused ops
    FundDepositKilled = 30,
    FundRequestWithdrawKilled = 31,
    FundWithdrawKilled = 32,
}
