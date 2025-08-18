# Ensure the script exits on any errors
set -e

# Usage check
if [ "$#" -ne 5 ]; then
    echo "Usage: $0 <identity_string> <network> <contract_id> [-r|-b|-i|-f|-u|-l] <value>"
    echo "  -r  Set router address"
    echo "  -b  Set buffer address"
    echo "  -i  Set insurance fund address"
    echo "  -f  Set fee destination address"
    echo "  -u  Set buffer fraction (u32)"
    echo "  -l  Set LP revenue fraction (u32)"
    exit 1
fi

# Inputs
CONTRACT_ID="$3"
FLAG="$4"
VALUE="$5"

# Config
IDENTITY_STRING=$1
NETWORK=$2
ADMIN_ADDRESS=$(soroban keys address $IDENTITY_STRING)

# Load env vars dynamically
source "$(dirname "${BASH_SOURCE[0]}")/load-env.sh" "$NETWORK"

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
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase $STELLAR_NETWORK_PASSPHRASE \
    --fee $STELLAR_BASE_FEE \
    -- \
    "$FUNC" \
    --admin "$ADMIN_ADDRESS" \
    "$ARG_NAME" "$VALUE"

echo "✅ $FUNC updated successfully."
