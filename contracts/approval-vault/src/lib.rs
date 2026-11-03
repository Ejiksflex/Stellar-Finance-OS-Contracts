//! Approval vault: treasury funds leave only after M-of-N approvers sign off.
//!
//! Spec: README §3. Build order: tasks 2–4 in AGENTS.md.
#![no_std]

pub mod events;
mod storage;
pub mod types;

#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, Address, Env, Vec};

use crate::types::{Error, Policy};

#[contract]
pub struct ApprovalVault;

#[contractimpl]
impl ApprovalVault {
    /// Sets the initial policy. Rejects `threshold == 0`,
    /// `threshold > approvers.len()`, and duplicate approvers.
    pub fn __constructor(env: Env, approvers: Vec<Address>, threshold: u32) -> Result<(), Error> {
        let policy = validate_policy(approvers, threshold)?;
        storage::set_policy(&env, &policy);
        storage::bump_instance(&env);
        Ok(())
    }

    // TODO(task 2): deposit
    // TODO(task 3): propose, approve, execute, cancel, get_proposal, balance
    // TODO(task 4): policy update via threshold-approved proposal

    pub fn get_policy(env: Env) -> Policy {
        storage::bump_instance(&env);
        storage::get_policy(&env)
    }
}

fn validate_policy(approvers: Vec<Address>, threshold: u32) -> Result<Policy, Error> {
    if threshold == 0 || threshold > approvers.len() {
        return Err(Error::InvalidThreshold);
    }
    for (i, a) in approvers.iter().enumerate() {
        if approvers.iter().skip(i + 1).any(|b| b == a) {
            return Err(Error::DuplicateApprover);
        }
    }
    Ok(Policy {
        approvers,
        threshold,
    })
}
