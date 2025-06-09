use incentives::Incentives;
use soroban_sdk::Env;

use crate::storage::get_pool;

// page size of 100 is optimal since 8 bytes key + 16 bytes value * 100 = 2400 bytes per page
// it gives us up to 26 aggregation layers
#[cfg(not(test))]
pub(crate) const PAGE_SIZE: u64 = 100;

#[cfg(test)]
pub(crate) const PAGE_SIZE: u64 = 5;

pub(crate) fn get_incentives_manager(e: &Env) -> Incentives {
    let pool = get_pool(e);
    Incentives::new(e, PAGE_SIZE, pool.token_a, pool.token_b)
}
