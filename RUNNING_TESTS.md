# Running Tests (Quick Guide)

## Prerequisites
- Rust toolchain installed (stable)
- For integration-tests only: wasm32 target (run: `rustup target add wasm32-unknown-unknown`)

## Run unit tests per crate
- Pool:
```bash
cargo test -p pool --lib
```
- Insurance Fund:
```bash
cargo test -p insurance-fund --lib
```
- Oracle Registry:
```bash
cargo test -p oracle-registry --lib
```
- Pool Swap Fee:
```bash
cargo test -p pool-swap-fee --lib
```

Tips:
- Show logs: append `-- --nocapture`
- Filter a test: `cargo test -p pool --lib test_delta_a_precision_attack`

## Integration-tests (optional)
Integration tests require prebuilt WASM artifacts under `./wasm/`.

1) Build WASM for each contract:
```bash
cargo build --release --target wasm32-unknown-unknown -p pool
cargo build --release --target wasm32-unknown-unknown -p pool-swap-fee
cargo build --release --target wasm32-unknown-unknown -p pool-router
cargo build --release --target wasm32-unknown-unknown -p oracle-registry
cargo build --release --target wasm32-unknown-unknown -p pool-plane
cargo build --release --target wasm32-unknown-unknown -p liquidity-calculator
cargo build --release --target wasm32-unknown-unknown -p buffer
cargo build --release --target wasm32-unknown-unknown -p insurance-fund
```
2) Copy outputs to `./wasm/` with expected names:
```bash
mkdir -p wasm
cp target/wasm32-unknown-unknown/release/pool.wasm                 wasm/pool.wasm
cp target/wasm32-unknown-unknown/release/pool_swap_fee.wasm        wasm/pool_swap_fee.wasm
cp target/wasm32-unknown-unknown/release/pool_router.wasm          wasm/pool_router.wasm
cp target/wasm32-unknown-unknown/release/oracle_registry.wasm      wasm/oracle_registry.wasm
cp target/wasm32-unknown-unknown/release/pool_plane.wasm           wasm/pool_plane.wasm
cp target/wasm32-unknown-unknown/release/liquidity_calculator.wasm wasm/liquidity_calculator.wasm
cp target/wasm32-unknown-unknown/release/buffer.wasm               wasm/buffer.wasm
cp target/wasm32-unknown-unknown/release/insurance_fund.wasm       wasm/insurance_fund.wasm
```
3) Run integration tests:
```bash
cargo test -p integration-tests --lib
```

## Known caveats (current codebase)
- Oracle Registry tests may fail due to strict timestamp validation ("published timestamp cannot be in the future"). Ensure ledger time progression or relax test expectations.
- Pool tests that depend on oracle setup can fail without seeded price history and appropriate guard rails in test utils.
- Insurance Fund has a known missing clamp in `calculate_utilization`; related tests will fail until clamping is implemented.
