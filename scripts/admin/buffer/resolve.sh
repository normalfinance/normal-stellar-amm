# Ensure the script exits on any errors
set -e

# Check if the arguments are provided
# Required: identity_string, network, pool_address, fund_type, fund_address
if [ "$#" -lt 4 ]; then
    echo "Usage: $0 <identity_string> <network> <pool_address> <fund_type> <fund_address>"
    exit 1
fi

IDENTITY_STRING=$1
NETWORK=$2
POOL_ROUTER_ADDR=$3
FUND_TYPE=""
FUND_ADDR=$5

# Load env vars dynamically
REPO_ROOT="$(git rev-parse --show-toplevel)"
source "$REPO_ROOT/scripts/load-env.sh" "$NETWORK"

# Parse input flags
while [[ "$#" -gt 0 ]]; do
    case "$3" in
    -b)
        FUND_TYPE="buffer"
        ;;
    -if)
        FUND_TYPE="insurance_fund"
        ;;
    *)
        echo "Unknown option: $1"
        echo "Usage: $0 [-b | -if]"
        exit 1
        ;;
    esac
    shift
done

# Validate selection
if [ -z "$FUND_TYPE" ]; then
    echo "Error: You must specify either -b (buffer) or -if (insurance fund)"
    exit 1
fi

echo "Selected fund: $FUND_TYPE"

if [ -z "$FUND_ADDR" ]; then
    echo "Error: You must specify either -b (buffer) or -if (insurance fund)"
    exit 1
fi

if [ "$FUND_TYPE" = "buffer" ]; then
    echo "Running logic for buffer..."

    stellar contract invoke \
        --id $FUND_ADDR \
        --source $IDENTITY_STRING \
        --network $NETWORK \
        --rpc-url $STELLAR_RPC_URL \
        --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
        --fee $STELLAR_BASE_FEE \
        -- \
        resolve_liquidity_deficit \
        --admin $ADMIN_ADDRESS \
        --token $XLM \
        --amount 0 \
        --pool_address $POOL_ADDR

elif [ "$FUND_TYPE" = "insurance_fund" ]; then
    echo "Running logic for insurance fund..."

    stellar contract invoke \
        --id $FUND_ADDR \
        --source $IDENTITY_STRING \
        --network $NETWORK \
        --rpc-url $STELLAR_RPC_URL \
        --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
        --fee $STELLAR_BASE_FEE \
        -- \
        resolve_liquidity_deficit \
        --admin $ADMIN_ADDRESS \
        --pool_address $POOL_ADDR

fi

echo "Liquidity resolution initiated."
