//! Data types and errors for the approval vault. Shapes follow README §3.

use soroban_sdk::{contracterror, contracttype, Address, BytesN, Vec};

/// Who may approve outgoing payments and how many approvals are required.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Policy {
    pub approvers: Vec<Address>,
    pub threshold: u32,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProposalStatus {
    Pending,
    Executed,
    Cancelled,
}

/// An outgoing payment awaiting M-of-N approval.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Proposal {
    pub id: u64,
    pub token: Address,
    pub to: Address,
    pub amount: i128,
    /// Hash of the backend `PaymentRequest` id. No business data goes on-chain.
    pub memo_ref: BytesN<32>,
    pub created_by: Address,
    pub approvals: Vec<Address>,
    pub status: ProposalStatus,
    /// Ledger timestamp (seconds) after which the proposal can no longer be approved/executed.
    pub expires_at: u64,
}

/// Error codes are part of the public interface: never renumber, only append.
#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    InvalidThreshold = 1,
    DuplicateApprover = 2,
    NotApprover = 3,
    InvalidAmount = 4,
    ProposalNotFound = 5,
    NotPending = 6,
    AlreadyApproved = 7,
    Expired = 8,
    InvalidExpiry = 9,
    ThresholdNotMet = 10,
    ArithmeticOverflow = 11,
}
