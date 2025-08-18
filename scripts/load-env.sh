#!/bin/bash
set -e

# $1 is the network argument (e.g., "testnet" or "mainnet")
NETWORK="$1"

if [ -z "$NETWORK" ]; then
  echo "Usage: source load-env.sh [testnet|mainnet]"
  return 1
fi

case "$NETWORK" in
  testnet)
    ENV_FILE=".env.testnet"
    ;;
  mainnet)
    ENV_FILE=".env.mainnet"
    ;;
  *)
    echo "Error: Unknown network '$NETWORK'. Use 'testnet' or 'mainnet'."
    return 1
    ;;
esac

if [ -f "$ENV_FILE" ]; then
  echo "Loading environment from $ENV_FILE"
  source "$ENV_FILE"
else
  echo "Error: $ENV_FILE not found"
  return 1
fi
