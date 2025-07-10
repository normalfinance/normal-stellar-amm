use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone)]
#[repr(u32)]
pub enum AccessControlError {
    #[doc = "AccessControlError"]
    RoleNotFound = 101,
    Unauthorized = 102,
    AdminAlreadySet = 103,
    BadRoleUsage = 104,
    MustBeNewAddress = 105,

    // transfer ownership errors
    AnotherActionActive = 2906,
    NoActionActive = 2907,
    ActionNotReadyYet = 2908,
}
