# Contributing

Thanks for helping. These contracts guard real money once they reach mainnet,
so the bar for changes is deliberately high.

## Before you start

- Read the [README](README.md), especially §3/§4 (the contract spec) and §5 (security requirements).
- If you're using an AI agent, point it at [`AGENTS.md`](AGENTS.md) too.

## Making a change

1. Branch from `main`.
2. Keep each commit small and focused on one thing.
3. For every new or changed entrypoint, add tests for the happy path **and** every failure path.
   Mock auth explicitly; `mock_all_auths()` is only allowed in a test whose name says so.
4. Run the gate before pushing:

   ```bash
   ./scripts/check.sh      # fmt --check, clippy -D warnings, test
   ```

5. Update the README tables, `docs/threat-model.md` and `docs/coverage.md` if behaviour changed.
6. Open a PR and fill in the template.

## Breaking changes

The backend indexer and the generated TypeScript bindings depend on the
contracts' public surface. Any of these is a **major version bump** and must
be called out in the PR and `CHANGELOG.md`:

- changing a function name, parameter, or return type
- changing an event's name, topics or data fields
- renumbering or removing an error code (append-only)

## Things we won't merge

- Secret keys, seeds or `.stellar/` identity files
- Mainnet deployment changes without a completed external audit
- An upgrade (`update_current_contract_wasm`) path that isn't approver-gated and reviewed
