# Ensure the script exits on any errors
set -e

# Check if the argument is provided
if [ -z "$1" ]; then
    echo "Usage: $0 <identity_string>"
    exit 1
fi

IDENTITY_STRING=$1
NETWORK="testnet"

echo "Build and optimize the contracts..."

make build >/dev/null
cd target/wasm32v1-none/release

echo "Contracts compiled."
echo "Optimize contracts..."

soroban contract optimize --wasm soroban_token_contract.wasm

soroban contract optimize --wasm soroban_pool_router_contract.wasm
soroban contract optimize --wasm soroban_pool_contract.wasm

soroban contract optimize --wasm soroban_buffer_contract.wasm
soroban contract optimize --wasm soroban_insurance_fund_contract.wasm
soroban contract optimize --wasm soroban_oracle_registry_contract.wasm
soroban contract optimize --wasm soroban_pool_provider_swap_fee_contract.wasm

echo "Contracts optimized."

# Fetch the admin's address
ADMIN_ADDRESS=$(soroban keys address $IDENTITY_STRING)

echo "Deploy the soroban_token_contract and capture its contract ID hash..."

XLM="CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC"

TOKEN_ADDR1=$XLM

NORM_TOKEN_ADDR=$(soroban contract deploy \
    --wasm soroban_token_contract.optimized.wasm \
    --source $IDENTITY_STRING \
    --network $NETWORK)

soroban contract invoke \
    --id $TOKEN_ADDR2 \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    initialize \
    --admin $ADMIN_ADDRESS \
    --decimal 7 \
    --name NORMAL \
    --symbol NORM

echo "NORM Token initialized."

POOL_ROUTER_ADDR=$(soroban contract deploy \
    --wasm normal_synth_market_factory.optimized.wasm \
    --source $IDENTITY_STRING \
    --network $NETWORK)

echo "Tokens and pool router deployed."

echo "Install the soroban_token and soroban_pool_contract contracts..."

TOKEN_WASM_HASH=$(soroban contract install \
    --wasm soroban_token_contract.optimized.wasm \
    --source $IDENTITY_STRING \
    --network $NETWORK)

# Continue with the rest of the deployments
POOL_WASM_HASH=$(soroban contract install \
    --wasm soroban_pool_contract.optimized.wasm \
    --source $IDENTITY_STRING \
    --network $NETWORK)

echo "Token, pair and stake contracts deployed."

#     ______     _______        __       ______   ___       _______   ________
#    /    " \   /"      \      /""\     /" _  "\ |"  |     /"     "| /"       )
#   // ____  \ |:        |    /    \   (: ( \___)||  |    (: ______)(:   \___/
#  /  /    ) :)|_____/   )   /' /\  \   \/ \     |:  |     \/    |   \___  \
# (: (____/ //  //      /   //  __'  \  //  \ _   \  |___  // ___)_   __/  \\
#  \        /  |:  __   \  /   /  \\  \(:   _) \ ( \_|:  \(:      "| /" \   :)
#   \"_____/   |__|  \___)(___/    \___)\_______) \_______)\_______)(_______/

echo "Initialize oracle registry..."

ORACLE_REGISTRY_ADDR=$(soroban contract deploy \
    --wasm soroban_oracle_registry_contract.optimized.wasm \
    --source $IDENTITY_STRING \
    --network $NETWORK)

stellar contract invoke \
    --id $ORACLE_REGISTRY_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    --router $ROUTER_ADDR \
    --operator $FEE_COLLECTOR_ADDR

