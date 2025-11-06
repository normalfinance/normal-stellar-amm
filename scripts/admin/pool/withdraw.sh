# Ensure the script exits on any errors
set -e

# Check if the arguments are provided
# Required: identity_string, network, asset, share_amount
if [ "$#" -lt 5 ]; then
    echo "Usage: $0 <identity_string> <network> <normal_token> <pool_index> <share_amount>"
    exit 1
fi

IDENTITY_STRING=$1
NETWORK=$2
NORMAL_TOKEN=$3
POOL_INDEX=$4
SHARE_AMOUNT=$5

# Load env vars dynamically
REPO_ROOT="$(git rev-parse --show-toplevel)"
source "$REPO_ROOT/scripts/load-env.sh" "$NETWORK"

# Fetch the admin's address
ADMIN_ADDRESS=$(soroban keys address $IDENTITY_STRING)

# Check if timestamp is a valid number (only digits)
if ! [[ "$SHARE_AMOUNT" =~ ^[0-9]+$ ]]; then
    echo "Error: SHARE_AMOUNT is not a valid number."
    exit 1
fi

echo "Withdraw liquidity into pool..."

RESPONSE=$(stellar contract invoke \
    --id $POOL_ROUTER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE \
    -- \
    withdraw \
    --user $ADMIN_ADDRESS \
    --tokens "[\"$NORMAL_TOKEN\", \"$USDC_ADDRESS\"]" \
    --pool_index $POOL_INDEX \
    --share_amount $SHARE_AMOUNT \
    --min_amounts "[\"0\", \"0\"]")

echo "$RESPONSE"
echo "Pool withdrawal complete."
