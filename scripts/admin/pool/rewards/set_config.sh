# Ensure the script exits on any errors
set -e

# Check if the arguments are provided
# Required: identity_string, network, asset, amount
if [ "$#" -lt 5 ]; then
    echo "Usage: $0 <identity_string> <network> <pool_address> <expired_at> <tps>"
    exit 1
fi

IDENTITY_STRING=$1
NETWORK=$2
POOL_ADDR=$3
EXPIRED_AT=$4
TPS=$5

# Load env vars dynamically
REPO_ROOT="$(git rev-parse --show-toplevel)"
source "$REPO_ROOT/scripts/load-env.sh" "$NETWORK"

# Fetch the admin's address
ADMIN_ADDRESS=$(soroban keys address $IDENTITY_STRING)

RESPONSE=$(stellar contract invoke \
    --id $POOL_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE \
    -- \
    set_rewards_config \
    --admin $ADMIN_ADDRESS \
    --expired_at $EXPIRED_AT \
    --tps $TPS)

echo "$RESPONSE"