#   _______   _______   _______       ______    ______    ___      ___       _______   ______  ___________  ______     _______
#  /"     "| /"     "| /"     "|     /" _  "\  /    " \  |"  |    |"  |     /"     "| /" _  "\("     _   ")/    " \   /"      \
# (: ______)(: ______)(: ______)    (: ( \___)// ____  \ ||  |    ||  |    (: ______)(: ( \___))__/  \\__/// ____  \ |:        |
#  \/    |   \/    |   \/    |       \/ \    /  /    ) :)|:  |    |:  |     \/    |   \/ \        \\_ /  /  /    ) :)|_____/   )
#  // ___)   // ___)_  // ___)_      //  \ _(: (____/ //  \  |___  \  |___  // ___)_  //  \ _     |.  | (: (____/ //  //      /
# (:  (     (:      "|(:      "|    (:   _) \\        /  ( \_|:  \( \_|:  \(:      "|(:   _) \    \:  |  \        /  |:  __   \
#  \__/      \_______) \_______)     \_______)\"_____/    \_______)\_______)\_______) \_______)    \__|   \"_____/   |__|  \___)

echo "Initialize fee collector..."

FEE_COLLECTOR_ADDR=$(soroban contract deploy \
    --wasm soroban_pool_provider_swap_fee_contract.optimized.wasm \
    --source $IDENTITY_STRING \
    --network $NETWORK)

stellar contract invoke \
    --id $FEE_COLLECTOR_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    --router $ROUTER_ADDR \
    --operator $ADMIN_ADDRESS \
    --fee_destination $ADMIN_ADDRESS \
    --buffer $BUFFER_ADDR \
    --max_swap_fee_fraction 30 \
    --buffer_fraction 10

#  _______   ____  ____   _______   _______   _______   _______
# |   _  "\ ("  _||_ " | /"     "| /"     "| /"     "| /"      \
# (. |_)  :)|   (  ) : |(: ______)(: ______)(: ______)|:        |
# |:     \/ (:  |  | . ) \/    |   \/    |   \/    |  |_____/   )
# (|  _  \\  \\ \__/ //  // ___)   // ___)   // ___)_  //      /
# |: |_)  :) /\\ __ //\ (:  (     (:  (     (:      "||:  __   \
# (_______/ (__________) \__/      \__/      \_______)|__|  \___)

echo "Initialize buffer..."

BUFFER_ADDR=$(soroban contract deploy \
    --wasm soroban_buffer_contract.optimized.wasm \
    --source $IDENTITY_STRING \
    --network $NETWORK)

stellar contract invoke \
    --id $BUFFER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    init_admin \
    --admin $ADMIN_ADDRESS

stellar contract invoke \
    --id $BUFFER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    set_router \
    --router $ROUTER_ADDR

stellar contract invoke \
    --id $BUFFER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    set_fee_collector \
    --fee_collector $FEE_COLLECTOR_ADDR

#   __    _____  ___    ________  ____  ____   _______        __      _____  ___    ______    _______
#  |" \  (\"   \|"  \  /"       )("  _||_ " | /"      \      /""\    (\"   \|"  \  /" _  "\  /"     "|
#  ||  | |.\\   \    |(:   \___/ |   (  ) : ||:        |    /    \   |.\\   \    |(: ( \___)(: ______)
#  |:  | |: \.   \\  | \___  \   (:  |  | . )|_____/   )   /' /\  \  |: \.   \\  | \/ \      \/    |
#  |.  | |.  \    \. |  __/  \\   \\ \__/ //  //      /   //  __'  \ |.  \    \. | //  \ _   // ___)_
#  /\  |\|    \    \ | /" \   :)  /\\ __ //\ |:  __   \  /   /  \\  \|    \    \ |(:   _) \ (:      "|
# (__\_|_)\___|\____\)(_______/  (__________)|__|  \___)(___/    \___)\___|\____\) \_______) \_______)

echo "Initialize insurance fund..."

INSURANCE_FUND_ADDR=$(soroban contract deploy \
    --wasm soroban_insurance_fund_contract.optimized.wasm \
    --source $IDENTITY_STRING \
    --network $NETWORK)

stellar contract invoke \
    --id $INSURANCE_FUND_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    --admin $ADMIN_ADDRESS \
    --token $XLM \
    --unstaking_period 13 \
    --max_shares 1000000

