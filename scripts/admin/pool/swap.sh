# Ensure the script exits on any errors
set -e

# Check if the arguments are provided
# Required: identity_string, network, asset, direction, amount, out_minimum
if [ "$#" -lt 5 ]; then
    echo "Usage: $0 <identity_string> <network> <asset> <direction> <amount> <out_minimum>"
    exit 1
fi

IDENTITY_STRING=$1
NETWORK=$2
ASSET=$3
DIRECTION=$4
AMOUNT=$5
OUT_MINIMUM=$6

# Load env vars dynamically
REPO_ROOT="$(git rev-parse --show-toplevel)"
source "$REPO_ROOT/scripts/load-env.sh" "$NETWORK"

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
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE \
    -- \
    swap \
    --user $ADMIN_ADDRESS \
    --asset $ASSET \
    --direction $DIRECTION \
    --in_amount $AMOUNT \
    --out_min $OUT_MINIMUM

echo "Pool swap complete."
