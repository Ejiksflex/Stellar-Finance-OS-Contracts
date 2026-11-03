//! Storage TTL policy shared by all contracts.
//!
//! Soroban archives entries whose TTL runs out. Strategy:
//!
//! * **Instance storage** (config such as the vault `Policy`) is bumped to
//!   [`INSTANCE_BUMP_AMOUNT`] whenever any entrypoint runs and the remaining TTL
//!   is below [`INSTANCE_LIFETIME_THRESHOLD`].
//! * **Persistent storage** (proposals, invoices) is bumped to
//!   [`PERSISTENT_BUMP_AMOUNT`] every time the entry is read or written, if the
//!   remaining TTL is below [`PERSISTENT_LIFETIME_THRESHOLD`]. Anything that is
//!   still being used therefore never gets archived. Entries nobody touches for
//!   about 30 days may be archived. They can be restored with `stellar contract restore`.
//!
//! Values stay below the network's max entry TTL so `extend_ttl` never fails.

use soroban_sdk::{Env, IntoVal, Val};

/// ~5s ledgers → 17_280 ledgers per day.
pub const DAY_IN_LEDGERS: u32 = 17_280;

pub const INSTANCE_BUMP_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const INSTANCE_LIFETIME_THRESHOLD: u32 = INSTANCE_BUMP_AMOUNT - DAY_IN_LEDGERS;

pub const PERSISTENT_BUMP_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const PERSISTENT_LIFETIME_THRESHOLD: u32 = PERSISTENT_BUMP_AMOUNT - DAY_IN_LEDGERS;

// Thresholds must sit below bump amounts or every call would re-bump.
const _: () = assert!(INSTANCE_LIFETIME_THRESHOLD < INSTANCE_BUMP_AMOUNT);
const _: () = assert!(PERSISTENT_LIFETIME_THRESHOLD < PERSISTENT_BUMP_AMOUNT);

/// Extend the contract instance (and its code) TTL.
pub fn extend_instance(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
}

/// Extend a persistent entry's TTL. Call after every read/write of a live entry.
pub fn extend_persistent<K>(env: &Env, key: &K)
where
    K: IntoVal<Env, Val>,
{
    env.storage().persistent().extend_ttl(
        key,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}
