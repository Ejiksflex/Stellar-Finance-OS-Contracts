//! Shared helpers used by every Finance OS contract.
//!
//! Keep this crate small: only things that are genuinely identical across
//! contracts belong here (TTL policy, amount validation). Contract-specific
//! types and error enums live in each contract's `types.rs`.
#![no_std]

pub mod amount;
pub mod ttl;

#[cfg(test)]
mod test;
