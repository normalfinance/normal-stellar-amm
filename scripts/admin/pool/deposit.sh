# Ensure the script exits on any errors
set -e

# Check if the arguments are provided
# Required: identity_string, network, pool_router_address, asset, amount
if [ "$#" -lt 5 ]; then
    echo "Usage: $0 <identity_string> <network> <pool_router_address> <asset> <amount>"
    exit 1
fi

IDENTITY_STRING=$1
NETWORK=$2
POOL_ROUTER_ADDR=$3
ASSET=$4
AMOUNT=$5

# Load env vars dynamically
source "$(dirname "${BASH_SOURCE[0]}")/load-env.sh" "$NETWORK"

# Fetch the admin's address
ADMIN_ADDRESS=$(soroban keys address $IDENTITY_STRING)

# Check if timestamp is a valid number (only digits)
if ! [[ "$AMOUNT" =~ ^[0-9]+$ ]]; then
    echo "Error: AMOUNT is not a valid number."
    exit 1
fi

echo "Deposit liquidity into pool..."

stellar contract invoke \
    --id $POOL_ROUTER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE \
    -- \
    deposit \
    --user $ADMIN_ADDRESS \
    --asset $ASSET \
    --token_b_amount $AMOUNT

echo "Pool deposit complete."
