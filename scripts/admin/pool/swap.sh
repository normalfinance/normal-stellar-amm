# Ensure the script exits on any errors
set -e

# Check if the arguments are provided
# Required: identity_string, network, pool_router_address, asset, direction, amount, out_minimum
if [ "$#" -lt 6 ]; then
    echo "Usage: $0 <identity_string> <network> <pool_router_address> <asset> <direction> <amount> <out_minimum>"
    exit 1
fi

IDENTITY_STRING=$1
NETWORK=$2
POOL_ROUTER_ADDR=$3
ASSET=$4
DIRECTION=$5
AMOUNT=$6
OUT_MINIMUM=$7

# Load env vars dynamically
source "$(dirname "${BASH_SOURCE[0]}")/load-env.sh" "$NETWORK"

# Fetch the admin's address
ADMIN_ADDRESS=$(soroban keys address $IDENTITY_STRING)

# Check if timestamp is a valid number (only digits)
if ! [[ "$AMOUNT" =~ ^[0-9]+$ ]]; then
    echo "Error: AMOUNT is not a valid number."
    exit 1
fi

echo "Swapping with the pool..."

stellar contract invoke \
    --id $POOL_ROUTER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase $STELLAR_NETWORK_PASSPHRASE \
    --fee $STELLAR_BASE_FEE \
    -- \
    swap \
    --user $ADMIN_ADDRESS \
    --asset $ASSET \
    --direction $DIRECTION \
    --in_amount $AMOUNT \
    --out_min $OUT_MINIMUM

echo "Pool swap complete."
