# Ensure the script exits on any errors
set -e

# Load environment variables from .env file
source .env

# Check if the arguments are provided
# Required: identity_string, network
if [ "$#" -lt 2 ]; then
    echo "Usage: $0 <identity_string> <network>"
    exit 1
fi

IDENTITY_STRING=$1
NETWORK=$2

# Fetch the admin's address
ADMIN_ADDRESS=$(soroban keys address $IDENTITY_STRING)

#     ______     _______        __       ______   ___       _______   ________
#    /    " \   /"      \      /""\     /" _  "\ |"  |     /"     "| /"       )
#   // ____  \ |:        |    /    \   (: ( \___)||  |    (: ______)(:   \___/
#  /  /    ) :)|_____/   )   /' /\  \   \/ \     |:  |     \/    |   \___  \
# (: (____/ //  //      /   //  __'  \  //  \ _   \  |___  // ___)_   __/  \\
#  \        /  |:  __   \  /   /  \\  \(:   _) \ ( \_|:  \(:      "| /" \   :)
#   \"_____/   |__|  \___)(___/    \___)\_______) \_______)\_______)(_______/

echo "Setup oracle registry..."

# Transaction Fee: 13.8046416 XLM
stellar contract invoke \
    --id $ORACLE_REGISTRY_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase $STELLAR_NETWORK_PASSPHRASE \
    --fee $STELLAR_BASE_FEE \
    -- \
    initialize \
    --admin $ADMIN_ADDRESS \
    --emergency_admin $ADMIN_ADDRESS

# Transaction Fee: 0.4171378 XLM
stellar contract invoke \
    --id $ORACLE_REGISTRY_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase $STELLAR_NETWORK_PASSPHRASE \
    --fee $STELLAR_BASE_FEE \
    -- \
    set_oracle_guard_rails \
    --admin $ADMIN_ADDRESS \
    --oracle_guard_rails '{
        "price_divergence": {
            "oracle_twap_percent_divergence": 1000000
        },
        "validity": {
            "seconds_before_stale_for_pool": 300,
            "too_volatile_ratio": 2000000
        }
    }'

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
    --network-passphrase $STELLAR_NETWORK_PASSPHRASE \
    --fee $STELLAR_BASE_FEE \
    -- \
    init_admin \
    --account $ADMIN_ADDRESS

stellar contract invoke \
    --id $POOL_ROUTER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase $STELLAR_NETWORK_PASSPHRASE \
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
    --network-passphrase $STELLAR_NETWORK_PASSPHRASE \
    --fee $STELLAR_BASE_FEE \
    -- \
    set_lp_token_hash \
    --admin $ADMIN_ADDRESS \
    --new_hash $LP_TOKEN_WASM_HASH

stellar contract invoke \
    --id $POOL_ROUTER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase $STELLAR_NETWORK_PASSPHRASE \
    --fee $STELLAR_BASE_FEE \
    -- \
    set_reward_token \
    --admin $ADMIN_ADDRESS \
    --reward_token $XLM

stellar contract invoke \
    --id $POOL_ROUTER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase $STELLAR_NETWORK_PASSPHRASE \
    --fee $STELLAR_BASE_FEE \
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
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase $STELLAR_NETWORK_PASSPHRASE \
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
    --network-passphrase $STELLAR_NETWORK_PASSPHRASE \
    --fee $STELLAR_BASE_FEE \
    -- \
    set_liquidity_calculator \
    --admin $ADMIN_ADDRESS \
    --calculator $LIQUIDITY_CALCULATOR_ADDR

stellar contract invoke \
    --id $POOL_ROUTER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase $STELLAR_NETWORK_PASSPHRASE \
    --fee $STELLAR_BASE_FEE \
    -- \
    set_oracle_registry \
    --admin $ADMIN_ADDRESS \
    --oracle_registry $ORACLE_REGISTRY_ADDR

echo "Tokens and pool router deployed."

#  _______   ____  ____   _______   _______   _______   _______
# |   _  "\ ("  _||_ " | /"     "| /"     "| /"     "| /"      \
# (. |_)  :)|   (  ) : |(: ______)(: ______)(: ______)|:        |
# |:     \/ (:  |  | . ) \/    |   \/    |   \/    |  |_____/   )
# (|  _  \\  \\ \__/ //  // ___)   // ___)   // ___)_  //      /
# |: |_)  :) /\\ __ //\ (:  (     (:  (     (:      "||:  __   \
# (_______/ (__________) \__/      \__/      \_______)|__|  \___)

echo "Setup buffer..."

ONE_HOUR=$((3600))

