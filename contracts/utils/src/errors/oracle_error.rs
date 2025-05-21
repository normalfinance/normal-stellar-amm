use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone)]
#[repr(u32)]
pub enum OracleError {
    #[doc = "OracleError: OracleNonPositive"]
    OracleNonPositive = 601,
    OracleTooVolatile = 602,
    OracleTooUncertain = 603,
    OracleStaleForMargin = 604,
    OracleInsufficientDataPoints = 605,
    OracleStaleForAMM = 606,
}
