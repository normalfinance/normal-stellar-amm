#!/bin/bash
set -e

# Usage
usage() {
    echo "Usage:"
    echo "  $0 <identity_string> <network> <pool_router_address> <token_target> <token_name> <token_symbol> <fee_fraction> <pool_tier> <max_insurance>"
    echo ""
    echo "Example:"
    echo "  $0 josh CAS123 BTC 'Normal Bitcoin' nBTC 30 A 1000000"
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
NORMAL_TOKEN_NAME="$5"
NORMAL_TOKEN_SYMBOL="$6"
FEE_FRACTION="$7"
POOL_TIER="$8"
MAX_INSURANCE="$9"

# Fixed config
XLM="CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC"

# Derive LP token info
LP_TOKEN_NAME="$NORMAL_TOKEN_SYMBOL-XLM LP Token"
LP_TOKEN_SYMBOL="$NORMAL_TOKEN_SYMBOL-XLM-LP"

cd target/wasm32v1-none/release

# Get admin address
ADMIN_ADDRESS=$(soroban keys address "$IDENTITY_STRING")

# Deploy synth token
NORMAL_TOKEN_ADDR=$(soroban contract deploy \
    --wasm soroban_token_contract.optimized.wasm \
    --source $IDENTITY_STRING \
    --network $NETWORK)

# Initialize synth token
soroban contract invoke \
    --id "$NORMAL_TOKEN_ADDR" \
    --source "$IDENTITY_STRING" \
    --network "$NETWORK" \
    -- \
    initialize \
    --admin "$ADMIN_ADDRESS" \
    --decimal 7 \
    --name "$NORMAL_TOKEN_NAME" \
    --symbol "$NORMAL_TOKEN_SYMBOL"

echo "✅ Normal Token '$NORMAL_TOKEN_NAME' initialized at: $NORMAL_TOKEN_ADDR"

# Initialize pool
echo "📦 Initializing $NORMAL_TOKEN_NAME/XLM pool through Pool Router..."

stellar contract invoke \
    --id "$POOL_ROUTER_ADDR" \
    --source "$IDENTITY_STRING" \
    --network "$NETWORK" \
    -- \
    init_pool \
    --admin "$ADMIN_ADDRESS" \
    --assets "[\"$NORMAL_TOKEN_TARGET\", \"XLM\"]" \
    --tokens "[\"$NORMAL_TOKEN_ADDR\", \"$XLM\"]" \
    --lp_token_info "[\"$LP_TOKEN_NAME\", \"$LP_TOKEN_SYMBOL\"]" \
    --fee_fraction "$FEE_FRACTION" \
    --tier "$POOL_TIER" \
    --quote_max_insurance "$MAX_INSURANCE"

# Query initialized pool
echo "🔍 Querying $NORMAL_TOKEN_NAME/XLM pool address..."
POOL_ADDR=$(soroban contract invoke \
    --id "$POOL_ROUTER_ADDR" \
    --source "$IDENTITY_STRING" \
    --network "$NETWORK" --fee 100 \
    -- \
    get_pools | jq -r '.[0]')

echo "✅ Pool contract initialized at: $POOL_ADDR"

# # Ensure the script exits on any errors
# set -e

# # Check if the argument is provided
# if [ -z "$1" ]; then
#     echo "Usage: $0 <identity_string>"
#     exit 1
# fi

# IDENTITY_STRING=$1
# NETWORK="testnet"

# # mainnet = "CAS3J7GYLGXMF6TDJBBYYSE3HQ6BBSMLNUQ34T6TZMYMW2EVH34XOWMA"
# XLM="CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC"

# # Config
# POOL_ROUTER_ADDR="..."

# NORMAL_TOKEN_TARGET="BTC"
# NORMAL_TOKEN_NAME="Normal Bitcoin"
# NORMAL_TOKEN_SYMBOL="nBTC"

# LP_TOKEN_NAME="$NORMAL_TOKEN_SYMBOL-XLM LP Token"
# LP_TOKEN_SYMBOL="$NORMAL_TOKEN_SYMBOL-XLM-LP"

# FEE_FRACTION=30
# POOL_TIER="A"
# MAX_INSURANCE=1000000

# # Synth token init
# NORMAL_TOKEN_ADDR=$(soroban contract deploy \
#     --wasm soroban_token_contract.optimized.wasm \
#     --source $IDENTITY_STRING \
#     --network $NETWORK)

# soroban contract invoke \
#     --id $SYNTH_TOKEN_ADDR \
#     --source $IDENTITY_STRING \
#     --network $NETWORK \
#     -- \
#     initialize \
#     --admin $ADMIN_ADDRESS \
#     --decimal 7 \
#     --name $NORMAL_TOKEN_NAME \
#     --symbol $NORMAL_TOKEN_SYMBOL

# echo "Normal Token - $NORMAL_TOKEN_NAME - initialized."

# # Pool init
# echo "Initialize $NORMAL_TOKEN_NAME/XLM pool through Pool Router..."

# stellar contract invoke \
#     --id $POOL_ROUTER_ADDR \
#     --source $IDENTITY_STRING \
#     --network $NETWORK \
#     -- \
#     init_pool \
#     --admin $ADMIN_ADDRESS \
#     --assets "[\"$NORMAL_TOKEN_TARGET\", 'XLM']" \
#     --tokens "[\"$NORMAL_TOKEN_ADDR\", \"$XLM\"]" \
#     --lp_token_info "[\"$LP_TOKEN_NAME\", \"$LP_TOKEN_SYMBOL\"]" \
#     --fee_fraction $FEE_FRACTION \
#     --tier $POOL_TIER \
#     --quote_max_insurance $MAX_INSURANCE

# echo "Query $NORMAL_TOKEN_NAME/XLM pool address..."

# POOL_ADDR=$(soroban contract invoke \
#     --id $POOL_ROUTER_ADDR \
#     --source $IDENTITY_STRING \
#     --network $NETWORK --fee 100 \
#     -- \
#     get_pools | jq -r '.[0]')

# echo "Pool contract initialized."

# echo "$NORMAL_TOKEN_NAME/XLM Pool Contract address: $POOL_ADDR"
