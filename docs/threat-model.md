# Threat model — finance-os-contracts v0.1

Scope: `contracts/approval-vault`, `contracts/invoice-registry`, `packages/common`.
Status: **pre-audit, testnet only.** Not upgradeable (no `update_current_contract_wasm` path).

## Assets

| Asset | Where | Worst case |
|---|---|---|
| Treasury tokens | vault's SAC balances | Drained without M-of-N approval |
| Approver policy | vault instance storage | Captured by a minority, or bricked (no reachable threshold) |
| Invoice payment state | registry persistent storage | Payment attributed to the wrong invoice; invoice marked Paid without funds moving |
| Business data | off-chain only | Leaked on-chain |

## Actors

- **Approver**: a member of the current `Policy.approvers` list. Trusted only collectively, up to `threshold`.
- **Business**: registers and voids its own invoices.
- **Payer**: any account paying an invoice.
- **Anyone**: can call `execute`, `update_policy` and the views.
- **Token contract**: the address supplied as `token`. Trusted only as much as the party who chose it.
- **Deployer**: has no privileges after deployment (no admin key).

## Threats and mitigations

### approval-vault

| # | Threat | Mitigation | Test(s) |
|---|---|---|---|
| V1 | A non-approver proposes, approves or cancels | `require_auth` on the acting address, plus a membership check | `*_rejects_non_approver`, `*_rejects_wrong_signer` |
| V2 | One approver counts twice | `approvals` holds unique addresses; `AlreadyApproved` | `approve_rejects_double_approval`, `approve_rejects_proposer_approving_again` |
| V3 | Executing below threshold | `count_valid_approvals >= threshold`, checked in `execute`/`update_policy` | `execute_rejects_threshold_not_met`, `update_policy_rejects_threshold_not_met`, property test `executes_iff_threshold_met` |
| V4 | Replay / double execution | Status is set to Executed before the transfer; `NotPending` | `execute_rejects_double_execution`, `update_policy_rejects_double_apply` |
| V5 | Stale approvals after the approver set changes | Approvals are re-filtered against the *current* policy at execution time and pruned on every approve, so the list stays ≤ `MAX_APPROVERS` | `stale_approvals_do_not_count_after_policy_change`, `competing_policy_proposal_needs_approvals_from_new_set`, `approve_prunes_approvals_from_removed_approvers` |
| V6 | Executing an old proposal long after intent | `expires_at` must be in the future at creation; approve/execute reject once `now >= expires_at` | `*_rejects_expired`, `propose_rejects_expiry_*` |
| V7 | Policy bricked (threshold > approvers, empty set) | `validate_policy` on construction and on every policy proposal | `constructor_rejects_*`, `propose_policy_update_rejects_*`, property test `policy_validation_matches_spec` |
| V8 | Unbounded approver set exhausts resource limits | `MAX_APPROVERS = 20` | `constructor_rejects_too_many_approvers` |
| V9 | Re-entrancy through a malicious token | State is written before the external `transfer`; Soroban also forbids re-entering the same contract | Code review (ordering in `execute`) |
| V10 | Malicious `token` on a proposal | Approvers approve the full proposal, including `token`. Only vault-held balances of that token can move | `execute_rejects_unknown_token` |
| V11 | Proposal archived (TTL) while still needed | Every read/write bumps it to 30 days; archived entries can be restored | `proposal_ttl_is_extended_on_*` |
| V12 | Id overflow | `checked_add` on `NextProposalId` | Code review |

### invoice-registry

| # | Threat | Mitigation | Test(s) |
|---|---|---|---|
| R1 | Registering an invoice for someone else's business | `business.require_auth()` | `register_rejects_wrong_signer` |
| R2 | Overwriting an existing invoice | `InvoiceExists` on a duplicate id | `register_rejects_duplicate_id*` |
| R3 | Invoice marked Paid without funds moving | State and transfer happen in one transaction, and a failed transfer reverts everything | `pay_fails_atomically_with_insufficient_funds` |
| R4 | Overpayment locks extra funds / skews accounting | `Overpayment` once `paid + amount > amount` | `pay_rejects_overpayment_*`, property test `paid_never_exceeds_amount` |
| R5 | Paying from someone else's account | `payer.require_auth()`; the transfer is from `payer` | `pay_rejects_wrong_signer` |
| R6 | Voiding a partly paid invoice (hides received funds) | `AlreadyPaid` when `paid != 0` | `void_rejects_after_partial_payment` |
| R7 | Another business voids my invoice | `NotBusiness` | `void_rejects_other_business` |
| R8 | Fake token registered so the "paid" events are worthless | `register` checks `decimals()`, but a malicious token contract still passes. **The indexer must only trust `paid` events for allow-listed token addresses** | `register_rejects_unknown_token`, `register_rejects_non_token_contract` |
| R9 | i128 overflow on `paid` | `checked_add` → `ArithmeticOverflow` | `pay_rejects_overflowing_total` |
| R10 | PII on-chain | Only hashes and amounts are stored (`id`, `content_hash`) | Design / backend responsibility |

## Residual risks and open items

1. **Token trust (R8, V10).** The contracts accept any token address. Downstream systems must allow-list token contract IDs, e.g. testnet/mainnet USDC SAC.
2. **Front-running invoice ids.** Anyone who learns an `id` hash before registration can register it first under their own business, blocking the real one. Mitigation: the backend should derive `id` from a secret salt, or include the business address in the hash preimage.
3. **Vault as payee.** Invoices pay the `business` address directly. Routing payments to an approval vault would need a separate `payee` field.
4. **Approver key loss.** If enough approvers lose keys that `threshold` can't be reached, funds are stuck. This is operational: keep `threshold < approvers.len()` to leave headroom.
5. **No upgrade path.** This is intentional for v1 (README §5). Bugs require redeploying and migrating funds through a normal proposal.
6. **TTL archival.** Entries untouched for about 30 days can be archived. They are restorable with `stellar contract restore`; the backend should restore before acting on long-idle proposals or invoices.
