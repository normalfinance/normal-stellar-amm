# Ensure the script exits on any errors
set -e

# Check if the argument is provided
if [ -z "$1" ]; then
    echo "Usage: $0 <identity_string>"
    exit 1
fi

IDENTITY_STRING=$1
NETWORK="testnet"

# Config
REFLECTOR_ORACLE="CCYOZJCOPG34LLQQ7N24YXBM7LL62R7ONMZ3G6WZAAYPB5OYKOMJRN63"
ORACLE_REGISTRY_ADDR="..."
ASSET="BTC"
DECIMALS=14
CLAMP=0

# Oracle registration
echo "Registering a $ASSET oracle..."

stellar contract invoke \
    --id $ORACLE_REGISTRY_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    register_oracle \
    --admin $ADMIN_ADDRESS \
    --asset $ASSET \
    --oracle_addr $REFLECTOR_ORACLE \
    --decimals $DECIMALS \
    --sanitize_clamp_denominator $CLAMP

echo "$ASSET oracle registered."

echo "Query $ASSET price..."

PRICE_INFO=$(soroban contract invoke \
    --id $ORACLE_REGISTRY_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK --fee 100 \
    -- \
    get_price | jq -r '.[0]')

echo "$ASSET price info: $PRICE_INFO"
