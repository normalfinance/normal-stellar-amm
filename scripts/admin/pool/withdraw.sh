# Ensure the script exits on any errors
set -e

# Check if the arguments are provided
# Required: identity_string, network, pool_router_address, asset, share_amount
if [ "$#" -lt 5 ]; then
    echo "Usage: $0 <identity_string> <network> <pool_router_address> <asset> <share_amount>"
    exit 1
fi

IDENTITY_STRING=$1
NETWORK=$2
POOL_ROUTER_ADDR=$3
ASSET=$4
SHARE_AMOUNT=$5

# Load env vars dynamically
source "$(dirname "${BASH_SOURCE[0]}")/load-env.sh" "$NETWORK"

# Fetch the admin's address
ADMIN_ADDRESS=$(soroban keys address $IDENTITY_STRING)

# Check if timestamp is a valid number (only digits)
if ! [[ "$SHARE_AMOUNT" =~ ^[0-9]+$ ]]; then
    echo "Error: SHARE_AMOUNT is not a valid number."
    exit 1
fi

echo "Withdraw liquidity into pool..."

stellar contract invoke \
    --id $POOL_ROUTER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase $STELLAR_NETWORK_PASSPHRASE \
    --fee $STELLAR_BASE_FEE \
    -- \
    withdraw \
    --user $ADMIN_ADDRESS \
    --asset $ASSET \
    --share_amount $SHARE_AMOUNT

echo "Pool withdrawal complete."
