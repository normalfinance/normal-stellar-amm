pub(crate) mod constant_product_pool {
    soroban_sdk::contractimport!(file = "../../wasm/pool.wasm");
}
pub(crate) mod swap_fee {
    soroban_sdk::contractimport!(file = "../../wasm/pool_swap_fee.wasm");
}
pub(crate) mod router {
    soroban_sdk::contractimport!(file = "../../wasm/pool_router.wasm");
}

pub(crate) mod lp_token {
    soroban_sdk::contractimport!(file = "../../wasm/soroban_token_contract.wasm");
}
