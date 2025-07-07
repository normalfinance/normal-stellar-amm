# Ensure the script exits on any errors
set -e

# Check if the argument is provided
if [ -z "$1" ]; then
    echo "Usage: $0 <identity_string>"
    exit 1
fi

IDENTITY_STRING=$1
NETWORK="testnet"

# Config
CONTRACT_TYPE="..."
CONTRACT_ADDR="..."

stellar contract invoke \
    --id $CONTRACT_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    apply_upgrade \
    --admin $ADMIN_ADDRESS

echo "$CONTRACT_TYPE upgrade applied."