stellar contract invoke \
    --id $BUFFER_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase $STELLAR_NETWORK_PASSPHRASE \
    --fee $STELLAR_BASE_FEE \
    -- \
    initialize \
    --admin $ADMIN_ADDRESS \
    --emergency_admin $ADMIN_ADDRESS \
    --time_bt_payouts $ONE_HOUR \
    --min_reserve_ratio 1000

#   __    _____  ___    ________  ____  ____   _______        __      _____  ___    ______    _______
#  |" \  (\"   \|"  \  /"       )("  _||_ " | /"      \      /""\    (\"   \|"  \  /" _  "\  /"     "|
#  ||  | |.\\   \    |(:   \___/ |   (  ) : ||:        |    /    \   |.\\   \    |(: ( \___)(: ______)
#  |:  | |: \.   \\  | \___  \   (:  |  | . )|_____/   )   /' /\  \  |: \.   \\  | \/ \      \/    |
#  |.  | |.  \    \. |  __/  \\   \\ \__/ //  //      /   //  __'  \ |.  \    \. | //  \ _   // ___)_
#  /\  |\|    \    \ | /" \   :)  /\\ __ //\ |:  __   \  /   /  \\  \|    \    \ |(:   _) \ (:      "|
# (__\_|_)\___|\____\)(_______/  (__________)|__|  \___)(___/    \___)\___|\____\) \_______) \_______)

echo "Setup insurance fund..."

THIRTEEN_DAYS=$((ONE_HOUR * 24 * 13))

stellar contract invoke \
    --id $INSURANCE_FUND_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase $STELLAR_NETWORK_PASSPHRASE \
    --fee $STELLAR_BASE_FEE \
    -- \
    initialize \
    --admin $ADMIN_ADDRESS \
    --emergency_admin $ADMIN_ADDRESS \
    --token $XLM \
    --unstaking_period $THIRTEEN_DAYS \
    --optimal_utilization 8000 \
    --base_rate 200 \
    --rate_slopes '[2000, 6000]'

#   _______   _______   _______       ______    ______    ___      ___       _______   ______  ___________  ______     _______
#  /"     "| /"     "| /"     "|     /" _  "\  /    " \  |"  |    |"  |     /"     "| /" _  "\("     _   ")/    " \   /"      \
# (: ______)(: ______)(: ______)    (: ( \___)// ____  \ ||  |    ||  |    (: ______)(: ( \___))__/  \\__// ____  \ |:        |
#  \/    |   \/    |   \/    |       \/ \    /  /    ) :)|:  |    |:  |     \/    |   \/ \        \\_ /  /  /    ) :)|_____/   )
#  // ___)   // ___)_  // ___)_      //  \ _(: (____/ //  \  |___  \  |___  // ___)_  //  \ _     |.  | (: (____/ //  //      /
# (:  (     (:      "|(:      "|    (:   _) \\        /  ( \_|:  \( \_|:  \(:      "|(:   _) \    \:  |  \        /  |:  __   \
#  \__/      \_______) \_______)     \_______)\"_____/    \_______)\_______)\_______) \_______)    \__|   \"_____/   |__|  \___)

echo "Setup fee collector..."

stellar contract invoke \
    --id $POOL_SWAP_FEE \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase $STELLAR_NETWORK_PASSPHRASE \
    --fee $STELLAR_BASE_FEE \
    -- \
    init_admin \
    --admin $ADMIN_ADDRESS \
    --emergency_admin $ADMIN_ADDRESS

stellar contract invoke \
    --id $POOL_SWAP_FEE \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase $STELLAR_NETWORK_PASSPHRASE \
    --fee $STELLAR_BASE_FEE \
    -- \
    set_router \
    --admin $ADMIN_ADDRESS \
    --router $POOL_ROUTER_ADDR

stellar contract invoke \
    --id $POOL_SWAP_FEE \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase $STELLAR_NETWORK_PASSPHRASE \
    --fee $STELLAR_BASE_FEE \
    -- \
    set_buffer \
    --admin $ADMIN_ADDRESS \
    --buffer $BUFFER_ADDR

stellar contract invoke \
    --id $POOL_SWAP_FEE \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase $STELLAR_NETWORK_PASSPHRASE \
    --fee $STELLAR_BASE_FEE \
    -- \
    set_insurance_fund \
    --admin $ADMIN_ADDRESS \
    --insurance_fund $INSURANCE_FUND_ADDR

stellar contract invoke \
    --id $POOL_SWAP_FEE \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase $STELLAR_NETWORK_PASSPHRASE \
    --fee $STELLAR_BASE_FEE \
    -- \
    set_fee_destination \
    --admin $ADMIN_ADDRESS \
    --fee_destination $ADMIN_ADDRESS

echo "#############################"

echo "Initialization complete!"

