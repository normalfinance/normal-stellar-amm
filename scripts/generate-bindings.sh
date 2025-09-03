# Create bindings directory
mkdir -p bindings

# Build all
task build

# Generate bindings for each contract
soroban contract bindings typescript --overwrite --wasm target/wasm32v1-none/release/soroban_token_contract.wasm --output-dir bindings/soroban_token_contract

soroban contract bindings typescript --overwrite --wasm target/wasm32v1-none/release/pool_token.wasm --output-dir bindings/pool_token

soroban contract bindings typescript --overwrite --wasm target/wasm32v1-none/release/oracle_registry.wasm --output-dir bindings/oracle_registry

soroban contract bindings typescript --overwrite --wasm target/wasm32v1-none/release/pool_plane.wasm --output-dir bindings/pool_plane

soroban contract bindings typescript --overwrite --wasm target/wasm32v1-none/release/liquidity_calculator.wasm --output-dir bindings/liquidity_calculator

soroban contract bindings typescript --overwrite --wasm target/wasm32v1-none/release/pool.wasm --output-dir bindings/pool

soroban contract bindings typescript --overwrite --wasm target/wasm32v1-none/release/pool_router.wasm --output-dir bindings/pool_router

soroban contract bindings typescript --overwrite --wasm target/wasm32v1-none/release/insurance_fund.wasm --output-dir bindings/insurance_fund