#!/bin/bash
set -e

# Usage check
if [ "$#" -ne 4 ]; then
    echo "Usage: $0 <identity_string> <contract_id> [-a|-d|-f|-c] <value>"
    echo "  -a  Set oracle address"
    echo "  -d  Set decimals (u32)"
    echo "  -f  Set frozen status (bool)"
    echo "  -c  Set clamp (i64)"
    exit 1
fi

# Inputs
CONTRACT_ID="$2"
FLAG="$3"
VALUE="$4"

# Config
IDENTITY_STRING=$1
NETWORK="testnet"
ADMIN_ADDRESS=$(soroban keys address $IDENTITY_STRING)

# Select function based on flag
case "$FLAG" in
-r)
    FUNC="set_router"
    ARG_NAME="--router"
    ;;
-b)
    FUNC="set_buffer"
    ARG_NAME="--buffer"
    ;;
-i)
    FUNC="set_insurance_fund"
    ARG_NAME="--insurance_fund"
    ;;
-f)
    FUNC="set_fee_destination"
    ARG_NAME="--fee_destination"
    ;;
-u)
    FUNC="set_buffer_fraction"
    ARG_NAME="--fraction"
    ;;
-l)
    FUNC="set_lp_revenue_fraction"
    ARG_NAME="--fraction"
    ;;
*)
    echo "Unknown flag: $FLAG"
    exit 1
    ;;
esac

echo "Calling $FUNC with value: $VALUE"

# Call contract
stellar contract invoke \
    --id "$CONTRACT_ID" \
    --source "$IDENTITY_STRING" \
    --network "$NETWORK" \
    -- \
    "$FUNC" \
    --admin "$ADMIN_ADDRESS" \
    "$ARG_NAME" "$VALUE"

echo "✅ $FUNC updated successfully."
