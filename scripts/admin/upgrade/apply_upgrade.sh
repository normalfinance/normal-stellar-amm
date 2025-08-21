# Ensure the script exits on any errors
set -e

# Check if the argument is provided
if [ -z "$1" ]; then
    echo "Usage: $0 <identity_string> <network>"
    exit 1
fi

IDENTITY_STRING=$1
NETWORK="$2"

# Load env vars dynamically
source "$(dirname "${BASH_SOURCE[0]}")/load-env.sh" "$NETWORK"

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
