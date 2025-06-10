use core::cmp::max;

use soroban_sdk::{contracttype, log, Address, Env};

#[contracttype]
#[derive(Default, Clone, Copy, Debug)]
pub struct OraclePriceData {
    pub price: u128,
    pub delay: u64,
}
