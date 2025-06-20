use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone)]
#[repr(u32)]
pub enum Error {
    #[doc = "PoolProviderSwapFeeError"]
    OutMinNotSatisfied = 2006,
}
