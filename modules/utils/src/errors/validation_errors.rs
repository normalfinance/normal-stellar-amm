use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone)]
#[repr(u32)]
pub enum ValidationError {
    #[doc = "ValidationError"]
    InvalidToken = 801,
    InvalidPercentage = 802,
    Reentrancy = 803,
    ZeroAmount = 804,
}
