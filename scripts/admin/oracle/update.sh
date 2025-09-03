# Ensure the script exits on any errors
set -e

# Usage check
if [ "$#" -ne 5 ]; then
    echo "Usage: $0 <identity_string> <network> <contract_id> [-r|-i|-f|-l] <value>"
    echo "  -r  Set router address"
    echo "  -i  Set insurance fund address"
    echo "  -f  Set fee destination address"
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
REPO_ROOT="$(git rev-parse --show-toplevel)"
source "$REPO_ROOT/scripts/load-env.sh" "$NETWORK"

# Select function based on flag
case "$FLAG" in
-r)
    FUNC="set_router"
    ARG_NAME="--router"
    ;;
-i)
    FUNC="set_insurance_fund"
    ARG_NAME="--insurance_fund"
    ;;
-f)
    FUNC="set_fee_destination"
    ARG_NAME="--fee_destination"
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
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE \
    -- \
    "$FUNC" \
    --admin "$ADMIN_ADDRESS" \
    "$ARG_NAME" "$VALUE"

echo "✅ $FUNC updated successfully."
