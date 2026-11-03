//! Approval vault: treasury funds leave only after M-of-N approvers sign off.
//!
//! Spec: README §3. Build order: tasks 2–4 in AGENTS.md.
#![no_std]

pub mod events;
mod storage;
pub mod types;

#[cfg(test)]
mod test;

use finance_os_common::amount;
use soroban_sdk::{contract, contractimpl, token, Address, Env, Vec};

use crate::types::{Error, Policy, MAX_APPROVERS};

#[contract]
pub struct ApprovalVault;

#[contractimpl]
impl ApprovalVault {
    /// Sets the initial policy. Rejects `threshold == 0`,
    /// `threshold > approvers.len()`, duplicate approvers, and more than
    /// [`MAX_APPROVERS`] approvers.
    pub fn __constructor(env: Env, approvers: Vec<Address>, threshold: u32) -> Result<(), Error> {
        let policy = validate_policy(approvers, threshold)?;
        storage::set_policy(&env, &policy);
        storage::bump_instance(&env);
        Ok(())
    }

    /// Transfer `amount` of `token` from `from` into the vault.
    ///
    /// Anyone may deposit. `token` must be a contract implementing the standard
    /// token interface (e.g. a Stellar Asset Contract).
    pub fn deposit(env: Env, from: Address, token: Address, amount: i128) -> Result<(), Error> {
        from.require_auth();
        if !amount::is_positive(amount) {
            return Err(Error::InvalidAmount);
        }
        storage::bump_instance(&env);
        token::TokenClient::new(&env, &token).transfer(
            &from,
            env.current_contract_address(),
            &amount,
        );
        Ok(())
    }

    // TODO(task 3): propose, approve, execute, cancel, get_proposal, balance
    // TODO(task 4): policy update via threshold-approved proposal

    pub fn get_policy(env: Env) -> Policy {
        storage::bump_instance(&env);
        storage::get_policy(&env)
    }
}

fn validate_policy(approvers: Vec<Address>, threshold: u32) -> Result<Policy, Error> {
    if approvers.len() > MAX_APPROVERS {
        return Err(Error::TooManyApprovers);
    }
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
