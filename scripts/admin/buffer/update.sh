# Ensure the script exits on any errors
set -e

# Usage instructions
usage() {
    echo "Usage:"
    echo "  $0 <identity_string> <network> <contract_id> <flag> <args...>"
    echo ""
    echo "Flags:"
    echo "  -t <min_time_between_payouts_u64>"
    echo "  -r <min_reserve_ratio_u32>"
    echo "  -m <token_address> <max_balance_u128>"
    exit 1
}

# Validate input
if [ "$#" -lt 4 ]; then
    usage
fi

# Inputs
CONTRACT_ID="$3"
FLAG="$4"
shift 3

# Config
IDENTITY_STRING="$1"
NETWORK="$2"
ADMIN_ADDRESS=$(soroban keys address $IDENTITY_STRING)

# Load env vars dynamically
source "$(dirname "${BASH_SOURCE[0]}")/load-env.sh" "$NETWORK"

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
        --rpc-url $STELLAR_RPC_URL \
        --network-passphrase $STELLAR_NETWORK_PASSPHRASE \
        --fee $STELLAR_BASE_FEE \
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
        --rpc-url $STELLAR_RPC_URL \
        --network-passphrase $STELLAR_NETWORK_PASSPHRASE \
        --fee $STELLAR_BASE_FEE \
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
        --rpc-url $STELLAR_RPC_URL \
        --network-passphrase $STELLAR_NETWORK_PASSPHRASE \
        --fee $STELLAR_BASE_FEE \
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
