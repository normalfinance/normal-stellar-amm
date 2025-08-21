# Ensure the script exits on any errors
set -e

# Check if the argument is provided
if [ "$#" -lt 5 ]; then
    echo "Usage: $0 <identity_string> <network> <asset> <decimals> <clamp>"
    exit 1
fi

IDENTITY_STRING="$1"
NETWORK="$2"
ASSET="$3"
DECIMALS="$4"
CLAMP="$5"

# Load env vars dynamically
REPO_ROOT="$(git rev-parse --show-toplevel)"
source "$REPO_ROOT/scripts/load-env.sh" "$NETWORK"

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
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE \
    -- \
    register_oracle \
    --admin $ADMIN_ADDRESS \
    --asset $ASSET \
    --oracle_addr $REFLECTOR_ORACLE \
    --decimals $DECIMALS \
    --sanitize_clamp_denominator $CLAMP

echo "$ASSET oracle registered."
