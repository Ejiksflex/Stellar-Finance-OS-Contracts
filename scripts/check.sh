#!/usr/bin/env bash
# The "done" gate from AGENTS.md rule 6: fmt, clippy (deny warnings), tests.
set -euo pipefail
cd "$(dirname "$0")/.."

cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
