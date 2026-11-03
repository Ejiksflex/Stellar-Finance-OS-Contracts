extern crate std;

use soroban_sdk::{testutils::Address as _, vec, Address, Env};

use crate::{ApprovalVault, ApprovalVaultClient};

fn approvers(env: &Env, n: u32) -> soroban_sdk::Vec<Address> {
    let mut v = soroban_sdk::Vec::new(env);
    for _ in 0..n {
        v.push_back(Address::generate(env));
    }
    v
}

#[test]
fn constructor_sets_policy() {
    let env = Env::default();
    let a = approvers(&env, 3);
    let id = env.register(ApprovalVault, (a.clone(), 2u32));
    let client = ApprovalVaultClient::new(&env, &id);

    let policy = client.get_policy();
    assert_eq!(policy.approvers, a);
    assert_eq!(policy.threshold, 2);
}

// Constructor failures abort registration, so they surface as panics carrying
// the contract error code (see `types::Error`).

#[test]
#[should_panic(expected = "Error(Contract, #1)")] // InvalidThreshold
fn constructor_rejects_zero_threshold() {
    let env = Env::default();
    let a = approvers(&env, 2);
    env.register(ApprovalVault, (a, 0u32));
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")] // InvalidThreshold
fn constructor_rejects_threshold_above_approver_count() {
    let env = Env::default();
    let a = approvers(&env, 2);
    env.register(ApprovalVault, (a, 3u32));
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")] // DuplicateApprover
fn constructor_rejects_duplicate_approvers() {
    let env = Env::default();
    let a = Address::generate(&env);
    env.register(ApprovalVault, (vec![&env, a.clone(), a], 1u32));
}
