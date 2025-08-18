# Ensure the script exits on any errors
set -e

# Load environment variables from .env file
source .env

# Usage
usage() {
    echo "Usage:"
    echo "  $0 <issuer> <network> <symbol>"
    echo ""
    echo "Example:"
    echo "  $0 admin testnet"
    exit 1
}

# Validate args
if [ "$#" -ne 3 ]; then
    usage
fi

# Parse arguments
ISSUER=$1
NETWORK=$2
SYMBOL=$3

# Get admin address
ISSUER_ADDRESS=$(soroban keys address "$ISSUER")

# Deploy the built-in SAC contract for the new asset
stellar contract asset deploy \
    --source "$ISSUER" \
    --network "$NETWORK" \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase $STELLAR_NETWORK_PASSPHRASE \
    --fee $STELLAR_BASE_FEE \
    --asset "${SYMBOL}:${ISSUER_ADDRESS}"