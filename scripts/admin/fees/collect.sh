# Ensure the script exits on any errors
set -e

# Check if the arguments are provided
# Required: identity_string, pool_swap_fee_address
if [ "$#" -lt 3 ]; then
    echo "Usage: $0 <identity_string> <network> <pool_swap_fee_address>"
    exit 1
fi

IDENTITY_STRING=$1
NETWORK=$2
POOL_SWAP_FEE_ADDR=$3

# Load env vars dynamically
source "$(dirname "${BASH_SOURCE[0]}")/load-env.sh" "$NETWORK"

stellar contract invoke \
    --id $POOL_SWAP_FEE_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase $STELLAR_NETWORK_PASSPHRASE \
    --fee $STELLAR_BASE_FEE \
    -- \
    claim_fees \
    --admin $ADMIN_ADDRESS \
    --token $XLM

echo "Fees collected."
