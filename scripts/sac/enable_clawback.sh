# Ensure the script exits on any errors
set -e

# Usage
usage() {
    echo "Usage:"
    echo "  $0 <issuer> <network> <symbol>"
    echo ""
    echo "Example:"
    echo "  $0 admin testnet nSOL"
    exit 1
}

# Validate args
if [ "$#" -ne 3 ]; then
    usage
fi

# Parse arguments
ISSUER=$1
NETWORK=$2
SYMBOL=$3

# Load env vars dynamically
REPO_ROOT="$(git rev-parse --show-toplevel)"
source "$REPO_ROOT/scripts/load-env.sh" "$NETWORK"

# Get admin address
ISSUER_ADDRESS=$(soroban keys address "$ISSUER")

# Issue an asset by creating a trustline
# stellar tx new set-trustline-flags \
#     --source-account "GCDPXHV4QZPDRCP65CKDAA4JBBNMDO5LTO3TCHC6HXGZV54HN63K3KY5" \
#     --network "testnet" \
#     --trustor "CAGD5SCICK23F65YUUGCRWSYGAQM4VPY36LG5WG2KQTMYCBOYD5WGO4W" \
#     --asset "nETH:GCDPXHV4QZPDRCP65CKDAA4JBBNMDO5LTO3TCHC6HXGZV54HN63K3KY5" \
#     --set-trustline-clawback-enabled


stellar tx new set-options \
    --source-account "$ISSUER" \
    --network "$NETWORK" \
    --rpc-url $STELLAR_RPC_URL \
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --fee $STELLAR_BASE_FEE \
    --set-revocable \
    --set-clawback-enabled