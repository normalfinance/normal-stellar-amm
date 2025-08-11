use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone)]
#[repr(u32)]
pub enum PoolSwapFeeError {
    #[doc = "PoolSwapFeeError"]
    OutMinNotSatisfied = 2006,
    InvalidFeeCalculation = 2007,
}
