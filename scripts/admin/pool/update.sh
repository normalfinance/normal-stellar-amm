# Ensure the script exits on any errors
set -e

# Usage guide
usage() {
    echo "Usage:"
    echo "  $0 <identity_string> <network> <contract_id> <flag> <args...>"
    echo ""
    echo "Flags:"
    echo "  -p <rewards_admin> <operations_admin> <pause_admin> <emergency_admin1,admin2,...>"
    echo "  -f <fee_fraction_u32>"
    echo "  -t <tier>                # e.g. A, B, C"
    echo "  -s <status>              # e.g. Active, Paused, Closed"
    echo "  -m <liq_max> <quote_max> # max imbalances (u128s)"
    exit 1
}

# Ensure minimum arguments
if [ "$#" -lt 4 ]; then
    usage
fi

# Inputs
CONTRACT_ID="$2"
FLAG="$3"
shift 3

# Config
NETWORK="$2"
ADMIN_ADDRESS=$(soroban keys address $IDENTITY_STRING)
IDENTITY_STRING="$1"

# Load env vars dynamically
REPO_ROOT="$(git rev-parse --show-toplevel)"
source "$REPO_ROOT/scripts/load-env.sh" "$NETWORK"

case "$FLAG" in
-p)
    if [ "$#" -ne 4 ]; then
        echo "Error: -p requires 4 args: <rewards_admin> <operations_admin> <pause_admin> <emergency_admins_comma_sep>"
        exit 1
    fi
    REWARDS_ADMIN="$1"
    OPERATIONS_ADMIN="$2"
    PAUSE_ADMIN="$3"
    IFS=',' read -ra EMERGENCY_ADMINS <<<"$4"
    ADDR_ARGS=()
    for addr in "${EMERGENCY_ADMINS[@]}"; do
        ADDR_ARGS+=("--emergency_pause_admins" "$addr")
    done

    stellar contract invoke \
        --id "$CONTRACT_ID" \
        --source "$IDENTITY_STRING" \
        --network "$NETWORK" \
        --rpc-url $STELLAR_RPC_URL \
        --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
        --fee $STELLAR_BASE_FEE \
        -- \
        set_privileged_addrs \
        --admin "$ADMIN_ADDRESS" \
        --rewards_admin "$REWARDS_ADMIN" \
        --operations_admin "$OPERATIONS_ADMIN" \
        --pause_admin "$PAUSE_ADMIN" \
        "${ADDR_ARGS[@]}"
    ;;

-f)
    if [ "$#" -ne 1 ]; then
        echo "Error: -f requires <fee_fraction_u32>"
        exit 1
    fi
    stellar contract invoke \
        --id "$CONTRACT_ID" \
        --source "$IDENTITY_STRING" \
        --network "$NETWORK" \
        --rpc-url $STELLAR_RPC_URL \
        --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
        --fee $STELLAR_BASE_FEE \
        -- \
        set_fee \
        --admin "$ADMIN_ADDRESS" \
        --fee_fraction "$1"
    ;;

-t)
    if [ "$#" -ne 1 ]; then
        echo "Error: -t requires <tier>"
        exit 1
    fi
    stellar contract invoke \
        --id "$CONTRACT_ID" \
        --source "$IDENTITY_STRING" \
        --network "$NETWORK" \
        --rpc-url $STELLAR_RPC_URL \
        --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
        --fee $STELLAR_BASE_FEE \
        -- \
        set_tier \
        --admin "$ADMIN_ADDRESS" \
        --tier "$1"
    ;;

-s)
    if [ "$#" -ne 1 ]; then
        echo "Error: -s requires <status>"
        exit 1
    fi
    stellar contract invoke \
        --id "$CONTRACT_ID" \
        --source "$IDENTITY_STRING" \
        --network "$NETWORK" \
        --rpc-url $STELLAR_RPC_URL \
        --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
        --fee $STELLAR_BASE_FEE \
        -- \
        set_status \
        --admin "$ADMIN_ADDRESS" \
        --status "$1"
    ;;

-m)
    if [ "$#" -ne 2 ]; then
        echo "Error: -m requires <liquidity_max_imbalance> <max_insurance>"
        exit 1
    fi
    stellar contract invoke \
        --id "$CONTRACT_ID" \
        --source "$IDENTITY_STRING" \
        --network "$NETWORK" \
        --rpc-url $STELLAR_RPC_URL \
        --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
        --fee $STELLAR_BASE_FEE \
        -- \
        set_max_imbalances \
        --admin "$ADMIN_ADDRESS" \
        --liquidity_max_imbalance "$1" \
        --max_insurance "$2"
    ;;

*)
    echo "Unknown flag: $FLAG"
    usage
    ;;
esac

echo "✅ Contract updated successfully with $FLAG"
