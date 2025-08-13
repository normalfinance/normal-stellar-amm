
#!/bin/bash
set -e

# Usage
usage() {
    echo "Usage:"
    echo "  $0 <issuer> <network>"
    echo ""
    echo "Example:"
    echo "  $0 josh CAS123 BTC"
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

# Issue an asset by creating a trustline
stellar tx new change-trust \
    --source-account "$ISSUER" \
    --network "$NETWORK" \

# Deploy the built-in SAC contract for the new asset
stellar contract asset deploy \
    --source-account "$ISSUER" \
    --network "$NETWORK" \
    --asset STAR:GCS5NEHKJALCSVJAKIORXXVS554QQV5FNDLBK33CCAH6UIRYPXYZFC34