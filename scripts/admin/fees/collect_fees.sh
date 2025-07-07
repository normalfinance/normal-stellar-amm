# Ensure the script exits on any errors
set -e

# Check if the arguments are provided
# Required: identity_string, pool_swap_fee_address
if [ "$#" -lt 2 ]; then
    echo "Usage: $0 <identity_string> <pool_swap_fee_address>"
    exit 1
fi

# mainnet = "CAS3J7GYLGXMF6TDJBBYYSE3HQ6BBSMLNUQ34T6TZMYMW2EVH34XOWMA"
XLM="CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC"

IDENTITY_STRING=$1
POOL_SWAP_FEE_ADDR=$2
NETWORK="testnet"

stellar contract invoke \
    --id $POOL_SWAP_FEE_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    claim_fees \
    --admin $ADMIN_ADDRESS \
    --token $XLM

echo "Fees collected."
