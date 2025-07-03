# Ensure the script exits on any errors
set -e

# Check if the argument is provided
if [ -z "$1" ]; then
    echo "Usage: $0 <identity_string>"
    exit 1
fi

IDENTITY_STRING=$1
NETWORK="testnet"

# Config
CONTRACT_TYPE="..."
CONTRACT_ADDR="..."

echo "Build and optimize the contracts..."

# make build >/dev/null
task build
cd target/wasm32v1-none/release

echo "Contracts compiled."
echo "Optimize contracts..."

soroban contract optimize --wasm $CONTRACT_TYPE.wasm

echo "Contracts optimized."

NEW_WASM_HASH=$(soroban contract upload \
    --wasm $CONTRACT_TYPE.optimized.wasm \
    --source $IDENTITY_STRING \
    --network $NETWORK)

stellar contract invoke \
    --id $CONTRACT_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    commit_upgrade \
    --admin $ADMIN_ADDRESS \
    --new_wasm_hash $NEW_WASM_HASH

echo "$CONTRACT_TYPE upgrade committed."
