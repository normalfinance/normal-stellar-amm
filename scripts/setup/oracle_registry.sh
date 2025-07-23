# Ensure the script exits on any errors
set -e

# Check if the arguments are provided
# Required: identity_string, network, oracle_registry_address
if [ "$#" -lt 2 ]; then
    echo "Usage: $0 <identity_string> <network> <oracle_registry_address>"
    exit 1
fi

IDENTITY_STRING=$1
NETWORK=$2
ORACLE_REGISTRY_ADDR=$3

cd target/wasm32v1-none/release

# Fetch the admin's address
ADMIN_ADDRESS=$(soroban keys address $IDENTITY_STRING)

# Dynamically assign XLM based on network
case "$NETWORK" in
    "testnet")
        XLM="CBBIOHUCWZ6JS5XGZUPJW7CWZXIV2P6YJ53PAX2JGMIQQQ7O3SW2JWFN" # Testnet XLM contract address
    ;;
    "mainnet")
        XLM="CAS3J7GYLGXMF6TDJBBYYSE3HQ6BBSMLNUQ34T6TZMYMW2EVH34XOWMA" # Mainnet XLM contract address
    ;;
    *)
        echo "❌ Unknown network: $NETWORK"
        exit 1
    ;;
esac

echo "Setting up the Oracle Registry..."

stellar contract invoke \
    --id $ORACLE_REGISTRY_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    initialize \
    --admin $ADMIN_ADDRESS \
    --emergency_admin $ADMIN_ADDRESS

stellar contract invoke \
    --id $ORACLE_REGISTRY_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    set_oracle_guard_rails \
    --admin $ADMIN_ADDRESS \
    --oracle_guard_rails '{
        "price_divergence": {
            "oracle_twap_percent_divergence": 1200000000
        },
        "validity": {
            "seconds_before_stale_for_pool": 3000,
            "too_volatile_ratio": 1200000000
        }
    }'

echo "#############################"

echo "Setup complete!"
