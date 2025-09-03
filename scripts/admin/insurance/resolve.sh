# Ensure the script exits on any errors
set -e

# Check if the arguments are provided
# Required: identity_string, network, pool_address
if [ "$#" -lt 3 ]; then
    echo "Usage: $0 <identity_string> <network> <pool_address>"
    exit 1
fi

IDENTITY_STRING=$1
NETWORK=$2
POOL_ROUTER_ADDR=$3

# Load env vars dynamically
REPO_ROOT="$(git rev-parse --show-toplevel)"
source "$REPO_ROOT/scripts/load-env.sh" "$NETWORK"

echo "Running logic for insurance fund..."

stellar contract invoke \
    --id $FUND_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE \
    -- \
    resolve_liquidity_deficit \
    --admin $ADMIN_ADDRESS \
    --pool_address $POOL_ADDR

echo "Liquidity resolution initiated."
