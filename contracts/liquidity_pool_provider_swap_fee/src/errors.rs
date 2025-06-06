use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone)]
#[repr(u32)]
pub enum Error {
    #[doc = "PoolProviderSwapFeeError"]
    Unauthorized = 102,
    InsuranceFractionTooLow = 103, // TODO: is 103 taken?
    PathIsEmpty = 307,
    OutMinNotSatisfied = 2006,
    InMaxNotSatisfied = 2020,
    FeeFractionTooHigh = 2904,
}
