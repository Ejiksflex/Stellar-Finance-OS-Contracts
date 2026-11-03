//! Storage keys and accessors for the approval vault.
//!
//! Layout:
//! * `Policy`, `NextProposalId` → **instance** storage. These are small, read on
//!   almost every call, and live exactly as long as the contract.
//! * `Proposal(id)` / `PolicyProposal(id)` → **persistent** storage, one
//!   entry per proposal. Both kinds share the `NextProposalId` sequence.
//!
//! TTL strategy (see `finance_os_common::ttl`): every entrypoint bumps the
//! instance TTL, and every proposal read/write bumps that proposal's TTL.
//! Pending proposals stay live while anyone interacts with them. Proposals
//! nobody touches may be archived after about 30 days and can be restored.

use finance_os_common::ttl;
use soroban_sdk::{contracttype, Env};

use crate::types::{Error, Policy, PolicyProposal, Proposal};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Policy,
    NextProposalId,
    Proposal(u64),
    PolicyProposal(u64),
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

/// Allocate the next proposal id. Ids start at 0 and are never reused.
pub fn next_proposal_id(env: &Env) -> Result<u64, Error> {
    let id: u64 = env
        .storage()
        .instance()
        .get(&DataKey::NextProposalId)
        .unwrap_or(0);
    let next = id.checked_add(1).ok_or(Error::ArithmeticOverflow)?;
    env.storage()
        .instance()
        .set(&DataKey::NextProposalId, &next);
    Ok(id)
}

pub fn get_proposal(env: &Env, id: u64) -> Result<Proposal, Error> {
    let key = DataKey::Proposal(id);
    let proposal = env
        .storage()
        .persistent()
        .get(&key)
        .ok_or(Error::ProposalNotFound)?;
    ttl::extend_persistent(env, &key);
    Ok(proposal)
}

pub fn set_proposal(env: &Env, proposal: &Proposal) {
    let key = DataKey::Proposal(proposal.id);
    env.storage().persistent().set(&key, proposal);
    ttl::extend_persistent(env, &key);
}

pub fn get_policy_proposal(env: &Env, id: u64) -> Result<PolicyProposal, Error> {
    let key = DataKey::PolicyProposal(id);
    let proposal = env
        .storage()
        .persistent()
        .get(&key)
        .ok_or(Error::ProposalNotFound)?;
    ttl::extend_persistent(env, &key);
    Ok(proposal)
}

pub fn set_policy_proposal(env: &Env, proposal: &PolicyProposal) {
    let key = DataKey::PolicyProposal(proposal.id);
    env.storage().persistent().set(&key, proposal);
    ttl::extend_persistent(env, &key);
}
