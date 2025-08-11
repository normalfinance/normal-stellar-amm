# Ensure the script exits on any errors
set -e

# Check if the arguments are provided
# Required: identity_string, network, oracle_registry_address
if [ "$#" -lt 2 ]; then
    echo "Usage: $0 <identity_string> <network> <oracle_registry_address>"
    exit 1
fi

IDENTITY_STRING=$1
NETWORK=$2
ORACLE_REGISTRY_ADDR=$3

cd target/wasm32v1-none/release

# Fetch the admin's address
ADMIN_ADDRESS=$(soroban keys address $IDENTITY_STRING)

# Dynamically assign XLM based on network
case "$NETWORK" in
    "testnet")
        XLM="CBBIOHUCWZ6JS5XGZUPJW7CWZXIV2P6YJ53PAX2JGMIQQQ7O3SW2JWFN" # Testnet XLM contract address
    ;;
    "mainnet")
        XLM="CAS3J7GYLGXMF6TDJBBYYSE3HQ6BBSMLNUQ34T6TZMYMW2EVH34XOWMA" # Mainnet XLM contract address
    ;;
    *)
        echo "❌ Unknown network: $NETWORK"
        exit 1
    ;;
esac

echo "Setting up the Oracle Registry..."

stellar contract invoke \
    --id $POOL_ROUTER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    init_admin \
    --account $ADMIN_ADDRESS

stellar contract invoke \
    --id $POOL_ROUTER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    set_pool_hash \
    --admin $ADMIN_ADDRESS \
    --new_hash $POOL_WASM_HASH

stellar contract invoke \
    --id $POOL_ROUTER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    set_lp_token_hash \
    --admin $ADMIN_ADDRESS \
    --new_hash $LP_TOKEN_WASM_HASH

stellar contract invoke \
    --id $POOL_ROUTER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    set_synthetic_token_hash \
    --admin $ADMIN_ADDRESS \
    --new_hash $SYNTHETIC_TOKEN_WASM_HASH

stellar contract invoke \
    --id $POOL_ROUTER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    set_reward_token \
    --admin $ADMIN_ADDRESS \
    --reward_token $XLM

stellar contract invoke \
    --id $POOL_ROUTER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    set_privileged_addrs \
    --admin $ADMIN_ADDRESS \
    --rewards_admin $ADMIN_ADDRESS \
    --operations_admin $ADMIN_ADDRESS \
    --pause_admin $ADMIN_ADDRESS \
    --emergency_pause_admins "[{\"address\":\"$ADMIN_ADDRESS\"}]"

stellar contract invoke \
    --id $POOL_ROUTER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    set_pools_plane \
    --admin $ADMIN_ADDRESS \
    --plane $POOL_PLANE_ADDR

stellar contract invoke \
    --id $POOL_ROUTER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    set_liquidity_calculator \
    --admin $ADMIN_ADDRESS \
    --calculator $LIQUIDITY_CALCULATOR_ADDR

stellar contract invoke \
    --id $POOL_ROUTER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    set_oracle_registry \
    --admin $ADMIN_ADDRESS \
    --oracle_registry $ORACLE_REGISTRY_ADDR

echo "#############################"

echo "Setup complete!"
