# Ensure the script exits on any errors
set -e

# Usage
usage() {
    echo "Usage:"
    echo "  $0 <issuer> <network>"
    echo ""
    echo "Example:"
    echo "  $0 admin testnet"
    exit 1
}

# Validate args
if [ "$#" -ne 2 ]; then
    usage
fi

# Parse arguments
ISSUER=$1
NETWORK=$2

# Load env vars dynamically
REPO_ROOT="$(git rev-parse --show-toplevel)"
source "$REPO_ROOT/scripts/load-env.sh" "$NETWORK"

# Get issuer address
ISSUER_ADDRESS=$(soroban keys address "$ISSUER")

# Update the SAC admin to the Pool address
stellar contract invoke \
    --source-account "$ISSUER" \
    --network "$NETWORK" \
    --id $ASSET_SAC_ADDRESS \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE \
    -- \
    set_admin \
    --new_admin $ASSET_POOL_ADDRESS