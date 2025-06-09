#![no_std]

use soroban_sdk::{ Address, Env };

pub mod errors;
pub mod events;
pub mod manager;
pub mod storage;

pub use manager::Manager;
pub use storage::Storage;
pub use utils;

#[derive(Clone)]
pub struct IncentivesConfig {
    page_size: u64,
    token_a: Address,
    token_b: Address,
}

#[derive(Clone)]
pub struct Incentives {
    env: Env,
    config: IncentivesConfig,
}

impl Incentives {
    #[inline(always)]
    pub fn new(env: &Env, page_size: u64, token_a: Address, token_b: Address) -> Incentives {
        Incentives {
            env: env.clone(),
            config: IncentivesConfig { page_size, token_a, token_b },
        }
    }

    pub fn storage(&self) -> Storage {
        Storage::new(&self.env)
    }

    pub fn manager(&self) -> Manager {
        Manager::new(&self.env, self.storage(), &self.config)
    }
}
