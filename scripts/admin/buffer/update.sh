#!/bin/bash
set -e

# Usage instructions
usage() {
    echo "Usage:"
    echo "  $0 <identity_string> <contract_id> <flag> <args...>"
    echo ""
    echo "Flags:"
    echo "  -t <min_time_between_payouts_u64>"
    echo "  -r <min_reserve_ratio_u32>"
    echo "  -m <token_address> <max_balance_u128>"
    exit 1
}

# Validate input
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
-t)
    if [ "$#" -ne 1 ]; then
        echo "Error: -t requires <min_time_between_payouts_u64>"
        exit 1
    fi
    stellar contract invoke \
        --id "$CONTRACT_ID" \
        --source "$IDENTITY_STRING" \
        --network "$NETWORK" \
        -- \
        set_min_time_between_payouts \
        --admin "$ADMIN_ADDRESS" \
        --min_time "$1"
    ;;

-r)
    if [ "$#" -ne 1 ]; then
        echo "Error: -r requires <min_reserve_ratio_u32>"
        exit 1
    fi
    stellar contract invoke \
        --id "$CONTRACT_ID" \
        --source "$IDENTITY_STRING" \
        --network "$NETWORK" \
        -- \
        set_min_reserve_ratio \
        --admin "$ADMIN_ADDRESS" \
        --min_ratio "$1"
    ;;

-m)
    if [ "$#" -ne 2 ]; then
        echo "Error: -m requires <token_address> <max_balance_u128>"
        exit 1
    fi
    stellar contract invoke \
        --id "$CONTRACT_ID" \
        --source "$IDENTITY_STRING" \
        --network "$NETWORK" \
        -- \
        set_reserve_max_balance \
        --admin "$ADMIN_ADDRESS" \
        --token "$1" \
        --max_balance "$2"
    ;;

*)
    echo "Unknown flag: $FLAG"
    usage
    ;;
esac

echo "✅ Buffer contract updated successfully with $FLAG"
