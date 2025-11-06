# Ensure the script exits on any errors
set -e

# Check if the arguments are provided
# Required: identity_string, network, asset, share_amount
if [ "$#" -lt 3 ]; then
    echo "Usage: $0 <identity_string> <network> <normal_token>"
    exit 1
fi

IDENTITY_STRING=$1
NETWORK=$2
NORMAL_TOKEN=$3

# Load env vars dynamically
REPO_ROOT="$(git rev-parse --show-toplevel)"
source "$REPO_ROOT/scripts/load-env.sh" "$NETWORK"

# Fetch the admin's address
ADMIN_ADDRESS=$(soroban keys address $IDENTITY_STRING)

RESPONSE=$(stellar contract invoke \
    --id $POOL_ROUTER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE \
    -- \
    fill_liquidity \
    --admin $ADMIN_ADDRESS \
    --tokens "[\"$NORMAL_TOKEN\", \"$USDC_ADDRESS\"]")

echo "$RESPONSE"
