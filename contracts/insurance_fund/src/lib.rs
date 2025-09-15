#![no_std]

mod contract;
pub mod errors;
mod events;
mod interest;
mod interface;
mod stake;
mod storage;
// mod test; // Temporarily disabled
mod test_permissions;
mod testutils;

#[cfg(test)]
mod tests {
    mod calculation_tests;
    mod security_calculation_tests;
    // mod advanced_attack_tests; // Has Vec usage issues
}

pub use crate::contract::{InsuranceFund, InsuranceFundClient};
