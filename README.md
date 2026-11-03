# Stellar Finance OS — Contracts

> Repo 1 of 3 · `finance-os-contracts` · Soroban (Rust) workspace
> Sibling repos: [`finance-os-backend`](../finance-os-backend) (API, indexer) · [`finance-os-web`](../finance-os-web) (frontend)

Soroban smart contracts for Stellar Business Treasury & Finance OS.

**Important: contracts are Phase 2.** The MVP works with plain Stellar payments and off-chain approvals in the backend. This repo adds on-chain guarantees once the MVP has pilot users:

1. **`approval-vault`** — treasury funds that can only leave after M-of-N approvers sign off, enforced on-chain.
2. **`invoice-registry`** — anchors invoice hashes on-chain and lets clients pay *through* the contract, emitting an event that identifies the invoice exactly.

You can scaffold this repo now and build it in parallel, but don't block the MVP on it.

### Current status

Build-order task 1 (scaffold) is done. See [§8](#build-order).

| Piece | State |
|---|---|
| Workspace, `common` crate, scripts, CI | ✅ |
| `approval-vault` | types, storage, events defined; `__constructor` + `get_policy` implemented and tested. Rest is tasks 2–4 |
| `invoice-registry` | types, storage, events defined; `get` implemented and tested. Rest is task 5 |
| Testnet deployment, bindings | not yet (tasks 6–7) |

---

## 1. Workspace layout

```
finance-os-contracts/
├── contracts/
│   ├── approval-vault/
│   │   ├── src/
│   │   │   ├── lib.rs          # contract entrypoints
│   │   │   ├── storage.rs      # storage keys + TTL helpers
│   │   │   ├── types.rs        # Proposal, Policy, errors
│   │   │   ├── events.rs
│   │   │   └── test.rs
│   │   └── Cargo.toml
│   └── invoice-registry/
│       └── (same structure)
├── packages/
│   └── common/                 # shared types/errors used by both contracts (Rust crate)
├── bindings/                   # generated TypeScript bindings (consumed by backend + web)
├── deployments/
│   ├── testnet.json            # { "approvalVault": "C...", "invoiceRegistry": "C...", "wasmHash": {...} }
│   └── mainnet.json
├── scripts/
│   ├── build.sh
│   ├── deploy.sh               # deploy + write deployments/<network>.json
│   └── bindings.sh             # regenerate TS bindings
├── Cargo.toml                  # workspace
├── AGENTS.md
└── README.md
```

---

## 2. Tech stack

| Concern | Choice |
|---|---|
| Language | Rust (stable), `wasm32` target required by current Stellar CLI docs |
| SDK | `soroban-sdk` — pin to the latest stable release at scaffold time |
| CLI | `stellar` CLI (build, deploy, invoke, generate bindings) |
| Tokens | Interact with assets (e.g. USDC) via their **Stellar Asset Contract (SAC)** using the standard token interface |
| Tests | Built-in `soroban-sdk` testutils (`Env::default()`, `mock_all_auths` only where explicitly intended) |

**Pinned at scaffold time (2026-11-03):**

| Item | Version |
|---|---|
| `soroban-sdk` | `29.0.1` (30.x is still RC) |
| Rust target | `wasm32v1-none` (see `rust-toolchain.toml`) |
| `stellar` CLI (CI) | `v28.1.0`. soroban-sdk 29 needs ≥ v25.2.0. |

> soroban-sdk ≥ 29 refuses a raw `cargo build --target wasm32v1-none`. Always build Wasm with `stellar contract build` (`./scripts/build.sh`), which post-processes the contract spec.

> ⚠️ Verify at scaffold time: the exact Rust target name, CLI commands, and SDK version change between Stellar protocol releases. Follow the current [Stellar developer docs](https://developers.stellar.org) rather than this README if they differ.

---

## 3. Contract: `approval-vault`

Holds an organisation's treasury tokens. Outgoing payments are proposals that need M-of-N approver signatures before they execute.

### Storage
- `Policy { approvers: Vec<Address>, threshold: u32 }` (instance; at most 20 approvers)
- `Proposal { id: u64, token: Address, to: Address, amount: i128, memo_ref: BytesN<32>, created_by: Address, approvals: Vec<Address>, status: Pending | Executed | Cancelled, expires_at: u64 }` (persistent)
- `PolicyProposal { id: u64, new_policy: Policy, created_by: Address, approvals: Vec<Address>, status, expires_at: u64 }` (persistent)
- `NextProposalId: u64` (instance; shared by both proposal kinds, so ids are unique across them)

`expires_at` is a ledger timestamp in seconds. A proposal is expired once `now >= expires_at`.

### Functions

| Function | Auth | Behaviour |
|---|---|---|
| `__constructor(approvers, threshold)` | — | Sets initial policy. Rejects `threshold == 0`, `threshold > approvers.len()`, duplicate approvers, more than 20 approvers |
| `deposit(from, token, amount)` | `from` | Transfers tokens into the vault. Rejects `amount <= 0` |
| `propose(proposer, token, to, amount, memo_ref, expires_at) -> u64` | `proposer` (must be approver) | Creates a Pending proposal; proposer's approval counts. Rejects `amount <= 0`, `expires_at <= now` |
| `approve(approver, id)` | `approver` (must be approver) | Adds approval; no double-approving; rejects expired/non-pending |
| `execute(id)` | anyone | If approvals from **current** approvers ≥ threshold, not expired, and the vault holds enough → marks Executed, then transfers |
| `cancel(caller, id)` | `caller` (must be approver) | Marks a Pending proposal Cancelled (allowed after expiry) |
| `propose_policy_update(proposer, new_approvers, new_threshold, expires_at) -> u64` | `proposer` (must be approver) | Creates a Pending `PolicyProposal`. New policy validated like the constructor |
| `approve_policy_update(approver, id)` | `approver` (must be approver) | Same rules as `approve` |
| `update_policy(id)` | anyone | If approvals from current approvers ≥ current threshold and not expired → replaces the policy |
| `cancel_policy_update(caller, id)` | `caller` (must be approver) | Marks a Pending policy proposal Cancelled |
| `get_proposal(id)`, `get_policy_proposal(id)`, `get_policy()`, `balance(token)` | — | Views |

**Deviation from the original sketch:** `update_policy(new_approvers, new_threshold)` is split into `propose_policy_update` / `approve_policy_update` / `update_policy(id)` / `cancel_policy_update`. This is how "its own proposal type" works in practice.

**Approvals are re-checked against the current policy.** If an approver is removed, their approvals on still-pending proposals stop counting.

### Events

Topics are `[<name>, proposal_id]`; data is a map of the remaining fields.

| Event | Data fields |
|---|---|
| `proposed` | `proposer, token, to, amount` |
| `policy_proposed` | `proposer, approvers, threshold` |
| `approved` | `approver, approvals` (running count; used for both kinds) |
| `executed` | — |
| `cancelled` | `cancelled_by` (used for both kinds) |
| `policy_updated` | `approvers, threshold` |

`memo_ref` is a hash of the backend `PaymentRequest` ID, linking on-chain proposals to off-chain records without exposing business data.

### Errors

`InvalidThreshold=1, DuplicateApprover=2, NotApprover=3, InvalidAmount=4, ProposalNotFound=5, NotPending=6, AlreadyApproved=7, Expired=8, InvalidExpiry=9, ThresholdNotMet=10, ArithmeticOverflow=11, TooManyApprovers=12, InsufficientBalance=13`

---

## 4. Contract: `invoice-registry`

Lets a business register invoices on-chain and lets clients pay them through the contract, so payment → invoice matching is exact.

### Storage
- `Invoice { id: BytesN<32>, business: Address, token: Address, amount: i128, paid: i128, status: Open | Paid | Void, content_hash: BytesN<32> }` (persistent)

`id` and `content_hash` are hashes generated by the backend. **No customer names, amounts-in-words, or personal data go on-chain** — only hashes and the payable amount.

### Functions

| Function | Auth | Behaviour |
|---|---|---|
| `register(business, id, token, amount, content_hash)` | `business` | Creates Open invoice. Rejects duplicate id, `amount <= 0`, and a `token` that doesn't implement the token interface |
| `pay(payer, id, amount)` | `payer` | Transfers `amount` of `token` from payer → `business`; updates `paid`; sets Paid when `paid == amount`. Partial payments allowed; **overpayment rejected** |
| `void(business, id)` | `business` (must be the invoice's business) | Only while Open and nothing paid yet |
| `get(id)` | — | View |

Payments go to the `business` address. Routing them to the business's `approval-vault` instead is not supported in v1, because `register` needs `business` auth and the vault can't sign it. That needs a separate `payee` field (open item).

### Events

Topics are `[<name>, id]`; data is a map of the remaining fields.

| Event | Data fields |
|---|---|
| `registered` | `business, token, amount` |
| `paid` | `payer, amount, total_paid` |
| `voided` | — |

The backend uses `paid` events as the `contract_event` match method.

### Errors

`InvoiceExists=1, InvoiceNotFound=2, InvalidAmount=3, NotOpen=4, Overpayment=5, AlreadyPaid=6, ArithmeticOverflow=7, InvalidToken=8, NotBusiness=9`

---

## 5. Security requirements

These are acceptance criteria, not suggestions:

- Every state-changing function calls `require_auth()` on the right address.
- Use checked arithmetic on all `i128` amounts; reject `amount <= 0`.
- No re-entrancy assumptions — update state before calling the token contract's `transfer`.
- Handle storage TTL: extend persistent entries (proposals, invoices) so they don't get archived while still in use. Document the TTL strategy in `storage.rs`.
- Explicit error enum (`#[contracterror]`) — no `panic!` with strings.
- Tests for every failure path: wrong signer, double approval, expired proposal, threshold not met, overpayment, void-after-payment, unknown token.
- **No mainnet deployment without an external audit.** Stellar runs an audit programme for ecosystem projects — check current eligibility at time of applying.
- Contracts are not upgradeable in v1 unless a deliberate, approver-gated upgrade path is designed and reviewed.

---

## 6. Commands

```bash
# build all contracts
stellar contract build        # or ./scripts/build.sh

# test
cargo test

# deploy to testnet (script wraps `stellar contract deploy` and writes deployments/testnet.json)
STELLAR_ACCOUNT=finance-os-deployer \
VAULT_APPROVERS='["G...","G..."]' VAULT_THRESHOLD=2 \
./scripts/deploy.sh testnet

# generate TypeScript bindings for backend/web
./scripts/bindings.sh     # wraps `stellar contract bindings typescript`
```

Create and fund a testnet identity with the Stellar CLI (`stellar keys generate ... --fund`) — verify flags against current CLI help. Never commit identity files.

`deploy.sh mainnet` refuses to run unless `FINANCE_OS_MAINNET_AUDITED=yes` is set (§5).

---

## 7. How other repos consume this one

| Output | Consumer | How |
|---|---|---|
| `deployments/<network>.json` | backend | Copied/published; read via `CONTRACT_DEPLOYMENTS_PATH` |
| `bindings/` (TS) | backend (`packages/stellar`), web (`packages/wallet`) | Published as a package or git dependency |
| Event schemas (§3, §4) | backend indexer | Documented here; changes are breaking |

Version contracts with semver tags; changing an event shape or function signature is a major version.

---

## 8. Building this with AI

Rules live in [`AGENTS.md`](AGENTS.md).

### Build order

| # | Task | Done when |
|---|---|---|
| 1 | Scaffold workspace, `common` crate, scripts, CI (fmt, clippy, test) | `stellar contract build` succeeds on empty contracts |
| 2 | `approval-vault`: types, storage, constructor, deposit | Tests pass |
| 3 | `approval-vault`: propose/approve/execute/cancel + events | All §5 failure tests pass |
| 4 | `approval-vault`: policy update via threshold | Tests pass |
| 5 | `invoice-registry`: register/pay/void + events | Tests pass incl. partial & over-payment |
| 6 | Deploy scripts + `deployments/testnet.json` | Contracts live on testnet |
| 7 | TS bindings generation | Backend can import and call `get_policy` |
| 8 | Integration: backend indexer reads contract events (done in backend repo) | Contract `paid` event marks invoice Paid |
| 9 | Audit prep: threat model doc, test coverage report | Ready to submit for audit |

### Prompt template

```
You are working in finance-os-contracts. Read README.md and AGENTS.md first.
Task: <task # and name>. Follow the function tables and security requirements exactly.
Deliver: contract code + tests for every failure path. List any deviations from the README.
```
