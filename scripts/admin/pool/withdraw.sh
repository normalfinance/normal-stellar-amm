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

# Fetch the admin's address
ADMIN_ADDRESS=$(soroban keys address $IDENTITY_STRING)

# Check if timestamp is a valid number (only digits)
if ! [[ "$SHARE_AMOUNT" =~ ^[0-9]+$ ]]; then
    echo "Error: SHARE_AMOUNT is not a valid number."
    exit 1
fi

# XLM_BALANCE=$(stellar account balance $ADMIN_ADDRESS)

# if [ -z "$XLM_BALANCE" ]; then
#     echo "Error: Could not retrieve XLM balance for account $ADMIN_ADDRESS"
#     exit 1
# fi

# if [ "$XLM_BALANCE" -lt "$AMOUNT" ]; then
#     echo "❌ Error: Account has insufficient XLM. Balance: $XLM_BALANCE, required: $AMOUNT"
#     exit 1
# fi

echo "Withdraw liquidity into pool..."

stellar contract invoke \
    --id $POOL_ROUTER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    withdraw \
    --user $ADMIN_ADDRESS \
    --asset $ASSET \
    --share_amount $SHARE_AMOUNT

echo "Pool withdrawal complete."
