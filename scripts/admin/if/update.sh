# Ensure the script exits on any errors
set -e

# Usage instructions
usage() {
    echo "Usage:"
    echo "  $0 <identity_string> <network> <contract_id> <flag> <args...>"
    echo ""
    echo "Flags:"
    echo "  -u <unstaking_period_u64>"
    echo "  -o <optimal_insurance_u128>"
    echo "  -r <optimal_util> <base_rate_i32> <slope_a> <slope_b>"
    exit 1
}

# Ensure minimum args
if [ "$#" -lt 4 ]; then
    usage
fi

# Inputs
IDENTITY_STRING="$1"
NETWORK="$2"
CONTRACT_ID="$3"
FLAG="$4"

# Load env vars dynamically
source "$(dirname "${BASH_SOURCE[0]}")/load-env.sh" "$NETWORK"

ADMIN_ADDRESS=$(soroban keys address $IDENTITY_STRING)

shift 4

case "$FLAG" in
-u)
    if [ "$#" -ne 1 ]; then
        echo "Error: -u requires <unstaking_period_u64>"
        exit 1
    fi
    stellar contract invoke \
        --id "$CONTRACT_ID" \
        --source "$IDENTITY_STRING" \
        --network "$NETWORK" \
        --rpc-url $STELLAR_RPC_URL \
        --network-passphrase $STELLAR_NETWORK_PASSPHRASE \
        --fee $STELLAR_BASE_FEE \
        -- \
        set_unstaking_period \
        --admin "$ADMIN_ADDRESS" \
        --unstaking_period "$1"
    ;;

-o)
    if [ "$#" -ne 1 ]; then
        echo "Error: -o requires <optimal_insurance_u128>"
        exit 1
    fi
    stellar contract invoke \
        --id "$CONTRACT_ID" \
        --source "$IDENTITY_STRING" \
        --network "$NETWORK" \
        --rpc-url $STELLAR_RPC_URL \
        --network-passphrase $STELLAR_NETWORK_PASSPHRASE \
        --fee $STELLAR_BASE_FEE \
        -- \
        set_optimal_insurance \
        --admin "$ADMIN_ADDRESS" \
        --optimal_insurance "$1"
    ;;

-r)
    if [ "$#" -ne 4 ]; then
        echo "Error: -r requires 4 args: <optimal_utilization_u32> <base_rate> <rate_slope_a> <rate_slope_b>"
        exit 1
    fi
    echo $4
    stellar contract invoke \
        --id $CONTRACT_ID \
        --source $IDENTITY_STRING \
        --network $NETWORK \
        --rpc-url $STELLAR_RPC_URL \
        --network-passphrase $STELLAR_NETWORK_PASSPHRASE \
        --fee $STELLAR_BASE_FEE \
        -- \
        set_rate_config \
        --admin $ADMIN_ADDRESS \
        --optimal_utilization $1 \
        --base_rate $2 \
        --rate_slope_a $3 \
        --rate_slope_b $4
    ;;

*)
    echo "Unknown flag: $FLAG"
    usage
    ;;
esac

echo "✅ Insurance fund updated successfully with $FLAG"
