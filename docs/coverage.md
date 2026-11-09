# Test coverage report

Generated with `./scripts/coverage.sh` (cargo-llvm-cov 0.9.1, soroban-sdk 29.0.1)
on 2026-11-04 with 154 tests (including 4 property tests). Test and
property-test modules are excluded from the figures.

| File | Lines | Regions | Functions |
|---|---|---|---|
| contracts/approval-vault/src/lib.rs | 100.00% | 99.56% | 100.00% |
| contracts/approval-vault/src/storage.rs | 98.18% | 97.75% | 88.89% |
| contracts/approval-vault/src/types.rs | 0.00% | 0.00% | 0.00% |
| contracts/invoice-registry/src/lib.rs | 100.00% | 100.00% | 100.00% |
| contracts/invoice-registry/src/storage.rs | 95.45% | 97.14% | 80.00% |
| contracts/invoice-registry/src/types.rs | 0.00% | 0.00% | 0.00% |
| packages/common/src/amount.rs | 100.00% | 100.00% | 100.00% |
| packages/common/src/ttl.rs | 100.00% | 100.00% | 100.00% |
| **Total** | **98.31%** | **98.48%** | **84.31%** |

## Uncovered lines

Every uncovered line is a `#[contracttype]` / `#[contracterror]` attribute. The
code there is macro-generated (XDR conversion and spec helpers), not
hand-written logic:

- `approval-vault/src/storage.rs:19`: `#[contracttype]` on `DataKey`
- `approval-vault/src/types.rs:10, 17, 26, 48`: attributes on `Policy`, `ProposalStatus`, `Proposal`, `Error`
- `invoice-registry/src/storage.rs:16`: `#[contracttype]` on `DataKey`
- `invoice-registry/src/types.rs:8, 16`: attributes on `InvoiceStatus`, `Invoice`

All hand-written contract logic (`lib.rs` in both contracts) has 100% line
coverage, and every failure path in README §5 has a dedicated test. See
[`threat-model.md`](threat-model.md) for the threat → test mapping.
