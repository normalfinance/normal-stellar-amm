# Ensure the script exits on any errors
set -e

# Check if the argument is provided
if [ "$#" -lt 3 ]; then
    echo "Usage: $0 <identity_string> <network> <pool_addr>"
    exit 1
fi

IDENTITY_STRING=$1
NETWORK=$2
POOL_ADDR=$3

# Load env vars dynamically
source "$(dirname "${BASH_SOURCE[0]}")/load-env.sh" "$NETWORK"

echo $STELLAR_RPC_URL
echo "$STELLAR_NETWORK_PASSPHRASE"

echo "Build and optimize the contracts..."

task build
cd target/wasm32v1-none/release

echo "Contracts compiled."
echo "Optimize contracts..."

soroban contract optimize --wasm rewards_gauge.wasm

echo "Contracts optimized."

# Fetch the admin's address
ADMIN_ADDRESS=$(soroban keys address $IDENTITY_STRING)

REWARDS_GAUGE_ADDR=$(soroban contract deploy \
    --wasm rewards_gauge.optimized.wasm \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE \
    -- --pool $POOL_ADDR --reward_token $XLM_ADDRESS
    )

echo "#############################"

echo "Initialization complete!"
# echo "XLM address: $XLM_ADDRESS"

echo "Pool Router Contract address: $REWARDS_GAUGE_ADDR"