use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone)]
#[repr(u32)]
pub enum BufferError {
    #[doc = "BufferError"]
    MaxBalanceHit = 13,
    AlreadyInitialized = 15,
    NotAuthorized = 16,

}
