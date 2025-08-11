# Ensure the script exits on any errors
set -e

# Check if the argument is provided
if [ "$#" -lt 6 ]; then
    echo "Usage: $0 <identity_string> <network> <oracle_registry_address> <asset> <decimals> <clamp>"
    exit 1
fi

IDENTITY_STRING="$1"
NETWORK="$2"
ORACLE_REGISTRY_ADDR="$3"
ASSET="$4"
DECIMALS="$5"
CLAMP="$6"

# Conditionally set REFLECTOR_ORACLE based on network
if [ "$NETWORK" = "testnet" ]; then
    REFLECTOR_ORACLE="CCYOZJCOPG34LLQQ7N24YXBM7LL62R7ONMZ3G6WZAAYPB5OYKOMJRN63"
elif [ "$NETWORK" = "mainnet" ]; then
    REFLECTOR_ORACLE="CAFJZQWSED6YAWZU3GWRTOCNPPCGBN32L7QV43XX5LZLFTK6JLN34DLN"
else
    echo "Unknown network: $NETWORK"
    exit 1
fi

# Get admin address
ADMIN_ADDRESS=$(soroban keys address "$IDENTITY_STRING")

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

# echo "Query $ASSET price..."

# PRICE_INFO=$(soroban contract invoke \
#     --id $ORACLE_REGISTRY_ADDR \
#     --source $IDENTITY_STRING \
#     --network $NETWORK --fee 100 \
#     -- \
#     get_price | jq -r '.[0]')

# echo "$ASSET price info: $PRICE_INFO"
