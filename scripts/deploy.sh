#!/usr/bin/env bash
# Deploy both contracts and write deployments/<network>.json.
#
# Usage:
#   STELLAR_ACCOUNT=finance-os-deployer \
#   VAULT_APPROVERS='["G...","G..."]' VAULT_THRESHOLD=2 \
#   ./scripts/deploy.sh testnet
#
# STELLAR_ACCOUNT is read by the stellar CLI itself (as --source-account). It
# can be a `stellar keys` identity name or, in CI, a secret key from a
# repository secret. It never appears in command arguments. Create a local
# identity with:
#   stellar keys generate finance-os-deployer --network testnet --fund
# Never commit identities.
#
# Set SKIP_BUILD=1 to deploy Wasm already in target/ (e.g. built by CI).
set -euo pipefail
cd "$(dirname "$0")/.."

NETWORK="${1:-}"
case "$NETWORK" in
  testnet) ;;
  mainnet)
    # README §5: no mainnet deployment without an external audit.
    if [[ "${FINANCE_OS_MAINNET_AUDITED:-}" != "yes" ]]; then
      echo "Refusing mainnet deploy: contracts must be externally audited first (README §5)." >&2
      echo "Set FINANCE_OS_MAINNET_AUDITED=yes only once an audit report is in hand." >&2
      exit 1
    fi ;;
  *) echo "usage: $0 <testnet|mainnet>" >&2; exit 1 ;;
esac

: "${STELLAR_ACCOUNT:?set STELLAR_ACCOUNT to a stellar keys identity name or secret key}"
export STELLAR_ACCOUNT
: "${VAULT_APPROVERS:?set VAULT_APPROVERS to a JSON array of approver addresses}"
: "${VAULT_THRESHOLD:?set VAULT_THRESHOLD}"
command -v jq >/dev/null || { echo "jq is required" >&2; exit 1; }

# Fail fast on bad constructor input, before spending fees on uploads.
# The contract re-checks all of this; these checks just give clearer errors.
if ! jq -e 'type == "array" and length > 0 and all(.[]; type == "string" and test("^[GC][A-Z2-7]{55}$"))' \
    <<<"$VAULT_APPROVERS" >/dev/null 2>&1; then
  echo "VAULT_APPROVERS must be a non-empty JSON array of G... or C... addresses" >&2
  exit 1
fi
APPROVER_COUNT=$(jq 'length' <<<"$VAULT_APPROVERS")
if [[ $(jq '(map(.) | unique | length)' <<<"$VAULT_APPROVERS") != "$APPROVER_COUNT" ]]; then
  echo "VAULT_APPROVERS contains duplicates" >&2
  exit 1
fi
if ! [[ "$VAULT_THRESHOLD" =~ ^[0-9]+$ ]] || (( VAULT_THRESHOLD < 1 || VAULT_THRESHOLD > APPROVER_COUNT )); then
  echo "VAULT_THRESHOLD must be between 1 and $APPROVER_COUNT" >&2
  exit 1
fi

if [[ "${SKIP_BUILD:-}" != "1" ]]; then
  ./scripts/build.sh
fi

WASM_DIR=target/wasm32v1-none/release
NET_ARGS=(--network "$NETWORK")

upload() { stellar contract upload --wasm "$1" "${NET_ARGS[@]}" | tail -n1; }

echo "Uploading Wasm…"
VAULT_HASH=$(upload "$WASM_DIR/approval_vault.wasm")
REGISTRY_HASH=$(upload "$WASM_DIR/invoice_registry.wasm")

echo "Deploying approval-vault…"
# --alias lets you run e.g. `stellar contract invoke --id approval-vault ...`
# locally after deploying.
VAULT_ID=$(stellar contract deploy --wasm-hash "$VAULT_HASH" "${NET_ARGS[@]}" --alias approval-vault \
  -- --approvers "$VAULT_APPROVERS" --threshold "$VAULT_THRESHOLD" | tail -n1)

echo "Deploying invoice-registry…"
REGISTRY_ID=$(stellar contract deploy --wasm-hash "$REGISTRY_HASH" "${NET_ARGS[@]}" --alias invoice-registry | tail -n1)

OUT="deployments/$NETWORK.json"
jq -n \
  --arg network "$NETWORK" \
  --arg vault "$VAULT_ID" --arg registry "$REGISTRY_ID" \
  --arg vaultHash "$VAULT_HASH" --arg registryHash "$REGISTRY_HASH" \
  --arg version "$(git describe --tags --always --dirty 2>/dev/null || echo unknown)" \
  --arg sourceRev "$(git rev-parse HEAD 2>/dev/null || echo unknown)" \
  --arg cliVersion "$(stellar --version | head -n1)" \
  --arg deployedAt "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  '{network: $network, version: $version, sourceRev: $sourceRev, cliVersion: $cliVersion, deployedAt: $deployedAt,
    approvalVault: $vault, invoiceRegistry: $registry,
    wasmHash: {approvalVault: $vaultHash, invoiceRegistry: $registryHash}}' > "$OUT"

echo "Wrote $OUT"
cat "$OUT"
