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
source "$(dirname "${BASH_SOURCE[0]}")/load-env.sh" "$NETWORK"

# Get admin address
ISSUER_ADDRESS=$(soroban keys address "$ISSUER")

# Issue an asset by creating a trustline
stellar tx new change-trust \
    --source-account "$ISSUER" \
    --network "$NETWORK" \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE 