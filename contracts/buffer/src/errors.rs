use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone)]
#[repr(u32)]
pub enum BufferError {
    #[doc = "BufferError"]
    ReserveMaxBalanceThreshold = 2,
    PayoutTooSoon = 3,
    InsufficentFunds = 4,
    WithdrawalOverMinimumReserve = 5,
    ZeroAmount = 8,
    AlreadyInitialized = 15,

    // Paused Ops
    BufferDepositKilled = 6,
    BufferRequestPayoutKilled = 7,
}
