mod liquidity_calculator_client {
    soroban_sdk::contractimport!(
        file = "../../../target/wasm32v1-none/release/liquidity_calculator.wasm"
    );
}

pub use liquidity_calculator_client::Client as LiquidityCalculatorClient;
