# Ensure the script exits on any errors
set -e

# Check if the arguments are provided
# Required: identity_string, network, pool_router_address, asset
if [ "$#" -lt 4 ]; then
    echo "Usage: $0 <identity_string> <network> <pool_router_address> <asset>"
    exit 1
fi

IDENTITY_STRING=$1
NETWORK=$2
POOL_ROUTER_ADDR=$3
ASSET=$4

# Fetch the admin's address
ADMIN_ADDRESS=$(soroban keys address $IDENTITY_STRING)

echo "Removing pool..."

stellar contract invoke \
    --id $POOL_ROUTER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    remove_pool \
    --user $ADMIN_ADDRESS \
    --asset $ASSET

echo "Pool removal complete."
