#!/usr/bin/env bash
# Test coverage report (audit prep, build-order task 9).
# Requires: rustup component add llvm-tools-preview && cargo install cargo-llvm-cov
#
#   ./scripts/coverage.sh          # summary table (contract code only)
#   ./scripts/coverage.sh --html   # also writes target/llvm-cov/html/index.html
set -euo pipefail
cd "$(dirname "$0")/.."

command -v cargo-llvm-cov >/dev/null || {
  echo "cargo-llvm-cov not found: cargo install cargo-llvm-cov" >&2
  exit 1
}

# Exclude test modules themselves from the numbers.
IGNORE='(^|/)test\.rs$'

cargo llvm-cov --workspace --ignore-filename-regex "$IGNORE" --summary-only
if [[ "${1:-}" == "--html" ]]; then
  cargo llvm-cov report --ignore-filename-regex "$IGNORE" --html
  echo "HTML report: target/llvm-cov/html/index.html"
fi
