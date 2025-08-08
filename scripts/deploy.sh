# Ensure the script exits on any errors
set -e

# Check if the argument is provided
if [ -z "$1" ]; then
    echo "Usage: $0 <identity_string> <network>"
    exit 1
fi

IDENTITY_STRING=$1
NETWORK=$2

echo "Build and optimize the contracts..."

# make build >/dev/null
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

echo "Deploy the soroban_token_contract and capture its contract ID hash..."

XLM="CAS3J7GYLGXMF6TDJBBYYSE3HQ6BBSMLNUQ34T6TZMYMW2EVH34XOWMA"

echo "Install the soroban_token and pool contracts..."

TOKEN_WASM_HASH=$(soroban contract upload \
    --wasm soroban_token_contract.optimized.wasm \
    --source $IDENTITY_STRING \
--network $NETWORK)

# Continue with the rest of the deployments
POOL_WASM_HASH=$(soroban contract upload \
    --wasm pool.optimized.wasm \
    --source $IDENTITY_STRING \
--network $NETWORK)

echo "Token and pool contracts deployed."

#     ______     _______        __       ______   ___       _______   ________
#    /    " \   /"      \      /""\     /" _  "\ |"  |     /"     "| /"       )
#   // ____  \ |:        |    /    \   (: ( \___)||  |    (: ______)(:   \___/
#  /  /    ) :)|_____/   )   /' /\  \   \/ \     |:  |     \/    |   \___  \
# (: (____/ //  //      /   //  __'  \  //  \ _   \  |___  // ___)_   __/  \\
#  \        /  |:  __   \  /   /  \\  \(:   _) \ ( \_|:  \(:      "| /" \   :)
#   \"_____/   |__|  \___)(___/    \___)\_______) \_______)\_______)(_______/

echo "Initialize oracle registry..."

ORACLE_REGISTRY_ADDR=$(soroban contract deploy \
    --wasm oracle_registry.optimized.wasm \
    --source $IDENTITY_STRING \
--network $NETWORK)

#   _______     ______    ____  ____  ___________  _______   _______
#  /"      \   /    " \  ("  _||_ " |("     _   ")/"     "| /"      \
# |:        | // ____  \ |   (  ) : | )__/  \\__/(: ______)|:        |
# |_____/   )/  /    ) :)(:  |  | . )    \\_ /    \/    |  |_____/   )
#  //      /(: (____/ //  \\ \__/ //     |.  |    // ___)_  //      /
# |:  __   \ \        /   /\\ __ //\     \:  |   (:      "||:  __   \
# |__|  \___) \"_____/   (__________)     \__|    \_______)|__|  \___)

echo "Initialize pool router..."

POOL_PLANE_ADDR=$(soroban contract deploy \
    --wasm pool_plane.optimized.wasm \
    --source $IDENTITY_STRING \
--network $NETWORK)

LIQUIDITY_CALCULATOR_ADDR=$(soroban contract deploy \
    --wasm liquidity_calculator.optimized.wasm \
    --source $IDENTITY_STRING \
--network $NETWORK)

POOL_ROUTER_ADDR=$(soroban contract deploy \
    --wasm pool_router.optimized.wasm \
    --source $IDENTITY_STRING \
--network $NETWORK)

echo "Tokens and pool router deployed."

#  _______   ____  ____   _______   _______   _______   _______
# |   _  "\ ("  _||_ " | /"     "| /"     "| /"     "| /"      \
# (. |_)  :)|   (  ) : |(: ______)(: ______)(: ______)|:        |
# |:     \/ (:  |  | . ) \/    |   \/    |   \/    |  |_____/   )
# (|  _  \\  \\ \__/ //  // ___)   // ___)   // ___)_  //      /
# |: |_)  :) /\\ __ //\ (:  (     (:  (     (:      "||:  __   \
# (_______/ (__________) \__/      \__/      \_______)|__|  \___)

echo "Initialize buffer..."

BUFFER_ADDR=$(soroban contract deploy \
    --wasm buffer.optimized.wasm \
    --source $IDENTITY_STRING \
--network $NETWORK)

ONE_HOUR=$((3600))


#   __    _____  ___    ________  ____  ____   _______        __      _____  ___    ______    _______
#  |" \  (\"   \|"  \  /"       )("  _||_ " | /"      \      /""\    (\"   \|"  \  /" _  "\  /"     "|
#  ||  | |.\\   \    |(:   \___/ |   (  ) : ||:        |    /    \   |.\\   \    |(: ( \___)(: ______)
#  |:  | |: \.   \\  | \___  \   (:  |  | . )|_____/   )   /' /\  \  |: \.   \\  | \/ \      \/    |
#  |.  | |.  \    \. |  __/  \\   \\ \__/ //  //      /   //  __'  \ |.  \    \. | //  \ _   // ___)_
#  /\  |\|    \    \ | /" \   :)  /\\ __ //\ |:  __   \  /   /  \\  \|    \    \ |(:   _) \ (:      "|
# (__\_|_)\___|\____\)(_______/  (__________)|__|  \___)(___/    \___)\___|\____\) \_______) \_______)

echo "Initialize insurance fund..."

INSURANCE_FUND_ADDR=$(soroban contract deploy \
    --wasm insurance_fund.optimized.wasm \
    --source $IDENTITY_STRING \
--network $NETWORK)


#   _______   _______   _______       ______    ______    ___      ___       _______   ______  ___________  ______     _______
#  /"     "| /"     "| /"     "|     /" _  "\  /    " \  |"  |    |"  |     /"     "| /" _  "\("     _   ")/    " \   /"      \
# (: ______)(: ______)(: ______)    (: ( \___)// ____  \ ||  |    ||  |    (: ______)(: ( \___))__/  \\__// ____  \ |:        |
#  \/    |   \/    |   \/    |       \/ \    /  /    ) :)|:  |    |:  |     \/    |   \/ \        \\_ /  /  /    ) :)|_____/   )
#  // ___)   // ___)_  // ___)_      //  \ _(: (____/ //  \  |___  \  |___  // ___)_  //  \ _     |.  | (: (____/ //  //      /
# (:  (     (:      "|(:      "|    (:   _) \\        /  ( \_|:  \( \_|:  \(:      "|(:   _) \    \:  |  \        /  |:  __   \
#  \__/      \_______) \_______)     \_______)\"_____/    \_______)\_______)\_______) \_______)    \__|   \"_____/   |__|  \___)

echo "Initialize fee collector..."

FEE_COLLECTOR_ADDR=$(soroban contract deploy \
    --wasm pool_swap_fee.optimized.wasm \
    --source $IDENTITY_STRING \
--network $NETWORK)

echo "#############################"

echo "Initialization complete!"
echo "XLM address: $XLM"

echo "Pool Router Contract address: $POOL_ROUTER_ADDR"

echo "Oracle Registry Contract address: $ORACLE_REGISTRY_ADDR"
echo "Buffer Contract address: $BUFFER_ADDR"
echo "Insurance Fund Contract address: $INSURANCE_FUND_ADDR"
echo "Fee Collector Contract address: $FEE_COLLECTOR_ADDR"
echo "Pool Plane Contract address: $POOL_PLANE_ADDR"
echo "Liq. Calculator Contract address: $LIQUIDITY_CALCULATOR_ADDR"
