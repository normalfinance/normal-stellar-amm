# Ensure the script exits on any errors
set -e

# Usage
usage() {
    echo "Usage:"
    echo "  $0 <issuer> <admin> <network> <asset> <fee_fraction> <pool_tier> <max_insurance>"
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
ASSET="$3"
NORMAL_TOKEN_SYMBOL="n$ASSET"
FEE_FRACTION="$5"
POOL_TIER="$6"
MAX_INSURANCE="$7"

# Load env vars dynamically
REPO_ROOT="$(git rev-parse --show-toplevel)"
source "$REPO_ROOT/scripts/load-env.sh" "$NETWORK"

# Get addresses
ADMIN_ADDRESS=$(soroban keys address "$ADMIN")
ISSUER_ADDRESS=$(soroban keys address "$ISSUER")

echo "Issuing a new asset with trustline..."

stellar tx new change-trust \
    --source-account "$DISTRIBUTOR" \
    --network "$NETWORK" \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE \
    --line "$SYMBOL:$ISSUER_ADDRESS"

echo "Deploying Stellar Asset Contract (SAC)..."

stellar contract asset deploy \
    --source "$ISSUER" \
    --network "$NETWORK" \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE \
    --asset "${SYMBOL}:${ISSUER_ADDRESS}"

echo "Launching liquidity pool..."

LP_TOKEN_NAME="$NORMAL_TOKEN_SYMBOL-XLM LP Token"
LP_TOKEN_SYMBOL="$NORMAL_TOKEN_SYMBOL-XLM-LP"

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
    --max_insurance "$MAX_INSURANCE"

echo "Setting SAC admin to pool address..."

stellar contract invoke \
    --source-account "$ISSUER" \
    --network "$NETWORK" \
    --id $SAC_ADDRESS \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE \
    -- \
    set_admin \
    --new_admin $POOL_ADDRESS

echo "Deposit liquidity into pool..."

stellar contract invoke \
    --id $POOL_ROUTER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE \
    -- \
    deposit \
    --user $ADMIN_ADDRESS \
    --asset $ASSET \
    --token_b_amount $AMOUNT
