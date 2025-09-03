# Ensure the script exits on any errors
set -e

# Check if the argument is provided
if [ "$#" -lt 3 ]; then
    echo "Usage: $0 <identity_string> <network> <asset>"
    exit 1
fi

IDENTITY_STRING="$1"
NETWORK="$2"
ASSET="$3"

# Load env vars dynamically
REPO_ROOT="$(git rev-parse --show-toplevel)"
source "$REPO_ROOT/scripts/load-env.sh" "$NETWORK"

# Get admin address
ADMIN_ADDRESS=$(soroban keys address "$IDENTITY_STRING")

# Oracle registration
echo "Deleting a $ASSET oracle..."

stellar contract invoke \
    --id $ORACLE_REGISTRY_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE \
    -- \
    delete_oracle \
    --admin $ADMIN_ADDRESS \
    --asset $ASSET

echo "$ASSET oracle deleted."
