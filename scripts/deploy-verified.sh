# Ensure the script exits on any errors
set -e

# TODO: this script is not complete!

# Check if the argument is provided
if [ "$#" -lt 3 ]; then
    echo "Usage: $0 <identity_string> <network> <version>"
    exit 1
fi

IDENTITY_STRING=$1
NETWORK=$2
VERSION_TAG=$3

# Load env vars dynamically
source "$(dirname "${BASH_SOURCE[0]}")/load-env.sh" "$NETWORK"

# Insurance Fund
soroban contract deploy \
    --wasm "insurance-fund_$VERSION_TAG.wasm"\
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase $STELLAR_NETWORK_PASSPHRASE \
    --fee $STELLAR_BASE_FEE

# Oracle Registry
soroban contract deploy \
    --wasm "oracle-registry_$VERSION_TAG.wasm"\
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase $STELLAR_NETWORK_PASSPHRASE \
    --fee $STELLAR_BASE_FEE

# Pool Plane
soroban contract deploy \
    --wasm "pool-plane_$VERSION_TAG.wasm" \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase $STELLAR_NETWORK_PASSPHRASE \
    --fee $STELLAR_BASE_FEE

# Liquidity Calculator
soroban contract deploy \
    --wasm "liquidity-calculator_$VERSION_TAG.wasm" \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase $STELLAR_NETWORK_PASSPHRASE \
    --fee $STELLAR_BASE_FEE

# Pool Router
soroban contract deploy \
    --wasm "pool-router_$VERSION_TAG.wasm" \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase $STELLAR_NETWORK_PASSPHRASE \
    --fee $STELLAR_BASE_FEE

# Pool Swap Fee
soroban contract deploy \
    --wasm "pool-swap-fee_$VERSION_TAG.wasm" \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase $STELLAR_NETWORK_PASSPHRASE \
    --fee $STELLAR_BASE_FEE