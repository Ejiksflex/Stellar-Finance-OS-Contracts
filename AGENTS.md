# AGENTS.md — rules for AI agents working in finance-os-contracts

Read `README.md` first. The function tables (§3, §4) and security requirements (§5) are the spec.

## Rules

1. Every state-changing entrypoint gets a test for the happy path **and** each failure path in README §5.
2. Do not use `mock_all_auths()` in a test unless the test name says so; prefer explicit auth mocking (`env.mock_auths(&[...])`) so auth bugs aren't hidden.
3. No secret keys, seeds, or identity files committed. The CLI identity directories (`.stellar/`, `.soroban/`) are git-ignored.
4. Keep contracts minimal — business logic (invoice content, client records, approval comments) stays in the backend.
5. Don't change event shapes without updating README §3/§4 and bumping the major version.
6. Run `./scripts/check.sh` (`cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`) before declaring done.
7. Testnet only.

## Conventions in this repo

- **Errors:** each contract has one `#[contracterror] enum Error` in `types.rs`. Codes are public API: append, never renumber. No `panic!` with strings. The only `expect` allowed is for invariants the constructor guarantees.
- **Amounts:** validate with `finance_os_common::amount::is_positive`, and use `checked_add`/`checked_sub`, mapping `None` to `Error::ArithmeticOverflow`.
- **Ordering:** `require_auth()` first, then validate and write state, then call the token contract's `transfer`. Emit events last.
- **TTL:** use `storage::bump_instance` in every entrypoint. Call `ttl::extend_persistent` on every read or write of a proposal or invoice. The strategy is documented in each `storage.rs` and in `packages/common/src/ttl.rs`.
- **Events:** `#[contractevent]` structs in `events.rs`. The topic is the snake_case struct name followed by the id field marked `#[topic]`.
- **Constructor error tests:** `env.register` panics when the constructor returns an error, so assert with `#[should_panic(expected = "Error(Contract, #N)")]`.
- **Test fixtures:** each `test.rs` has a `Setup` fixture plus `auth(...)` / `try_*` helpers that mock exactly one authorisation, including nested `token.transfer` sub-invokes. Wrong-signer tests sign with a different address and expect `Err(None)` (a host auth failure, not a contract error).
- **Event assertions:** `env.events().all()` only holds the **latest** invocation's events. Assert on them immediately after the call under test, before any view call.
- **Approvals:** approval counts are always re-filtered against the *current* policy (`count_valid_approvals`). Don't cache counts in storage.
- **Coverage:** `./scripts/coverage.sh`. Keep hand-written contract logic at 100% line coverage, and update `docs/coverage.md` and `docs/threat-model.md` when you add entrypoints.
- **Wasm builds:** use `stellar contract build` (`./scripts/build.sh`). soroban-sdk ≥ 29 refuses a plain `cargo build --target wasm32v1-none`.

## Prompt template

```
You are working in finance-os-contracts. Read README.md and AGENTS.md first.
Task: <task # and name>. Follow the function tables and security requirements exactly.
Deliver: contract code + tests for every failure path. List any deviations from the README.
```
