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
INSURANCE_FUND_ADDR="..."

# ONLY set to a value if intending to update it, otherwise leave empty
UNSTAKING_PERIOD=$((60 * 60 * 24 * 13))
OPTIMAL_INSURANCE=1000000_0000000
# rate config
OPTIMAL_UTILIZATION=0
BASE_RATE=0
RATE_SLOPE_A=0
RATE_SLOPE_B=0

# Unstaking period
if [ -n "$UNSTAKING_PERIOD" ]; then
    echo "Updating the Insurance Fund unstaking period..."

    stellar contract invoke \
        --id $INSURANCE_FUND_ADDR \
        --source $IDENTITY_STRING \
        --network $NETWORK \
        -- \
        set_unstaking_period \
        --admin $ADMIN_ADDRESS \
        --unstaking_period $UNSTAKING_PERIOD

    echo "Insurance Fund unstaking period updated."

    # Validation
    RESULT=$(soroban contract invoke \
        --id $INSURANCE_FUND_ADDR \
        --source $IDENTITY_STRING \
        --network $NETWORK --fee 100 \
        -- \
        get_unstaking_period | jq -r '.[0]')

    [ "$UNSTAKING_PERIOD" = "$RESULT" ] || {
        echo "Assertion failed: UNSTAKING_PERIOD != RESULT"
        exit 1
    }
fi

# Optimal insurance
if [ -n "$OPTIMAL_INSURANCE" ]; then
    echo "Updating the Insurance Fund optimal insurance..."

    stellar contract invoke \
        --id $INSURANCE_FUND_ADDR \
        --source $IDENTITY_STRING \
        --network $NETWORK \
        -- \
        set_optimal_insurance \
        --admin $ADMIN_ADDRESS \
        --optimal_insurance $OPTIMAL_INSURANCE

    echo "Insurance Fund optimal insurance updated."

    # Validation
    RESULT=$(soroban contract invoke \
        --id $INSURANCE_FUND_ADDR \
        --source $IDENTITY_STRING \
        --network $NETWORK --fee 100 \
        -- \
        get_optimal_insurance | jq -r '.[0]')

    [ "$OPTIMAL_INSURANCE" = "$RESULT" ] || {
        echo "Assertion failed: OPTIMAL_INSURANCE != RESULT"
        exit 1
    }

fi

# Rate config
if [ -n "$OPTIMAL_UTILIZATION" ]; then
    echo "Updating the Insurance Fund rate config..."

    stellar contract invoke \
        --id $INSURANCE_FUND_ADDR \
        --source $IDENTITY_STRING \
        --network $NETWORK \
        -- \
        set_rate_config \
        --admin $ADMIN_ADDRESS \
        --optimal_utilization $OPTIMAL_UTILIZATION \
        --base_rate $BASE_RATE \
        --rate_slope_a $RATE_SLOPE_A \
        --rate_slope_b $RATE_SLOPE_B

    echo "Insurance Fund rate config updated."

    # Validation
    RESULT=$(soroban contract invoke \
        --id $INSURANCE_FUND_ADDR \
        --source $IDENTITY_STRING \
        --network $NETWORK --fee 100 \
        -- \
        get_optimal_utilization | jq -r '.[0]')

    [ "$OPTIMAL_UTILIZATION" = "$RESULT" ] || {
        echo "Assertion failed: OPTIMAL_UTILIZATION != RESULT"
        exit 1
    }
fi
