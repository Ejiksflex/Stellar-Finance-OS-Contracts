# Changelog

All notable changes to the contracts' public surface (functions, events,
error codes) and tooling. Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versioning: [SemVer](https://semver.org/). A change to a function signature,
event shape or error code is a **major** bump.

## [Unreleased]

### Added
- `approval-vault`: constructor, `deposit`, `propose`, `approve`, `execute`,
  `cancel`, `propose_policy_update`, `approve_policy_update`, `update_policy`,
  `cancel_policy_update`, and views `get_policy`, `get_proposal`,
  `get_policy_proposal`, `balance`.
- `approval-vault` events: `proposed`, `policy_proposed`, `approved`,
  `executed`, `cancelled`, `policy_updated`.
- `approval-vault` errors 1–13 (see README §3).
- `invoice-registry`: `register`, `pay`, `void`, `get`.
- `invoice-registry` events: `registered`, `paid`, `voided`.
- `invoice-registry` errors 1–9 (see README §4).
- Shared TTL policy (30-day bump, 1-day threshold) in `packages/common`.
- Scripts: `build`, `check`, `coverage`, `deploy`, `bindings`.
- CI: fmt/clippy/test gate, Wasm build with size limit, coverage, cargo-audit.
- Workflows: manual testnet deploy, tag-triggered release with checksums.
- Docs: threat model, coverage report, security policy, contributing guide,
  event decoding guide for the indexer, testnet runbook.
- Tests: property tests for M-of-N execution, policy validation and invoice
  payment invariants; exact auth-tree assertions; event XDR pinned to docs.
- Build: contract meta records `source_repo` and `source_rev`; deployments
  record `sourceRev` and `cliVersion`.

### Deviations from the original design
- Policy changes use a dedicated proposal flow (`propose_policy_update` →
  `approve_policy_update` → `update_policy(id)`) instead of
  `update_policy(new_approvers, new_threshold)`.
- `pay` rejects overpayment instead of accepting `paid > amount`.
