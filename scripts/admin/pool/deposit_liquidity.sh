# Ensure the script exits on any errors
set -e

# Check if the arguments are provided
# Required: identity_string, pool_router_address, expiry_ts
if [ "$#" -lt 3 ]; then
    echo "Usage: $0 <identity_string> <pool_router_address> <token_b_amount>"
    exit 1
fi

IDENTITY_STRING=$1
POOL_ROUTER_ADDR=$2
TOKEN_B_AMOUNT=$3
NETWORK="testnet"

# Check if timestamp is a valid number (only digits)
if ! [[ "$TOKEN_B_AMOUNT" =~ ^[0-9]+$ ]]; then
    echo "Error: TOKEN_B_AMOUNT is not a valid number."
    exit 1
fi

XLM_BALANCE=$(stellar account balance $ADMIN_ADDRESS)

if [ -z "$XLM_BALANCE" ]; then
    echo "Error: Could not retrieve XLM balance for account $ADMIN_ADDRESS"
    exit 1
fi

if [ "$XLM_BALANCE" -lt "$TOKEN_B_AMOUNT" ]; then
    echo "❌ Error: Account has insufficient XLM. Balance: $XLM_BALANCE, required: $TOKEN_B_AMOUNT"
    exit 1
fi

echo "Deposit liquidity into pool..."

stellar contract invoke \
    --id $POOL_ROUTER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    deposit \
    --user $ADMIN_ADDRESS \
    --token_b_amount $TOKEN_B_AMOUNT

echo "Pool deposit complete."
