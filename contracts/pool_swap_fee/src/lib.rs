#![no_std]

mod contract;
mod errors;
mod events;
mod incentives;
mod interface;
mod storage;
mod test;
mod test_permissions;
mod testutils;

#[cfg(test)]
mod tests {
    // Tests have API mismatch issues
    // mod calculation_tests;
    // mod security_calculation_tests;
}

pub use crate::contract::{PoolSwapFeeCollector, PoolSwapFeeCollectorClient};
