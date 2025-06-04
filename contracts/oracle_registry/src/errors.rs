use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone)]
#[repr(u32)]
pub enum OracleRegistryError {
    #[doc = "OracleRegistryError: MaxIFWithdrawReached"]
    AlreadyInitialized = 15,
    NotAuthorized = 16,
    AdminNotSet = 17,

}
