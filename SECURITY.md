# Security policy

## Status

These contracts are **pre-audit and deployed to testnet only**. Do not use
them to hold real funds. See README §5 and [`docs/threat-model.md`](docs/threat-model.md).

## Reporting a vulnerability

Please **do not open a public issue** for security problems.

Report privately through GitHub:
**Security → Report a vulnerability** on this repository
(<https://github.com/Stellar-Finance-OS/Stellar-Finance-OS-Contracts/security/advisories/new>).

Please include:

- which contract and function is affected
- a description of the impact (e.g. funds movable without M-of-N approval)
- steps or a test case to reproduce, ideally as a `#[test]` against `Env::default()`

We aim to acknowledge reports within 3 working days.

## Scope

| In scope | Out of scope |
|---|---|
| `contracts/approval-vault`, `contracts/invoice-registry`, `packages/common` | The Stellar network, Soroban host, or `soroban-sdk` itself (report upstream to Stellar) |
| Deployment scripts and workflows in this repo | Backend and web repos (report in their own repositories) |
| | Issues that require a malicious token chosen by the victim (documented in the threat model) |
