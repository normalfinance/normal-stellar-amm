
#!/bin/bash
set -e

# Usage
usage() {
    echo "Usage:"
    echo "  $0 <issuer> <network>"
    echo ""
    echo "Example:"
    echo "  $0 josh CAS123"
    exit 1
}

# Validate args
if [ "$#" -ne 2 ]; then
    usage
fi

# Parse arguments
ISSUER=$1
NETWORK=$2

# Get admin address
ISSUER_ADDRESS=$(soroban keys address "$ISSUER")

# Locks the asset issuer so no additional tokens can be minted by it
stellar tx new set-options \
    --source-account "$ISSUER" \
    --network "$NETWORK" \
    --master-weight 0