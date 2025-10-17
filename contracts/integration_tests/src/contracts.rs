pub(crate) mod constant_product_pool {
    soroban_sdk::contractimport!(file = "../contracts/pool.wasm");
}
pub(crate) mod liquidity_calculator {
    soroban_sdk::contractimport!(file = "../contracts/liquidity_calculator.wasm");
}
pub(crate) mod pool_plane {
    soroban_sdk::contractimport!(file = "../contracts/pool_plane.wasm");
}
pub(crate) mod router {
    soroban_sdk::contractimport!(file = "../contracts/pool_router.wasm");
}
pub(crate) mod elastic_pool {
    soroban_sdk::contractimport!(file = "../contracts/pool_elastic.wasm");
}
pub(crate) mod token_share {
    soroban_sdk::contractimport!(file = "../contracts/token_share.wasm");
}

pub(crate) mod config_storage {
    soroban_sdk::contractimport!(file = "../contracts/config_storage.wasm");
}

pub(crate) mod rewards_gauge {
    soroban_sdk::contractimport!(file = "../contracts/rewards_gauge.wasm");
}
