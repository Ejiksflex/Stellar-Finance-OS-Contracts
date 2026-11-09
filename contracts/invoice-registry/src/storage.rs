//! Storage keys and accessors for the invoice registry.
//!
//! Layout: `Invoice(business, id)` → **persistent** storage, one entry per
//! invoice. The key is composite so an `id` is only meaningful together with
//! the business that registered it: another address registering the same `id`
//! creates a different entry and can never receive payments meant for
//! `business` (README §5, threat model R2).
//!
//! TTL strategy (see `finance_os_common::ttl`): every entrypoint bumps the
//! instance TTL. Every invoice read/write bumps that invoice's TTL, so an
//! invoice someone is still paying or querying stays live. Invoices nobody
//! touches may be archived after about 30 days. They remain restorable, and
//! the backend keeps the canonical record.

use finance_os_common::ttl;
use soroban_sdk::{contracttype, Address, BytesN, Env};

use crate::types::Invoice;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Invoice(Address, BytesN<32>),
}

pub fn bump_instance(env: &Env) {
    ttl::extend_instance(env);
}

pub fn get_invoice(env: &Env, business: &Address, id: &BytesN<32>) -> Option<Invoice> {
    let key = DataKey::Invoice(business.clone(), id.clone());
    let invoice = env.storage().persistent().get(&key);
    if invoice.is_some() {
        ttl::extend_persistent(env, &key);
    }
    invoice
}

pub fn has_invoice(env: &Env, business: &Address, id: &BytesN<32>) -> bool {
    env.storage()
        .persistent()
        .has(&DataKey::Invoice(business.clone(), id.clone()))
}

pub fn set_invoice(env: &Env, invoice: &Invoice) {
    let key = DataKey::Invoice(invoice.business.clone(), invoice.id.clone());
    env.storage().persistent().set(&key, invoice);
    ttl::extend_persistent(env, &key);
}
