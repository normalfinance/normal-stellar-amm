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
    OracleExists = 19,
    PriceOverrideLimitExceeded = 20,
    OracleNotFound = 21,
    OracleFrozen = 22,
}
