# Create bindings directory
mkdir -p bindings

# Build all
task build

# Generate bindings for each contract
soroban contract bindings typescript --overwrite --wasm target/wasm32v1-none/release/soroban_token_contract.wasm --output-dir bindings/soroban_token_contract

soroban contract bindings typescript --overwrite --wasm target/wasm32v1-none/release/token_share.wasm --output-dir bindings/token_share

soroban contract bindings typescript --overwrite --wasm target/wasm32v1-none/release/config_storage.wasm --output-dir bindings/config_storage

soroban contract bindings typescript --overwrite --wasm target/wasm32v1-none/release/rewards_gauge.wasm --output-dir bindings/rewards_gauge

soroban contract bindings typescript --overwrite --wasm target/wasm32v1-none/release/pool_plane.wasm --output-dir bindings/pool_plane

soroban contract bindings typescript --overwrite --wasm target/wasm32v1-none/release/liquidity_calculator.wasm --output-dir bindings/liquidity_calculator

soroban contract bindings typescript --overwrite --wasm target/wasm32v1-none/release/pool.wasm --output-dir bindings/pool

soroban contract bindings typescript --overwrite --wasm target/wasm32v1-none/release/pool_elastic.wasm --output-dir bindings/pool_elastic

soroban contract bindings typescript --overwrite --wasm target/wasm32v1-none/release/pool_router.wasm --output-dir bindings/pool_router

# soroban contract bindings typescript --overwrite --wasm target/wasm32v1-none/release/sink.wasm --output-dir bindings/sink