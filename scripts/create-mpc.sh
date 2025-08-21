# Ensure the script exits on any errors
set -e

# Load env vars dynamically
source "$(dirname "${BASH_SOURCE[0]}")/load-env.sh" "$NETWORK"

# ---------- Config ----------


BASE_SECRET="<MAIN_ACCOUNT_SECRET>"          # G... secret of the original account
SIGNER_1_PUBLIC="<SIGNER_1_PUBLIC_KEY>"      # G... of new signer 1
SIGNER_2_PUBLIC="<SIGNER_2_PUBLIC_KEY>"      # G... of new signer 2
# ----------------------------

# Fetch base public key from secret
BASE_PUBLIC=$(stellar keys address --secret "$BASE_SECRET")

echo "Base account: $BASE_PUBLIC"
echo "Adding signers: $SIGNER_1_PUBLIC, $SIGNER_2_PUBLIC"

# Generate operation XDR
stellar operation set-options \
  --source "$BASE_PUBLIC" \
  --master-weight 1 \
  --low-threshold 1 \
  --med-threshold 2 \
  --high-threshold 2 \
  --signer "$SIGNER_1_PUBLIC,1" \
  --signer "$SIGNER_2_PUBLIC,1" \
  --output-file op.xdr

# Build transaction XDR
stellar transaction build \
  --source "$BASE_PUBLIC" \
  --rpc-url "$RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE" \
  --add-operation op.xdr \
  --output-file tx.xdr

# Sign transaction
stellar transaction sign \
  --xdr tx.xdr \
  --secret "$BASE_SECRET" \
  --output tx_signed.xdr

# Submit transaction
stellar transaction submit \
  --xdr tx_signed.xdr \
  --rpc-url "$RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE"

echo "✅ Multisig setup complete. Thresholds set to 2-of-3 (including master key)."
