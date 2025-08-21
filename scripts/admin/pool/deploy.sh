# Ensure the script exits on any errors
set -e

# Usage
usage() {
    echo "Usage:"
    echo "  $0 <identity_string> <network> <pool_router_address> <token_target> <token_symbol> <fee_fraction> <pool_tier> <max_insurance> <sac_address>"
    echo ""
    echo "Example:"
    echo "  $0 josh CAS123 BTC 'Normal Bitcoin' nBTC 30 A 1000000 CAS123"
    exit 1
}

# Validate args
if [ "$#" -ne 9 ]; then
    usage
fi

# Parse arguments
IDENTITY_STRING="$1"
NETWORK=$2
POOL_ROUTER_ADDR="$3"
NORMAL_TOKEN_TARGET="$4"
NORMAL_TOKEN_SYMBOL="$5"
FEE_FRACTION="$6"
POOL_TIER="$7"
MAX_INSURANCE="$8"
SYNTHETIC_SAC_ADDRESS="$9"

# Load env vars dynamically
source "$(dirname "${BASH_SOURCE[0]}")/load-env.sh" "$NETWORK"

# Derive LP token info
LP_TOKEN_NAME="$NORMAL_TOKEN_SYMBOL-XLM LP Token"
LP_TOKEN_SYMBOL="$NORMAL_TOKEN_SYMBOL-XLM-LP"

cd target/wasm32v1-none/release

# Get admin address
ADMIN_ADDRESS=$(soroban keys address "$IDENTITY_STRING")

# Initialize pool
echo "📦 Initializing $NORMAL_TOKEN_SYMBOL/XLM pool through Pool Router..."

stellar contract invoke \
    --id "$POOL_ROUTER_ADDR" \
    --source "$IDENTITY_STRING" \
    --network "$NETWORK" \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE \
    -- \
    init_pool \
    --admin "$ADMIN_ADDRESS" \
    --assets "[\"$NORMAL_TOKEN_TARGET\", \"XLM\"]" \
    --token_b "$XLM" \
    --synthetic_sac_address "$SYNTHETIC_SAC_ADDRESS" \
    --lp_token_info "[\"$LP_TOKEN_NAME\", \"$LP_TOKEN_SYMBOL\"]" \
    --fee_fraction "$FEE_FRACTION" \
    --tier "$POOL_TIER" \
    --quote_max_insurance "$MAX_INSURANCE"

# Query initialized pool
echo "🔍 Querying $NORMAL_TOKEN_SYMBOL/XLM pool address..."
POOL_ADDR=$(soroban contract invoke \
    --id "$POOL_ROUTER_ADDR" \
    --source "$IDENTITY_STRING" \
    --network "$NETWORK" --fee 100 \
    -- \
    get_pools | jq -r '.[0]')

echo "✅ Pool contract initialized at: $POOL_ADDR"