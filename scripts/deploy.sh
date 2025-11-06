# Ensure the script exits on any errors
set -e

# Check if the argument is provided
if [ "$#" -lt 2 ]; then
    echo "Usage: $0 <identity_string> <network>"
    exit 1
fi

IDENTITY_STRING=$1
NETWORK=$2

# Load env vars dynamically
source "$(dirname "${BASH_SOURCE[0]}")/load-env.sh" "$NETWORK"

echo $STELLAR_RPC_URL
echo "$STELLAR_NETWORK_PASSPHRASE"

echo "Build and optimize the contracts..."

task build
cd target/wasm32v1-none/release

echo "Contracts compiled."
echo "Optimize contracts..."

soroban contract optimize --wasm soroban_token_contract.wasm
soroban contract optimize --wasm token_share.wasm

soroban contract optimize --wasm config_storage.wasm
soroban contract optimize --wasm rewards_gauge.wasm

soroban contract optimize --wasm pool.wasm
soroban contract optimize --wasm pool_elastic.wasm
soroban contract optimize --wasm pool_router.wasm

soroban contract optimize --wasm pool_plane.wasm
soroban contract optimize --wasm liquidity_calculator.wasm

echo "Contracts optimized."

# Fetch the admin's address
ADMIN_ADDRESS=$(soroban keys address $IDENTITY_STRING)

echo "Install the pool and pool elastic contract..."

POOL_WASM_HASH=$(soroban contract upload \
    --wasm pool.optimized.wasm \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE
)

POOL_ELASTIC_WASM_HASH=$(soroban contract upload \
    --wasm pool_elastic.optimized.wasm \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE
)

echo "Pool contracts deployed."

echo "Install the Token Share contract..."

TOKEN_SHARE_WASM_HASH=$(soroban contract upload \
    --wasm token_share.optimized.wasm \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE
)

echo "Token Share contract deployed."

REWARDS_GAUGE_WASM_HASH=$(soroban contract upload \
    --wasm rewards_gauge.optimized.wasm \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE
)

#   _______     ______    ____  ____  ___________  _______   _______
#  /"      \   /    " \  ("  _||_ " |("     _   ")/"     "| /"      \
# |:        | // ____  \ |   (  ) : | )__/  \\__/(: ______)|:        |
# |_____/   )/  /    ) :)(:  |  | . )    \\_ /    \/    |  |_____/   )
#  //      /(: (____/ //  \\ \__/ //     |.  |    // ___)_  //      /
# |:  __   \ \        /   /\\ __ //\     \:  |   (:      "||:  __   \
# |__|  \___) \"_____/   (__________)     \__|    \_______)|__|  \___)

echo "Initialize pool router..."

CONFIG_STORAGE_ADDR=$(soroban contract deploy \
    --wasm config_storage.optimized.wasm \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE \
    -- --admin $ADMIN_ADDRESS --emergency_admin $ADMIN_ADDRESS
)

# REWARDS_GAUGE_ADDR=$(soroban contract deploy \
#     --wasm rewards_gauge.optimized.wasm \
#     --source $IDENTITY_STRING \
#     --network $NETWORK \
#     --rpc-url $STELLAR_RPC_URL \
#     --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
#     --fee $STELLAR_BASE_FEE \
#     -- --pool $POOL --reward_token $XLM_ADDRESS
#     )

POOL_PLANE_ADDR=$(soroban contract deploy \
    --wasm pool_plane.optimized.wasm \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE
)

LIQUIDITY_CALCULATOR_ADDR=$(soroban contract deploy \
    --wasm liquidity_calculator.optimized.wasm \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE
)

POOL_ROUTER_ADDR=$(soroban contract deploy \
    --wasm pool_router.optimized.wasm \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE
)

echo "Tokens and pool router deployed."

echo "#############################"

echo "Initialization complete!"
# echo "XLM address: $XLM_ADDRESS"

echo "Pool Router Contract address: $POOL_ROUTER_ADDR"
echo "Pool Plane Contract address: $POOL_PLANE_ADDR"
echo "Liq. Calculator Contract address: $LIQUIDITY_CALCULATOR_ADDR"
echo "Config Storage Contract address: $CONFIG_STORAGE_ADDR"

echo "Pool wasm hash: $POOL_WASM_HASH"
echo "Pool Elastic wasm hash: $POOL_ELASTIC_WASM_HASH"
echo "Token Share wasm hash: $TOKEN_SHARE_WASM_HASH"
echo "Rewards Gauge wasm hash: $REWARDS_GAUGE_WASM_HASH"