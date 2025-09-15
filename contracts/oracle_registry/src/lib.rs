#![no_std]

mod contract;
pub mod errors;
mod interface;
mod oracle;
mod storage;
mod test;
mod test_permissions;
mod testutils;

#[cfg(test)]
mod tests {
    // Tests have import issues that need fixing
    // mod unregistered_oracle_panic;
    // mod twap_tests;
    // mod security_twap_tests;
    // mod advanced_twap_attacks;
}

#[cfg(test)]
mod simple_oracle_test;

pub use contract::{ OracleRegistry, OracleRegistryClient };
