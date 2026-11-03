#!/usr/bin/env bash
# Build all contracts to optimized Wasm via the Stellar CLI.
# Output: target/wasm32v1-none/release/{approval_vault,invoice_registry}.wasm
set -euo pipefail
cd "$(dirname "$0")/.."

command -v stellar >/dev/null || { echo "stellar CLI not found: https://developers.stellar.org/docs/tools/cli" >&2; exit 1; }

# soroban-sdk 29 refuses to build with stellar-cli < 25.2.0.
MIN_CLI="25.2.0"
CLI_VERSION=$(stellar --version | head -n1 | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -n1)
if [[ "$(printf '%s\n%s\n' "$MIN_CLI" "$CLI_VERSION" | sort -V | head -n1)" != "$MIN_CLI" ]]; then
  echo "stellar CLI $CLI_VERSION is too old; need >= $MIN_CLI" >&2
  exit 1
fi

# Embed provenance in each Wasm's contractmeta section so a deployed contract
# can be traced back to its exact source (SEP-55 style `source_repo`).
META=(--meta "source_repo=github:Stellar-Finance-OS/Stellar-Finance-OS-Contracts")
if REV=$(git rev-parse HEAD 2>/dev/null); then
  META+=(--meta "source_rev=$REV")
fi

stellar contract build "${META[@]}" "$@"
ls -lh target/wasm32v1-none/release/*.wasm
