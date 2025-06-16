use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone)]
#[repr(u32)]
pub enum OracleRegistryError {
    #[doc = "OracleRegistryError: MaxIFWithdrawReached"]
    AlreadyInitialized = 15,
    NotAuthorized = 16,
    AdminNotSet = 17,
    InvalidPrice = 18,
    OracleNotRegistered = 19,
    PriceOverrideLimitExceeded = 20,
    OracleNotFound = 21,
    OracleFrozen = 22,
    OracleInvalid = 23,
    PriceOverrideTooSoon = 24,
    OracleAlreadyRegistered = 25
}
