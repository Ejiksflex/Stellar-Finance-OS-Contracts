# Verifying a deployment against source

Goal: prove that the contract running on-chain was built from a specific commit
of this repo. This is what an auditor or integrator should check before trusting
a contract id.

## What makes builds reproducible

| Input | Pinned by |
|---|---|
| Source | the commit in `deployments/<network>.json` → `sourceRev` (also embedded in the Wasm as `source_rev` meta) |
| Rust compiler | `rust-toolchain.toml` (1.99.1) |
| Dependencies | `Cargo.lock` (committed) |
| Stellar CLI | 28.1.0 in CI (`deployments/<network>.json` → `cliVersion`) |
| Build OS | `ubuntu-24.04` runners |

A Soroban Wasm hash is the SHA-256 of the Wasm bytes, so a file hash
can be compared directly with the hash on-chain.

## Quick check

```bash
./scripts/verify-deployment.sh testnet
```

This fetches the on-chain Wasm for both contracts, then checks its hash against
`deployments/testnet.json` and any local build in `target/`.

## Full check from a clean checkout

```bash
REV=$(jq -r .sourceRev deployments/testnet.json)
git checkout "$REV"
./scripts/build.sh                       # same toolchain, lockfile, CLI version
./scripts/verify-deployment.sh testnet   # local build must match on-chain
```

## Checking a release

Each `vX.Y.Z` GitHub release carries both Wasm files and `SHA256SUMS`:

```bash
sha256sum -c SHA256SUMS
stellar contract info meta --wasm approval_vault.wasm   # shows source_repo / source_rev
```

The hashes in `SHA256SUMS` should equal the `wasmHash` values recorded in
`deployments/<network>.json` for a deployment of that release.

## If hashes differ

- Check the CLI version (`stellar --version`). Different optimizer versions
  produce different bytes.
- Check you are on `sourceRev` with a clean tree (`git status`).
- Check the toolchain (`rustc --version` inside the repo should print 1.99.1).
