#!/usr/bin/env bash
# Check that the contracts live on <network> are exactly what we think they are.
#
#   ./scripts/verify-deployment.sh testnet
#
# For each contract in deployments/<network>.json:
#   1. fetch the on-chain Wasm and hash it (a Soroban Wasm hash is its SHA-256)
#   2. compare with the recorded wasmHash
#   3. if a local build exists in target/, compare with that too
# Exits non-zero on any mismatch.
set -euo pipefail
cd "$(dirname "$0")/.."

NETWORK="${1:-}"
[[ "$NETWORK" == "testnet" || "$NETWORK" == "mainnet" ]] || { echo "usage: $0 <testnet|mainnet>" >&2; exit 1; }
command -v stellar >/dev/null || { echo "stellar CLI not found" >&2; exit 1; }
command -v jq >/dev/null || { echo "jq is required" >&2; exit 1; }

FILE="deployments/$NETWORK.json"
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

status=0
for key in approvalVault invoiceRegistry; do
  id=$(jq -r ".$key" "$FILE")
  recorded=$(jq -r ".wasmHash.$key" "$FILE")
  if [[ "$id" == "null" ]]; then
    echo "$key: not deployed on $NETWORK"
    status=1
    continue
  fi

  stellar contract fetch --id "$id" --network "$NETWORK" -o "$TMP/$key.wasm"
  onchain=$(sha256sum "$TMP/$key.wasm" | cut -d' ' -f1)

  if [[ "$onchain" == "$recorded" ]]; then
    echo "$key: on-chain Wasm matches $FILE ($onchain)"
  else
    echo "$key: MISMATCH on-chain $onchain vs recorded $recorded" >&2
    status=1
  fi

  case $key in
    approvalVault) local_wasm=target/wasm32v1-none/release/approval_vault.wasm ;;
    invoiceRegistry) local_wasm=target/wasm32v1-none/release/invoice_registry.wasm ;;
  esac
  if [[ -f "$local_wasm" ]]; then
    local_hash=$(sha256sum "$local_wasm" | cut -d' ' -f1)
    if [[ "$local_hash" == "$onchain" ]]; then
      echo "$key: local build matches on-chain"
    else
      echo "$key: local build differs ($local_hash). Check out sourceRev $(jq -r .sourceRev "$FILE") and rebuild." >&2
      status=1
    fi
  fi
done
exit $status
