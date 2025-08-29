# Ensure the script exits on any errors
set -e

# Usage
usage() {
    echo "Usage:"
    echo "  $0 <identity_string> <network> <token_target> <token_symbol> <fee_fraction> <pool_tier> <max_insurance>"
    echo ""
    echo "Example:"
    echo "  $0 admin testnet BTC nBTC 30 A 1000000"
    exit 1
}

# Validate args
if [ "$#" -ne 7 ]; then
    usage
fi

# Parse arguments
IDENTITY_STRING="$1"
NETWORK=$2
NORMAL_TOKEN_TARGET="$3"
NORMAL_TOKEN_SYMBOL="$4"
FEE_FRACTION="$5"
POOL_TIER="$6"
MAX_INSURANCE="$7"

# Load env vars dynamically
REPO_ROOT="$(git rev-parse --show-toplevel)"
source "$REPO_ROOT/scripts/load-env.sh" "$NETWORK"

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
    --token_b "$XLM_ADDRESS" \
    --synthetic_sac_address "$ASSET_SAC_ADDRESS" \
    --lp_token_info "[\"$LP_TOKEN_NAME\", \"$LP_TOKEN_SYMBOL\"]" \
    --fee_fraction "$FEE_FRACTION" \
    --tier "$POOL_TIER" \
    --quote_max_insurance "$MAX_INSURANCE"