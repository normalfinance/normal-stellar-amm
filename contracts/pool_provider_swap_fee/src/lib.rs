#![no_std]

mod contract;
mod errors;
mod events;
mod incentives;
mod interface;
mod storage;
mod test;
mod testutils;

pub use crate::contract::{ProviderSwapFeeCollector, ProviderSwapFeeCollectorClient};
