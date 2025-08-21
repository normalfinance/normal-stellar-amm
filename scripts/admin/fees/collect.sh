# Ensure the script exits on any errors
set -e

# Check if the arguments are provided
# Required: identity_string,
if [ "$#" -lt 3 ]; then
    echo "Usage: $0 <identity_string> <network>"
    exit 1
fi

IDENTITY_STRING=$1
NETWORK=$2

# Load env vars dynamically
REPO_ROOT="$(git rev-parse --show-toplevel)"
source "$REPO_ROOT/scripts/load-env.sh" "$NETWORK"

stellar contract invoke \
    --id $POOL_SWAP_FEE_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE \
    -- \
    claim_fees \
    --admin $ADMIN_ADDRESS \
    --token $XLM

echo "Fees collected."
