# Ensure the script exits on any errors
set -e

# Usage
usage() {
    echo "Usage:"
    echo "  $0 <identity_string> <network> <sac> <amount> <to>"
    echo ""
    echo "Example:"
    echo "  $0 admin testnet"
    exit 1
}

# Validate args
if [ "$#" -ne 5 ]; then
    usage
fi

# Parse arguments
IDENTITY_STRING=$1
NETWORK=$2
SAC=$3
AMOUNT=$4
TO=$5

# Load env vars dynamically
REPO_ROOT="$(git rev-parse --show-toplevel)"
source "$REPO_ROOT/scripts/load-env.sh" "$NETWORK"

# Get admin address
# ISSUER_ADDRESS=$(soroban keys address "$ISSUER")

stellar contract invoke \
    --id $SAC \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE \
    -- \
    mint \
    --to $TO \
    --amount $AMOUNT
