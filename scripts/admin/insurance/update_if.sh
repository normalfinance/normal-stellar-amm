#!/bin/bash
set -e

# Usage instructions
usage() {
    echo "Usage:"
    echo "  $0 <identity_string> <contract_id> <flag> <args...>"
    echo ""
    echo "Flags:"
    echo "  -u <unstaking_period_u64>"
    echo "  -o <optimal_insurance_u128>"
    echo "  -r <optimal_util> <base_rate_i32> <slope_a> <slope_b>"
    exit 1
}

# Ensure minimum args
if [ "$#" -lt 3 ]; then
    usage
fi

# Inputs
CONTRACT_ID="$2"
FLAG="$3"
shift 3

# Config
IDENTITY_STRING="$1"
NETWORK="testnet"
ADMIN_ADDRESS=$(soroban keys address $IDENTITY_STRING)

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
        -- \
        set_optimal_insurance \
        --admin "$ADMIN_ADDRESS" \
        --optimal_insurance "$1"
    ;;

-r)
    if [ "$#" -ne 4 ]; then
        echo "Error: -r requires 4 args: <optimal_utilization_u32> <base_rate_i32> <rate_slope_a> <rate_slope_b>"
        exit 1
    fi
    stellar contract invoke \
        --id "$CONTRACT_ID" \
        --source "$IDENTITY_STRING" \
        --network "$NETWORK" \
        -- \
        set_rate_config \
        --admin "$ADMIN_ADDRESS" \
        --optimal_utilization "$1" \
        --base_rate "$2" \
        --rate_slope_a "$3" \
        --rate_slope_b "$4"
    ;;

*)
    echo "Unknown flag: $FLAG"
    usage
    ;;
esac

echo "✅ Insurance fund updated successfully with $FLAG"
