# Ensure the script exits on any errors
set -e

# Check if the arguments are provided
# Required: identity_string, network, asset, amount
if [ "$#" -lt 6 ]; then
    echo "Usage: $0 <identity_string> <network> <normal_token> <pool_index> <amount_a> <amount_b>"
    exit 1
fi

IDENTITY_STRING=$1
NETWORK=$2
NORMAL_TOKEN=$3
POOL_INDEX=$4
AMOUNT_A=$5
AMOUNT_B=$6

# Load env vars dynamically
REPO_ROOT="$(git rev-parse --show-toplevel)"
source "$REPO_ROOT/scripts/load-env.sh" "$NETWORK"

# Fetch the admin's address
ADMIN_ADDRESS=$(soroban keys address $IDENTITY_STRING)

# Check if timestamp is a valid number (only digits)
if ! [[ "$AMOUNT_A" =~ ^[0-9]+$ ]]; then
    echo "Error: AMOUNT_A is not a valid number."
    exit 1
fi
if ! [[ "$AMOUNT_B" =~ ^[0-9]+$ ]]; then
    echo "Error: AMOUNT_B is not a valid number."
    exit 1
fi

echo "Deposit liquidity into pool..."

RESPONSE=$(stellar contract invoke \
    --id $POOL_ROUTER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE \
    -- \
    deposit \
    --user $ADMIN_ADDRESS \
    --tokens "[\"$NORMAL_TOKEN\", \"$USDC_ADDRESS\"]" \
    --pool_index $POOL_INDEX \
    --desired_amounts "[\"$AMOUNT_A\", \"$AMOUNT_B\"]" \
    --min_shares 0)

echo "$RESPONSE"
echo "Pool deposit complete."