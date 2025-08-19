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
soroban contract optimize --wasm lp_token.wasm
soroban contract optimize --wasm pool.wasm
soroban contract optimize --wasm pool_router.wasm
soroban contract optimize --wasm buffer.wasm
soroban contract optimize --wasm insurance_fund.wasm
soroban contract optimize --wasm oracle_registry.wasm
soroban contract optimize --wasm pool_swap_fee.wasm
soroban contract optimize --wasm pool_plane.wasm
soroban contract optimize --wasm liquidity_calculator.wasm

echo "Contracts optimized."

# Fetch the admin's address
ADMIN_ADDRESS=$(soroban keys address $IDENTITY_STRING)

echo "Install the pool contract..."

POOL_WASM_HASH=$(soroban contract upload \
    --wasm pool.optimized.wasm \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE 
    )

echo "Pool contract deployed."

echo "Install the LP token contract..."

LP_TOKEN_WASM_HASH=$(soroban contract upload \
    --wasm lp_token.optimized.wasm \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE 
    )

echo "LP token contract deployed."

# #     ______     _______        __       ______   ___       _______   ________
# #    /    " \   /"      \      /""\     /" _  "\ |"  |     /"     "| /"       )
# #   // ____  \ |:        |    /    \   (: ( \___)||  |    (: ______)(:   \___/
# #  /  /    ) :)|_____/   )   /' /\  \   \/ \     |:  |     \/    |   \___  \
# # (: (____/ //  //      /   //  __'  \  //  \ _   \  |___  // ___)_   __/  \\
# #  \        /  |:  __   \  /   /  \\  \(:   _) \ ( \_|:  \(:      "| /" \   :)
# #   \"_____/   |__|  \___)(___/    \___)\_______) \_______)\_______)(_______/

# echo "Initialize oracle registry..."

# ORACLE_REGISTRY_ADDR=$(stellar contract deploy \
#     --wasm oracle_registry.optimized.wasm \
#     --source $IDENTITY_STRING \
#     --network $NETWORK \
#     --rpc-url $STELLAR_RPC_URL \
#     --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
#     --fee $STELLAR_BASE_FEE 
#     )

# #   _______     ______    ____  ____  ___________  _______   _______
# #  /"      \   /    " \  ("  _||_ " |("     _   ")/"     "| /"      \
# # |:        | // ____  \ |   (  ) : | )__/  \\__/(: ______)|:        |
# # |_____/   )/  /    ) :)(:  |  | . )    \\_ /    \/    |  |_____/   )
# #  //      /(: (____/ //  \\ \__/ //     |.  |    // ___)_  //      /
# # |:  __   \ \        /   /\\ __ //\     \:  |   (:      "||:  __   \
# # |__|  \___) \"_____/   (__________)     \__|    \_______)|__|  \___)

# echo "Initialize pool router..."

# POOL_PLANE_ADDR=$(soroban contract deploy \
#     --wasm pool_plane.optimized.wasm \
#     --source $IDENTITY_STRING \
#     --network $NETWORK \
#     --rpc-url $STELLAR_RPC_URL \
#     --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
#     --fee $STELLAR_BASE_FEE
#     )

# LIQUIDITY_CALCULATOR_ADDR=$(soroban contract deploy \
#     --wasm liquidity_calculator.optimized.wasm \
#     --source $IDENTITY_STRING \
#     --network $NETWORK \
#     --rpc-url $STELLAR_RPC_URL \
#     --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
#     --fee $STELLAR_BASE_FEE
#     )

# POOL_ROUTER_ADDR=$(soroban contract deploy \
#     --wasm pool_router.optimized.wasm \
#     --source $IDENTITY_STRING \
#     --network $NETWORK \
#     --rpc-url $STELLAR_RPC_URL \
#     --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
#     --fee $STELLAR_BASE_FEE
#     )

# echo "Tokens and pool router deployed."

# #  _______   ____  ____   _______   _______   _______   _______
# # |   _  "\ ("  _||_ " | /"     "| /"     "| /"     "| /"      \
# # (. |_)  :)|   (  ) : |(: ______)(: ______)(: ______)|:        |
# # |:     \/ (:  |  | . ) \/    |   \/    |   \/    |  |_____/   )
# # (|  _  \\  \\ \__/ //  // ___)   // ___)   // ___)_  //      /
# # |: |_)  :) /\\ __ //\ (:  (     (:  (     (:      "||:  __   \
# # (_______/ (__________) \__/      \__/      \_______)|__|  \___)

