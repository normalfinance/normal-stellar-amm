use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone)]
#[repr(u32)]
pub enum RewardsError {
    #[doc = "RewardsError"]
    PastTimeNotAllowed = 701,
    SameIncentivesConfig = 702,
}
