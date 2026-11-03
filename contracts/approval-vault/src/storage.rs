//! Storage keys and accessors for the approval vault.
//!
//! Layout:
//! * `Policy`, `NextProposalId` → **instance** storage. These are small, read on
//!   almost every call, and live exactly as long as the contract.
//! * `Proposal(id)` → **persistent** storage, one entry per proposal.
//!
//! TTL strategy (see `finance_os_common::ttl`): every entrypoint bumps the
//! instance TTL, and every proposal read/write bumps that proposal's TTL.
//! Pending proposals stay live while anyone interacts with them. Proposals
//! nobody touches may be archived after about 30 days and can be restored.

use finance_os_common::ttl;
use soroban_sdk::{contracttype, Env};

use crate::types::Policy;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Policy,
    NextProposalId,
    Proposal(u64),
}

pub fn bump_instance(env: &Env) {
    ttl::extend_instance(env);
}

pub fn get_policy(env: &Env) -> Policy {
    // Always set by the constructor, so absence is an invariant violation.
    env.storage()
        .instance()
        .get(&DataKey::Policy)
        .expect("policy is set in constructor")
}

pub fn set_policy(env: &Env, policy: &Policy) {
    env.storage().instance().set(&DataKey::Policy, policy);
}
