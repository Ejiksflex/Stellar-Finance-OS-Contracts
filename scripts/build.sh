#!/usr/bin/env bash
# Build all contracts to optimized Wasm via the Stellar CLI.
# Output: target/wasm32v1-none/release/{approval_vault,invoice_registry}.wasm
set -euo pipefail
cd "$(dirname "$0")/.."

command -v stellar >/dev/null || { echo "stellar CLI not found: https://developers.stellar.org/docs/tools/cli" >&2; exit 1; }

stellar contract build "$@"
ls -lh target/wasm32v1-none/release/*.wasm
