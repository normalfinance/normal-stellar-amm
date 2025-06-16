#![no_std]

mod contract;
pub mod errors;
mod events;
mod interface;
mod oracle;
mod storage;
mod storage_types;
mod test;
mod test_permissions;
mod testutils;

pub use contract::{ OracleRegistry, OracleRegistryClient };
