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
mod tests;

pub use contract::{OracleRegistry, OracleRegistryClient};
