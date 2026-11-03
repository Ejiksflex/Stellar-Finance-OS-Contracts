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

use finance_os_common::amount;
use soroban_sdk::{contract, contractimpl, token, Address, BytesN, Env};

use crate::events::Registered;
use crate::types::{Error, Invoice, InvoiceStatus};

#[contract]
pub struct InvoiceRegistry;

#[contractimpl]
impl InvoiceRegistry {
    /// Register an Open invoice payable to `business` in `token`.
    ///
    /// `id` and `content_hash` are backend-generated hashes. `token` must be a
    /// contract implementing the standard token interface. This is checked
    /// here so a bad token fails at registration, not when a client pays.
    pub fn register(
        env: Env,
        business: Address,
        id: BytesN<32>,
        token: Address,
        amount: i128,
        content_hash: BytesN<32>,
    ) -> Result<(), Error> {
        business.require_auth();
        storage::bump_instance(&env);
        if !amount::is_positive(amount) {
            return Err(Error::InvalidAmount);
        }
        if storage::has_invoice(&env, &id) {
            return Err(Error::InvoiceExists);
        }
        require_token(&env, &token)?;

        storage::set_invoice(
            &env,
            &Invoice {
                id: id.clone(),
                business: business.clone(),
                token: token.clone(),
                amount,
                paid: 0,
                status: InvoiceStatus::Open,
                content_hash,
            },
        );
        Registered {
            id,
            business,
            token,
            amount,
        }
        .publish(&env);
        Ok(())
    }

    // TODO(task 5): pay, void

    pub fn get(env: Env, id: BytesN<32>) -> Result<Invoice, Error> {
        storage::bump_instance(&env);
        storage::get_invoice(&env, &id).ok_or(Error::InvoiceNotFound)
    }
}

/// `token` must answer the standard token interface's `decimals()`.
fn require_token(env: &Env, token: &Address) -> Result<(), Error> {
    match token::TokenClient::new(env, token).try_decimals() {
        Ok(Ok(_)) => Ok(()),
        _ => Err(Error::InvalidToken),
    }
}
