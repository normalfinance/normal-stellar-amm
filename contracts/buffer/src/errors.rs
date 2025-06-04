use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone)]
#[repr(u32)]
pub enum BufferError {
    #[doc = "BufferError: MaxIFWithdrawReached"]
    MaxBalanceHit = 13,
    #[doc = "Trying to remove liqudity too fast after adding it"]
    TryingToRemoveLiquidityTooFast = 14,
    AlreadyInitialized = 15,
    NotAuthorized = 16,
    AdminNotSet = 17,

}
