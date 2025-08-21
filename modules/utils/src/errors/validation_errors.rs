use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone)]
#[repr(u32)]
pub enum ValidationError {
    #[doc = "ValidationError"]
    InvalidToken = 801,
    Reentrancy = 802,
}
