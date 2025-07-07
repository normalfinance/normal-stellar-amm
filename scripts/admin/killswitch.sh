#!/bin/bash
set -e

IDENTITY_STRING=$1
NETWORK="testnet"
ADMIN_ADDRESS=$(soroban keys address $IDENTITY_STRING)

usage() {
    echo "Usage:"
    echo "  $0 <identity_string> <contract_id> <kill|unkill|status> <target>"
    echo ""
    echo "Supported targets:"
    echo "  deposit"
    echo "  withdraw"
    echo "  request_withdraw       (Insurance Fund only)"
    echo "  swap                   (Pool only)"
    echo "  claim                  (Pool only)"
    echo "  resolve_deficit        (Buffer only)"
    exit 1
}

# Validate inputs
if [ "$#" -ne 4 ]; then
    usage
fi

CONTRACT_ID="$2"
ACTION="$3"
TARGET="$4"

# Map target to function names
case "$TARGET" in
deposit)
    FUNC_KILL="kill_deposit"
    FUNC_UNKILL="unkill_deposit"
    FUNC_STATUS="get_is_killed_deposit"
    ;;
withdraw)
    FUNC_KILL="kill_withdraw"
    FUNC_UNKILL="unkill_withdraw"
    FUNC_STATUS="get_is_killed_withdraw"
    ;;
request_withdraw)
    FUNC_KILL="kill_request_withdraw"
    FUNC_UNKILL="unkill_request_withdraw"
    FUNC_STATUS="get_is_killed_request_withdraw"
    ;;
swap)
    FUNC_KILL="kill_swap"
    FUNC_UNKILL="unkill_swap"
    FUNC_STATUS="get_is_killed_swap"
    ;;
claim)
    FUNC_KILL="kill_claim"
    FUNC_UNKILL="unkill_claim"
    FUNC_STATUS="get_is_killed_claim"
    ;;
resolve_deficit)
    FUNC_KILL="kill_resolve_liquidity_deficit"
    FUNC_UNKILL="unkill_resolve_liquidity_deficit"
    FUNC_STATUS="get_is_killed_resolve_deficit"
    ;;
*)
    echo "❌ Unknown target: $TARGET"
    usage
    ;;
esac

# Perform the action
case "$ACTION" in
kill)
    echo "☠️  Calling $FUNC_KILL on contract $CONTRACT_ID"
    stellar contract invoke \
        --id "$CONTRACT_ID" \
        --source "$IDENTITY_STRING" \
        --network "$NETWORK" \
        -- \
        "$FUNC_KILL" \
        --admin "$ADMIN_ADDRESS"
    ;;
unkill)
    echo "🛠️  Calling $FUNC_UNKILL on contract $CONTRACT_ID"
    stellar contract invoke \
        --id "$CONTRACT_ID" \
        --source "$IDENTITY_STRING" \
        --network "$NETWORK" \
        -- \
        "$FUNC_UNKILL" \
        --admin "$ADMIN_ADDRESS"
    ;;
status)
    echo "🔍 Checking $FUNC_STATUS on contract $CONTRACT_ID"
    stellar contract invoke \
        --id "$CONTRACT_ID" \
        --source "$IDENTITY_STRING" \
        --network "$NETWORK" \
        -- \
        "$FUNC_STATUS"
    ;;
*)
    echo "❌ Unknown action: $ACTION"
    usage
    ;;
esac

echo "✅ Action '$ACTION' on '$TARGET' completed."
