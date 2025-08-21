use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone)]
#[repr(u32)]
pub enum OracleError {
    #[doc = "OracleError: OracleNonPositive"]
    OracleNonPositive = 601,
    OracleTooVolatile = 602,
    OracleStaleForPool = 603,
}