# echo "Initialize buffer..."

# BUFFER_ADDR=$(soroban contract deploy \
#     --wasm buffer.optimized.wasm \
#     --source $IDENTITY_STRING \
#     --network $NETWORK \
#     --rpc-url $STELLAR_RPC_URL \
#     --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
#     --fee $STELLAR_BASE_FEE
#     )

# #   __    _____  ___    ________  ____  ____   _______        __      _____  ___    ______    _______
# #  |" \  (\"   \|"  \  /"       )("  _||_ " | /"      \      /""\    (\"   \|"  \  /" _  "\  /"     "|
# #  ||  | |.\\   \    |(:   \___/ |   (  ) : ||:        |    /    \   |.\\   \    |(: ( \___)(: ______)
# #  |:  | |: \.   \\  | \___  \   (:  |  | . )|_____/   )   /' /\  \  |: \.   \\  | \/ \      \/    |
# #  |.  | |.  \    \. |  __/  \\   \\ \__/ //  //      /   //  __'  \ |.  \    \. | //  \ _   // ___)_
# #  /\  |\|    \    \ | /" \   :)  /\\ __ //\ |:  __   \  /   /  \\  \|    \    \ |(:   _) \ (:      "|
# # (__\_|_)\___|\____\)(_______/  (__________)|__|  \___)(___/    \___)\___|\____\) \_______) \_______)

# echo "Initialize insurance fund..."

# INSURANCE_FUND_ADDR=$(soroban contract deploy \
#     --wasm insurance_fund.optimized.wasm \
#     --source $IDENTITY_STRING \
#     --network $NETWORK \
#     --rpc-url $STELLAR_RPC_URL \
#     --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
#     --fee $STELLAR_BASE_FEE
#     )

# #   _______   _______   _______       ______    ______    ___      ___       _______   ______  ___________  ______     _______
# #  /"     "| /"     "| /"     "|     /" _  "\  /    " \  |"  |    |"  |     /"     "| /" _  "\("     _   ")/    " \   /"      \
# # (: ______)(: ______)(: ______)    (: ( \___)// ____  \ ||  |    ||  |    (: ______)(: ( \___))__/  \\__// ____  \ |:        |
# #  \/    |   \/    |   \/    |       \/ \    /  /    ) :)|:  |    |:  |     \/    |   \/ \        \\_ /  /  /    ) :)|_____/   )
# #  // ___)   // ___)_  // ___)_      //  \ _(: (____/ //  \  |___  \  |___  // ___)_  //  \ _     |.  | (: (____/ //  //      /
# # (:  (     (:      "|(:      "|    (:   _) \\        /  ( \_|:  \( \_|:  \(:      "|(:   _) \    \:  |  \        /  |:  __   \
# #  \__/      \_______) \_______)     \_______)\"_____/    \_______)\_______)\_______) \_______)    \__|   \"_____/   |__|  \___)

# echo "Initialize fee collector..."

# FEE_COLLECTOR_ADDR=$(soroban contract deploy \
#     --wasm pool_swap_fee.optimized.wasm \
#     --source $IDENTITY_STRING \
#     --network $NETWORK \
#     --rpc-url $STELLAR_RPC_URL \
#     --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
#     --fee $STELLAR_BASE_FEE
#     )

# echo "#############################"

# echo "Initialization complete!"
# echo "XLM address: $XLM_ADDRESS"

# echo "Pool Router Contract address: $POOL_ROUTER_ADDR"

# echo "Oracle Registry Contract address: $ORACLE_REGISTRY_ADDR"
# echo "Buffer Contract address: $BUFFER_ADDR"
# echo "Insurance Fund Contract address: $INSURANCE_FUND_ADDR"
# echo "Fee Collector Contract address: $FEE_COLLECTOR_ADDR"
# echo "Pool Plane Contract address: $POOL_PLANE_ADDR"
# echo "Liq. Calculator Contract address: $LIQUIDITY_CALCULATOR_ADDR"

# echo "Pool wasm hash: $POOL_WASM_HASH"
echo "LP Token wasm hash: $LP_TOKEN_WASM_HASH"