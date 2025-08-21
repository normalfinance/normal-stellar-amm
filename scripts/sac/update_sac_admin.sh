# Ensure the script exits on any errors
set -e

# Usage
usage() {
    echo "Usage:"
    echo "  $0 <issuer> <network> <sac_address> <pool_address>"
    echo ""
    echo "Example:"
    echo "  $0 admin testnet CAS123 CAS123"
    exit 1
}

# Validate args
if [ "$#" -ne 4 ]; then
    usage
fi

# Parse arguments
ISSUER=$1
NETWORK=$2
SAC_ADDRESS=$3
POOL_ADDRESS=$4

# Load env vars dynamically
source "$(dirname "${BASH_SOURCE[0]}")/load-env.sh" "$NETWORK"

# Get issuer address
ISSUER_ADDRESS=$(soroban keys address "$ISSUER")

# Update the SAC admin to the Pool address
stellar contract invoke \
    --source-account "$ISSUER" \
    --network "$NETWORK" \
    --id $SAC_ADDRESS \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE \
    -- \
    set_admin \
    --new_admin $POOL_ADDRESS