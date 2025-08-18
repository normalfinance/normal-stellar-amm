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

# Load env vars dynamically
source "$(dirname "${BASH_SOURCE[0]}")/load-env.sh" "$NETWORK"

# Get admin address
ADMIN_ADDRESS=$(soroban keys address "$IDENTITY_STRING")

# Oracle registration
echo "Registering a $ASSET oracle..."

# (Register XLM) - Transaction Fee: 0.8491044 XLM
# (Register SOL) - Transaction Fee: 0.7792485 XLM
stellar contract invoke \
    --id $ORACLE_REGISTRY_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase $STELLAR_NETWORK_PASSPHRASE \
    --fee $STELLAR_BASE_FEE \
    -- \
    register_oracle \
    --admin $ADMIN_ADDRESS \
    --asset $ASSET \
    --oracle_addr $REFLECTOR_ORACLE \
    --decimals $DECIMALS \
    --sanitize_clamp_denominator $CLAMP

echo "$ASSET oracle registered."
