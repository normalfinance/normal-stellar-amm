# Ensure the script exits on any errors
set -e

# TODO: this script is not complete!

# Check if the arguments are provided
# Required: identity_string, network, contract_name
if [ "$#" -lt 3 ]; then
    echo "Usage: $0 <identity_string> <network> <contract_name>"
    exit 1
fi

IDENTITY_STRING=$1
NETWORK=$2
CONTRACT_NAME=$3

# Load env vars dynamically
source "$(dirname "${BASH_SOURCE[0]}")/load-env.sh" "$NETWORK"

# ------------------- USER CONFIGURATION --------------------
WASM_FILE="../wasm/$CONTRACT_NAME.optimized.wasm"         # Path to your optimized .wasm file
CONTRACT_HASH="cafebabe..."                         # Contract hash (hex)
REPO_URL="https://github.com/normalfinance/normal-stellar-amm"    # GitHub repo
COMMIT_HASH="abcdef1234567890"                      # Git commit SHA
JOB_ID="build-job"                                  # CI/CD job name
RUN_ID="123456789"                                  # GitHub Actions or CI run ID

# Optional metadata
RELATIVE_PATH="contracts/$CONTRACT_NAME"
PACKAGE_NAME=$CONTRACT_NAME
MAKE_TARGET=""
# -----------------------------------------------------------

# Print input summary
echo "Submitting contract to Stellar Expert with:"
echo "- WASM: $WASM_FILE"
echo "- Contract Hash: $CONTRACT_HASH"
echo "- Repo: $REPO_URL"
echo "- Commit: $COMMIT_HASH"

# Build JSON payload
JSON_PAYLOAD=$(jq -n \
  --arg repo "$REPO_URL" \
  --arg commit "$COMMIT_HASH" \
  --arg job "$JOB_ID" \
  --arg run "$RUN_ID" \
  --arg hash "$CONTRACT_HASH" \
  --arg relPath "$RELATIVE_PATH" \
  --arg package "$PACKAGE_NAME" \
  --arg make "$MAKE_TARGET" \
  '{
    repository: $repo,
    commitHash: $commit,
    jobId: $job,
    runId: $run,
    contractHash: $hash,
    relativePath: $relPath,
    packageName: $package,
    makeTarget: $make
  }'
)

echo "Sending JSON to Stellar Expert:"
echo "$JSON_PAYLOAD"

# Submit to Stellar Expert
# curl -X POST "https://api.stellar.expert/explorer/public/contract-validation/match" \
#      -H "Content-Type: application/json" \
#      -d "$JSON_PAYLOAD" \
#      --max-time 15

echo "✅ Submission complete."
