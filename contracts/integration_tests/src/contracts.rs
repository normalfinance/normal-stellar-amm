pub(crate) mod constant_product_pool {
    soroban_sdk::contractimport!(file = "../../wasm/soroban_liquidity_pool_contract.wasm");
}
pub(crate) mod pool_plane {
    soroban_sdk::contractimport!(file = "../../wasm/soroban_liquidity_pool_plane_contract.wasm");
}
pub(crate) mod swap_fee {
    soroban_sdk::contractimport!(
        file = "../../wasm/soroban_liquidity_pool_provider_swap_fee_contract.wasm"
    );
}
pub(crate) mod swap_fee_factory {
    soroban_sdk::contractimport!(
        file = "../../wasm/soroban_liquidity_pool_provider_swap_fee_factory_contract.wasm"
    );
}
pub(crate) mod router {
    soroban_sdk::contractimport!(file = "../../wasm/soroban_liquidity_pool_router_contract.wasm");
}

pub(crate) mod lp_token {
    soroban_sdk::contractimport!(file = "../../wasm/soroban_token_contract.wasm");
}
