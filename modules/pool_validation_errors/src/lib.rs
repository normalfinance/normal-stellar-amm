#![no_std]

use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone)]
#[repr(u32)]
pub enum PoolValidationError {
    WrongInputVecSize = 2001,
    FeeOutOfBounds = 2003,
    AllCoinsRequired = 2004,
    InMinNotSatisfied = 2005,
    OutMinNotSatisfied = 2006,
    CannotSwapSameToken = 2007,
    InTokenOutOfBounds = 2008,
    OutTokenOutOfBounds = 2009,
    EmptyPool = 2010,
    InvalidDepositAmount = 2011,
    AdminFeeOutOfBounds = 2012,
    UnknownPoolType = 2013,
    ZeroSharesBurned = 2014,
    TooManySharesBurned = 2015,
    CannotComparePools = 2017,
    ZeroAmount = 2018,
    InsufficientBalance = 2019,
    InMaxNotSatisfied = 2020,
    // added
    WithdrawExceedsMinLiquidity = 2021,
    RebaseTooSoon = 2022,
    CircuitBreaker = 2023,
    InvalidOracle = 2024,
}