#   _______     ______    ____  ____  ___________  _______   _______
#  /"      \   /    " \  ("  _||_ " |("     _   ")/"     "| /"      \
# |:        | // ____  \ |   (  ) : | )__/  \\__/(: ______)|:        |
# |_____/   )/  /    ) :)(:  |  | . )    \\_ /    \/    |  |_____/   )
#  //      /(: (____/ //  \\ \__/ //     |.  |    // ___)_  //      /
# |:  __   \ \        /   /\\ __ //\     \:  |   (:      "||:  __   \
# |__|  \___) \"_____/   (__________)     \__|    \_______)|__|  \___)

echo "Initialize pool router..."

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
    set_token_hash \
    --admin $ADMIN_ADDRESS \
    --new_hash $TOKEN_WASM_HASH

stellar contract invoke \
    --id $POOL_ROUTER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    set_reward_token \
    --admin $ADMIN_ADDRESS \
    --reward_token $NORM_TOKEN_ADDR

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

# Pool Initialization process
# LP token init
LP_TOKEN_ADDR=$(soroban contract deploy \
    --wasm soroban_token_contract.optimized.wasm \
    --source $IDENTITY_STRING \
    --network $NETWORK)

soroban contract invoke \
    --id $LP_TOKEN_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    initialize \
    --admin $ADMIN_ADDRESS \
    --decimal 7 \
    --name Pool Share Token \
    --symbol POOL

echo "POOL Token initialized."

# nBTC token init
nBTC_TOKEN_ADDR=$(soroban contract deploy \
    --wasm soroban_token_contract.optimized.wasm \
    --source $IDENTITY_STRING \
    --network $NETWORK)

soroban contract invoke \
    --id $nBTC_TOKEN_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    initialize \
    --admin $ADMIN_ADDRESS \
    --decimal 7 \
    --name Normal Bitcoin \
    --symbol nBTC

echo "nBTC Token initialized."

echo "Initialize pool through router..."

stellar contract invoke \
    --id $POOL_ROUTER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    init_pool \
    --user $ADMIN_ADDRESS \
    --base_oracle_registry_id '{"symbol":"BTC", "chain":"Bitcoin"}' \
    --quote_oracle_registry_id '{"symbol":"XLM", "chain":"Stellar"}' \
    --asset CAVLP5DH2GJPZMVO7IJY4CVOD5MWEFTJFVPD2YY2FQXOQHRGHK4D6HLP \
    --tokens '[{"address":"$nBTC_TOKEN_ADDR"}, {"address":"$TOKEN_ADDR2"}]' \
    --lp_token_name "Test" \
    --lp_token_symbol "TEST" \
    --fee_fraction 30 \
    --tier

echo "Query nBTC/USDC pair address..."

POOL_ADDR2=$(soroban contract invoke \
    --id $POOL_ROUTER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK --fee 100 \
    -- \
    get_pools | jq -r '.[1]')

echo "Pool contract initialized."

echo "Mint USDC token to the admin and provide liquidity..."

soroban contract invoke \
    --id $TOKEN_ADDR2 \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    mint --to $ADMIN_ADDRESS --amount 10000000000 # 7 decimals, 10k tokens

# Provide liquidity to the pool
soroban contract invoke \
    --id $MARKET_ADDR2 \
    --source $IDENTITY_STRING \
    --network $NETWORK --fee 10000000 \
    -- \
    deposit --user $ADMIN_ADDRESS --desired_amount 6000000000 --min_shares 0

echo "Liquidity provided."

echo "#############################"

echo "Initialization complete!"
echo "XLM address: $XLM"
echo "NORM address: $TOKEN_ADDR2"
echo "USDC address: $TOKEN_ADDR1"
echo "XLM/NORM Pair Contract address: $PAIR_ADDR"
echo "XLM/NORM Stake Contract address: $STAKE_ADDR"
echo "NORM/USDC Pair Contract address: $PAIR_ADDR2"
echo "NORM/USDC Stake Contract address: $STAKE_ADDR2"
echo "Pool Router Contract address: $POOL_ROUTER_ADDR"
