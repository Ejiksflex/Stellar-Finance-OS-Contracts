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
use soroban_sdk::{contract, contractimpl, token, vec, Address, BytesN, Env, Vec};

use crate::events::{Approved, Proposed};
use crate::types::{Error, Policy, Proposal, ProposalStatus, MAX_APPROVERS};

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

    /// Create a Pending payment proposal. The proposer must be an approver,
    /// and their approval counts towards the threshold.
    ///
    /// `memo_ref` is a hash of the backend `PaymentRequest` id.
    /// `expires_at` is a ledger timestamp (seconds) that must be in the future.
    pub fn propose(
        env: Env,
        proposer: Address,
        token: Address,
        to: Address,
        amount: i128,
        memo_ref: BytesN<32>,
        expires_at: u64,
    ) -> Result<u64, Error> {
        proposer.require_auth();
        storage::bump_instance(&env);
        require_approver(&storage::get_policy(&env), &proposer)?;
        if !amount::is_positive(amount) {
            return Err(Error::InvalidAmount);
        }
        if expires_at <= env.ledger().timestamp() {
            return Err(Error::InvalidExpiry);
        }

        let id = storage::next_proposal_id(&env)?;
        storage::set_proposal(
            &env,
            &Proposal {
                id,
                token: token.clone(),
                to: to.clone(),
                amount,
                memo_ref,
                created_by: proposer.clone(),
                approvals: vec![&env, proposer.clone()],
                status: ProposalStatus::Pending,
                expires_at,
            },
        );
        Proposed {
            proposal_id: id,
            proposer,
            token,
            to,
            amount,
        }
        .publish(&env);
        Ok(id)
    }

    /// Add `approver`'s approval to a Pending, unexpired proposal.
    pub fn approve(env: Env, approver: Address, id: u64) -> Result<(), Error> {
        approver.require_auth();
        storage::bump_instance(&env);
        require_approver(&storage::get_policy(&env), &approver)?;

        let mut proposal = storage::get_proposal(&env, id)?;
        require_open(&env, &proposal)?;
        if proposal.approvals.contains(&approver) {
            return Err(Error::AlreadyApproved);
        }
        proposal.approvals.push_back(approver.clone());
        storage::set_proposal(&env, &proposal);

        Approved {
            proposal_id: id,
            approver,
            approvals: proposal.approvals.len(),
        }
        .publish(&env);
        Ok(())
    }

    // TODO(task 3): execute, cancel
    // TODO(task 4): policy update via threshold-approved proposal

    /// The vault's balance of `token`.
    pub fn balance(env: Env, token: Address) -> i128 {
        storage::bump_instance(&env);
        token::TokenClient::new(&env, &token).balance(&env.current_contract_address())
    }

    pub fn get_proposal(env: Env, id: u64) -> Result<Proposal, Error> {
        storage::bump_instance(&env);
        storage::get_proposal(&env, id)
    }

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

fn require_approver(policy: &Policy, who: &Address) -> Result<(), Error> {
    if policy.approvers.contains(who) {
        Ok(())
    } else {
        Err(Error::NotApprover)
    }
}

/// A proposal can be acted on only while Pending and before `expires_at`.
fn require_open(env: &Env, proposal: &Proposal) -> Result<(), Error> {
    if proposal.status != ProposalStatus::Pending {
        return Err(Error::NotPending);
    }
    if env.ledger().timestamp() >= proposal.expires_at {
        return Err(Error::Expired);
    }
    Ok(())
}
