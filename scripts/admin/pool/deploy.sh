# Ensure the script exits on any errors
set -e

# Usage
usage() {
    echo "Usage:"
    echo "  $0 <identity_string> <network> <normal_token> <fee_fraction> <base_asset> <quote_asset>"
    echo ""
    echo "Example:"
    echo "  $0 admin testnet 30 CA....123 BTC USDC"
    exit 1
}

# Validate args
if [ "$#" -ne 6 ]; then
    usage
fi

# Parse arguments
IDENTITY_STRING="$1"
NETWORK=$2
NORMAL_TOKEN="$3"
FEE_FRACTION="$4"
BASE_ASSET="$5"
QUOTE_ASSET="$6"

# Load env vars dynamically
REPO_ROOT="$(git rev-parse --show-toplevel)"
source "$REPO_ROOT/scripts/load-env.sh" "$NETWORK"

cd target/wasm32v1-none/release

# Get admin address
ADMIN_ADDRESS=$(soroban keys address "$IDENTITY_STRING")

# Initialize pool
echo "📦 Initializing pool through Pool Router..."

POOL_RESPONSE=$(stellar contract invoke \
    --id "$POOL_ROUTER_ADDR" \
    --source "$IDENTITY_STRING" \
    --network "$NETWORK" \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE \
    -- \
    init_elastic_pool \
    --user "$ADMIN_ADDRESS" \
    --tokens "[\"$NORMAL_TOKEN\", \"$USDC_ADDRESS\"]" \
    --fee_fraction "$FEE_FRACTION" \
    --oracle "$REFLECTOR_ORACLE" \
    --assets_config "[\"$BASE_ASSET\", \"$QUOTE_ASSET\"]"
)

echo "✅ Pool contract initialized at: $POOL_RESPONSE"