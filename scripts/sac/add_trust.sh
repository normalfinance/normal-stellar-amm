# Ensure the script exits on any errors
set -e

# Usage
usage() {
    echo "Usage:"
    echo "  $0 <identity> <network> <issuer> <symbol>"
    echo ""
    echo "Example:"
    echo "  $0 admin admin testnet nSOL"
    exit 1
}

# Validate args
if [ "$#" -ne 4 ]; then
    usage
fi

# Parse arguments
IDENTITY_STRING=$1
NETWORK=$2
ISSUER=$3
SYMBOL=$4

# Load env vars dynamically
REPO_ROOT="$(git rev-parse --show-toplevel)"
source "$REPO_ROOT/scripts/load-env.sh" "$NETWORK"

# Issue an asset by creating a trustline
stellar tx new change-trust \
    --source-account "$IDENTITY_STRING" \
    --network "$NETWORK" \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE \
    --line "$SYMBOL:$ISSUER"