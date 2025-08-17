
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
# stellar tx new change-trust \
#     --source-account "$ISSUER" \
#     --network "$NETWORK" \

# Deploy the built-in SAC contract for the new asset
stellar contract asset deploy \
    --source "$ISSUER" \
    --network "$NETWORK" \
    --asset nSOL:GAW562S4FWKNB5EOA7AH4Z5DLQT243BFHM2MX6UC3NL3RAKEASDDMBQU

# stellar contract asset deploy \
#     --source "$ISSUER" \
#     --network "$NETWORK" \
#     --rpc-url "https://rpc.ankr.com/stellar_soroban/<api_key>" \
#     --network-passphrase "Public Global Stellar Network ; September 2015" \
#     --asset nSOL:GAW562S4FWKNB5EOA7AH4Z5DLQT243BFHM2MX6UC3NL3RAKEASDDMBQU