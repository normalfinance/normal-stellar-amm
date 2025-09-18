#![no_std]

mod contract;
pub mod errors;
mod events;
mod incentives;
mod interface;
mod plane;
mod plane_interface;
mod pool;
mod storage;
mod test;
mod test_permissions;
mod testutils;
pub mod token;

#[cfg(test)]
mod tests {
    pub mod security_calculation_tests;
    pub mod advanced_attack_tests;
    pub mod calculation_tests;
    pub mod critical_security_tests;
    pub mod first_depositor_attack_tests;
    pub mod liquidity_crisis_tests;
    pub mod pool_security_tests;
    pub mod production_security_suite;
    pub mod reentrancy_poc;
    pub mod share_dilution_poc;
    pub mod share_dilution_simple_poc;
    pub mod simple_working_tests;
    pub mod twap_manipulation_tests;
    pub mod volume_fee_manipulation_tests;
}

pub use contract::{Pool, PoolClient};
