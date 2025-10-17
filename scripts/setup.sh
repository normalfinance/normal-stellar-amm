# Ensure the script exits on any errors
set -e

# Check if the arguments are provided
# Required: identity_string, network
if [ "$#" -lt 2 ]; then
    echo "Usage: $0 <identity_string> <network>"
    exit 1
fi

IDENTITY_STRING=$1
NETWORK=$2

# Load env vars dynamically
source "$(dirname "${BASH_SOURCE[0]}")/load-env.sh" "$NETWORK"

# Fetch the admin's address
ADMIN_ADDRESS=$(soroban keys address $IDENTITY_STRING)

echo "Setup pool plane..."

stellar contract invoke \
    --id $POOL_PLANE_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE \
    -- \
    init_admin \
    --account $ADMIN_ADDRESS

echo "Setup liquidity calculator..."

stellar contract invoke \
    --id $LIQUIDITY_CALCULATOR_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE \
    -- \
    init_admin \
    --account $ADMIN_ADDRESS

#   _______     ______    ____  ____  ___________  _______   _______
#  /"      \   /    " \  ("  _||_ " |("     _   ")/"     "| /"      \
# |:        | // ____  \ |   (  ) : | )__/  \\__/(: ______)|:        |
# |_____/   )/  /    ) :)(:  |  | . )    \\_ /    \/    |  |_____/   )
#  //      /(: (____/ //  \\ \__/ //     |.  |    // ___)_  //      /
# |:  __   \ \        /   /\\ __ //\     \:  |   (:      "||:  __   \
# |__|  \___) \"_____/   (__________)     \__|    \_______)|__|  \___)

echo "Setup pool router..."

stellar contract invoke \
    --id $POOL_ROUTER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE \
    -- \
    init_admin \
    --account $ADMIN_ADDRESS

stellar contract invoke \
    --id $POOL_ROUTER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE \
    -- \
    set_privileged_addrs \
    --admin $ADMIN_ADDRESS \
    --rewards_admin $ADMIN_ADDRESS \
    --operations_admin $ADMIN_ADDRESS \
    --pause_admin $ADMIN_ADDRESS \
    --emergency_pause_admins "[{\"address\":\"$ADMIN_ADDRESS\"}]" \
    --system_fee_admin $ADMIN_ADDRESS

stellar contract invoke \
    --id $POOL_ROUTER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE \
    -- \
    set_token_hash \
    --admin $ADMIN_ADDRESS \
    --new_hash $TOKEN_SHARE_WASM_HASH

stellar contract invoke \
    --id $POOL_ROUTER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE \
    -- \
    set_pool_hash \
    --admin $ADMIN_ADDRESS \
    --new_hash $POOL_WASM_HASH

stellar contract invoke \
    --id $POOL_ROUTER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE \
    -- \
    set_elastic_pool_hash \
    --admin $ADMIN_ADDRESS \
    --new_hash $POOL_ELASTIC_WASM_HASH

stellar contract invoke \
    --id $POOL_ROUTER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE \
    -- \
    init_config_storage \
    --admin $ADMIN_ADDRESS \
    --config_storage $CONFIG_STORAGE_ADDR

stellar contract invoke \
    --id $POOL_ROUTER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE \
    -- \
    set_rewards_gauge_hash \
    --admin $ADMIN_ADDRESS \
    --new_hash $REWARDS_GAUGE_WASM_HASH

stellar contract invoke \
    --id $POOL_ROUTER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE \
    -- \
    set_reward_token \
    --admin $ADMIN_ADDRESS \
    --reward_token $XLM_ADDRESS

stellar contract invoke \
    --id $POOL_ROUTER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE \
    -- \
    set_protocol_fee_fraction \
    --admin $ADMIN_ADDRESS \
    --new_fraction 5000

stellar contract invoke \
    --id $POOL_ROUTER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE \
    -- \
    set_pools_plane \
    --admin $ADMIN_ADDRESS \
    --plane $POOL_PLANE_ADDR

stellar contract invoke \
    --id $POOL_ROUTER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE \
    -- \
    set_liquidity_calculator \
    --admin $ADMIN_ADDRESS \
    --calculator $LIQUIDITY_CALCULATOR_ADDR

echo "Tokens and pool router deployed."

echo "#############################"

echo "Initialization complete!"

