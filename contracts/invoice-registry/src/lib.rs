//! Invoice registry: anchors invoice hashes on-chain and lets clients pay
//! through the contract so payment → invoice matching is exact.
//!
//! Spec: README §4. Build order: task 5 in AGENTS.md.
#![no_std]

pub mod events;
mod storage;
pub mod types;

#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, BytesN, Env};

use crate::types::{Error, Invoice};

#[contract]
pub struct InvoiceRegistry;

#[contractimpl]
impl InvoiceRegistry {
    // TODO(task 5): register, pay, void

    pub fn get(env: Env, id: BytesN<32>) -> Result<Invoice, Error> {
        storage::bump_instance(&env);
        storage::get_invoice(&env, &id).ok_or(Error::InvoiceNotFound)
    }
}
