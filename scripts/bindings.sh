#!/usr/bin/env bash
# Regenerate TypeScript bindings into bindings/<contract>/ from the built Wasm.
# Run after any change to a contract's public interface.
set -euo pipefail
cd "$(dirname "$0")/.."

./scripts/build.sh

for c in approval-vault invoice-registry; do
  wasm="target/wasm32v1-none/release/${c//-/_}.wasm"
  echo "Generating bindings for $c…"
  stellar contract bindings typescript \
    --wasm "$wasm" \
    --output-dir "bindings/$c" \
    --overwrite
done
