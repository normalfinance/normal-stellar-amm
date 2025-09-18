#![no_std]

mod contract;
pub mod errors;
mod events;
mod interest;
mod interface;
mod reserve;
mod stake;
mod storage;
mod test;
mod test_permissions;
mod testutils;

#[cfg(test)]
mod tests {
    pub mod calculation_tests;
    pub mod security_calculation_tests;
    pub mod advanced_attack_tests;
}

pub use crate::contract::{InsuranceFund, InsuranceFundClient};
