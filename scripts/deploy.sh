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

# make build >/dev/null
task build
cd target/wasm32v1-none/release

echo "Contracts compiled."
echo "Optimize contracts..."

soroban contract optimize --wasm soroban_token_contract.wasm
soroban contract optimize --wasm pool.wasm
soroban contract optimize --wasm pool_router.wasm
soroban contract optimize --wasm buffer.wasm
soroban contract optimize --wasm insurance_fund.wasm
soroban contract optimize --wasm oracle_registry.wasm
soroban contract optimize --wasm pool_swap_fee.wasm

echo "Contracts optimized."

# # Fetch the admin's address
ADMIN_ADDRESS=$(soroban keys address $IDENTITY_STRING)

echo "Deploy the soroban_token_contract and capture its contract ID hash..."

XLM="CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC"

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

#   _______     ______    ____  ____  ___________  _______   _______
#  /"      \   /    " \  ("  _||_ " |("     _   ")/"     "| /"      \
# |:        | // ____  \ |   (  ) : | )__/  \\__/(: ______)|:        |
# |_____/   )/  /    ) :)(:  |  | . )    \\_ /    \/    |  |_____/   )
#  //      /(: (____/ //  \\ \__/ //     |.  |    // ___)_  //      /
# |:  __   \ \        /   /\\ __ //\     \:  |   (:      "||:  __   \
# |__|  \___) \"_____/   (__________)     \__|    \_______)|__|  \___)

echo "Initialize pool router..."

POOL_ROUTER_ADDR=$(soroban contract deploy \
    --wasm pool_router.optimized.wasm \
    --source $IDENTITY_STRING \
    --network $NETWORK)

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

echo "Tokens and pool router deployed."

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

stellar contract invoke \
    --id $ORACLE_REGISTRY_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    init_admin \
    --account $ADMIN_ADDRESS

stellar contract invoke \
    --id $ORACLE_REGISTRY_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    set_oracle_guardrails \
    --admin $ADMIN_ADDRESS \
    --oracle_guard_rails '{
        "price_divergence": {
            "oracle_twap_percent_divergence": 50000
        },
        "validity": {
            "slots_before_stale_for_pool": 10,
            "confidence_interval_max_size": 20000,
            "too_volatile_ratio": 5
        }
    }'

stellar contract invoke \
    --id $ORACLE_REGISTRY_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    set_price_override_limit \
    --admin $ADMIN_ADDRESS \
    --limit 100

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

stellar contract invoke \
    --id $BUFFER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    init_admin \
    --account $ADMIN_ADDRESS

stellar contract invoke \
    --id $BUFFER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    set_router \
    --admin $ADMIN_ADDRESS \
    --router $POOL_ROUTER_ADDR

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

stellar contract invoke \
    --id $INSURANCE_FUND_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    initialize \
    --admin $ADMIN_ADDRESS \
    --token $XLM

stellar contract invoke \
    --id $INSURANCE_FUND_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    set_unstaking_period \
    --admin $ADMIN_ADDRESS \
    --unstaking_period 13

stellar contract invoke \
    --id $INSURANCE_FUND_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    set_max_shares \
    --admin $ADMIN_ADDRESS \
    --max_shares 1000000

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

stellar contract invoke \
    --id $FEE_COLLECTOR_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    init_admin \
    --account $ADMIN_ADDRESS

stellar contract invoke \
    --id $FEE_COLLECTOR_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    set_router \
    --admin $ADMIN_ADDRESS \
    --router $POOL_ROUTER_ADDR

stellar contract invoke \
    --id $FEE_COLLECTOR_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    set_buffer \
    --admin $ADMIN_ADDRESS \
    --buffer $BUFFER_ADDR

stellar contract invoke \
    --id $FEE_COLLECTOR_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    set_fee_destination \
    --admin $ADMIN_ADDRESS \
    --fee_destination $ADMIN_ADDRESS

# Finish setting up Buffer

stellar contract invoke \
    --id $BUFFER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    set_fee_collector \
    --admin $ADMIN_ADDRESS \
    --fee_collector $FEE_COLLECTOR_ADDR

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
    --name '"Pool Share Token"' \
    --symbol '"POOL"'

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
    --name '"Normal Bitcoin"' \
    --symbol '"nBTC"'

echo "nBTC Token initialized."

echo "Initialize pool through router..."

stellar contract invoke \
    --id $POOL_ROUTER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    init_pool \
    --user $ADMIN_ADDRESS \
    --oracle_registry_ids '["BTC", "XLM"]' \
    --asset CAVLP5DH2GJPZMVO7IJY4CVOD5MWEFTJFVPD2YY2FQXOQHRGHK4D6HLP \
    --tokens "[\"$nBTC_TOKEN_ADDR\", \"$XLM\"]" \
    --lp_token_info '["Pool Share Token", "POOL"]' \
    --fee_fraction 30 \
    --tier '"A"' \
    --quote_max_insurance 1000000 \
    --oracle_registry $ORACLE_REGISTRY_ADDR

echo "Query nBTC/XLM pool address..."

POOL_ADDR=$(soroban contract invoke \
    --id $POOL_ROUTER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK --fee 100 \
    -- \
    query_pools | jq -r '.[0]')

echo "Pool contract initialized."

echo "Mint XLM token to the admin and provide liquidity..."

soroban contract invoke \
    --id $XLM \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    mint --to $ADMIN_ADDRESS --amount 10000000000 # 7 decimals, 10k tokens

# Provide liquidity to the pool
soroban contract invoke \
    --id $POOL_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK --fee 10000000 \
    -- \
    deposit --user $ADMIN_ADDRESS --desired_amount 6000000000 --min_shares 0

echo "Liquidity provided."

echo "#############################"

echo "Initialization complete!"
echo "XLM address: $XLM"

echo "Pool Router Contract address: $POOL_ROUTER_ADDR"

echo "Oracle Registry Contract address: $ORACLE_REGISTRY_ADDR"
echo "Buffer Contract address: $BUFFER_ADDR"
echo "Insurance Fund Contract address: $INSURANCE_FUND_ADDR"
echo "Fee Collector Contract address: $FEE_COLLECTOR_ADDR"

echo "nBTC/XLM Pool Contract address: $POOL_ADDR"
