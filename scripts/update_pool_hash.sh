# Ensure the script exits on any errors
set -e

# Check if the argument is provided
if [ -z "$1" ]; then
    echo "Usage: $0 <identity_string> <network>"
    exit 1
fi

IDENTITY_STRING=$1
NETWORK=$2

echo "Build and optimize the contracts..."

# make build >/dev/null
task build
cd target/wasm32v1-none/release

echo "Contracts compiled."
echo "Optimize contracts..."

soroban contract optimize --wasm pool.wasm

echo "Contracts optimized."

# Fetch the admin's address
ADMIN_ADDRESS=$(soroban keys address $IDENTITY_STRING)


# Continue with the rest of the deployments
POOL_WASM_HASH=$(soroban contract upload \
    --wasm pool.optimized.wasm \
    --source $IDENTITY_STRING \
--network $NETWORK)

echo "Pool contracts deployed."

POOL_ROUTER_ADDR="CDE375VH2EQFLLCCZFSMLLOCNQJYSGQHUP23LNYAGAUX5PKOM3HLRLGR"

stellar contract invoke \
    --id $POOL_ROUTER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    set_pool_hash \
    --admin $ADMIN_ADDRESS \
    --new_hash $POOL_WASM_HASH

echo "#############################"

echo "Update complete!"
