extern crate std;

use soroban_sdk::{
    testutils::{Address as _, MockAuth, MockAuthInvoke},
    token::{StellarAssetClient, TokenClient},
    vec, Address, Env, IntoVal, Val,
};

use crate::{types::Error, ApprovalVault, ApprovalVaultClient};

/// Test fixture: a vault with 3 approvers, threshold 2, and a SAC token.
///
/// Auth is mocked explicitly per call (AGENTS.md rule 2) via [`auth`] and
/// [`token_transfer_auth`], never with `mock_all_auths`.
#[allow(dead_code)]
struct Setup<'a> {
    env: Env,
    vault_id: Address,
    vault: ApprovalVaultClient<'a>,
    approvers: [Address; 3],
    token: TokenClient<'a>,
    token_admin: Address,
}

impl Setup<'_> {
    fn new() -> Self {
        let env = Env::default();
        let approvers = [
            Address::generate(&env),
            Address::generate(&env),
            Address::generate(&env),
        ];
        let vault_id = env.register(
            ApprovalVault,
            (
                vec![
                    &env,
                    approvers[0].clone(),
                    approvers[1].clone(),
                    approvers[2].clone(),
                ],
                2u32,
            ),
        );
        let vault = ApprovalVaultClient::new(&env, &vault_id);
        let token_admin = Address::generate(&env);
        let sac = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token = TokenClient::new(&env, &sac.address());
        Setup {
            env,
            vault_id,
            vault,
            approvers,
            token,
            token_admin,
        }
    }

    /// Mint `amount` of the test token to `to`, authorising only the admin.
    fn mint(&self, to: &Address, amount: i128) {
        let args: soroban_sdk::Vec<Val> = (to.clone(), amount).into_val(&self.env);
        self.env.mock_auths(&[MockAuth {
            address: &self.token_admin,
            invoke: &MockAuthInvoke {
                contract: &self.token.address,
                fn_name: "mint",
                args,
                sub_invokes: &[],
            },
        }]);
        StellarAssetClient::new(&self.env, &self.token.address).mint(to, &amount);
    }

    /// Mint to a fresh account and deposit it into the vault.
    fn fund_vault(&self, amount: i128) {
        let from = Address::generate(&self.env);
        self.mint(&from, amount);
        let args = (from.clone(), self.token.address.clone(), amount).into_val(&self.env);
        let sub = [token_transfer_auth(
            &self.env,
            &self.token.address,
            &from,
            &self.vault_id,
            amount,
        )];
        auth(&self.env, &from, &self.vault_id, "deposit", args, &sub);
        self.vault.deposit(&from, &self.token.address, &amount);
    }
}

/// Mock exactly one authorisation: `signer` authorises `contract.fn_name(args)`.
fn auth(
    env: &Env,
    signer: &Address,
    contract: &Address,
    fn_name: &str,
    args: soroban_sdk::Vec<Val>,
    sub_invokes: &[MockAuthInvoke],
) {
    env.mock_auths(&[MockAuth {
        address: signer,
        invoke: &MockAuthInvoke {
            contract,
            fn_name,
            args,
            sub_invokes,
        },
    }]);
}

/// The nested `token.transfer(from, to, amount)` authorisation.
fn token_transfer_auth<'a>(
    env: &Env,
    token: &'a Address,
    from: &Address,
    to: &Address,
    amount: i128,
) -> MockAuthInvoke<'a> {
    MockAuthInvoke {
        contract: token,
        fn_name: "transfer",
        args: (from.clone(), to.clone(), amount).into_val(env),
        sub_invokes: &[],
    }
}

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

#[test]
#[should_panic(expected = "Error(Contract, #12)")] // TooManyApprovers
fn constructor_rejects_too_many_approvers() {
    let env = Env::default();
    let a = approvers(&env, crate::types::MAX_APPROVERS + 1);
    env.register(ApprovalVault, (a, 1u32));
}

#[test]
fn constructor_accepts_max_approvers() {
    let env = Env::default();
    let a = approvers(&env, crate::types::MAX_APPROVERS);
    let id = env.register(ApprovalVault, (a, crate::types::MAX_APPROVERS));
    let client = ApprovalVaultClient::new(&env, &id);
    assert_eq!(client.get_policy().threshold, crate::types::MAX_APPROVERS);
}

// ---------------------------------------------------------------- deposit

#[test]
fn deposit_moves_tokens_into_vault() {
    let s = Setup::new();
    s.fund_vault(1_000);
    assert_eq!(s.token.balance(&s.vault_id), 1_000);
}

#[test]
fn deposit_rejects_zero_amount() {
    let s = Setup::new();
    let from = Address::generate(&s.env);
    let args = (from.clone(), s.token.address.clone(), 0i128).into_val(&s.env);
    auth(&s.env, &from, &s.vault_id, "deposit", args, &[]);
    let res = s.vault.try_deposit(&from, &s.token.address, &0);
    assert_eq!(res, Err(Ok(Error::InvalidAmount)));
}

#[test]
fn deposit_rejects_negative_amount() {
    let s = Setup::new();
    let from = Address::generate(&s.env);
    let args = (from.clone(), s.token.address.clone(), -5i128).into_val(&s.env);
    auth(&s.env, &from, &s.vault_id, "deposit", args, &[]);
    let res = s.vault.try_deposit(&from, &s.token.address, &-5);
    assert_eq!(res, Err(Ok(Error::InvalidAmount)));
}

#[test]
fn deposit_rejects_wrong_signer() {
    let s = Setup::new();
    let from = Address::generate(&s.env);
    let attacker = Address::generate(&s.env);
    s.mint(&from, 100);
    let args = (from.clone(), s.token.address.clone(), 100i128).into_val(&s.env);
    auth(&s.env, &attacker, &s.vault_id, "deposit", args, &[]);
    assert!(s.vault.try_deposit(&from, &s.token.address, &100).is_err());
    assert_eq!(s.token.balance(&from), 100);
    assert_eq!(s.token.balance(&s.vault_id), 0);
}

#[test]
fn deposit_rejects_unknown_token() {
    let s = Setup::new();
    let from = Address::generate(&s.env);
    let not_a_token = Address::generate(&s.env);
    let args = (from.clone(), not_a_token.clone(), 100i128).into_val(&s.env);
    auth(&s.env, &from, &s.vault_id, "deposit", args, &[]);
    assert!(s.vault.try_deposit(&from, &not_a_token, &100).is_err());
}

#[test]
fn deposit_fails_with_insufficient_funds() {
    let s = Setup::new();
    let from = Address::generate(&s.env);
    s.mint(&from, 50);
    let args = (from.clone(), s.token.address.clone(), 100i128).into_val(&s.env);
    let sub = [token_transfer_auth(
        &s.env,
        &s.token.address,
        &from,
        &s.vault_id,
        100,
    )];
    auth(&s.env, &from, &s.vault_id, "deposit", args, &sub);
    assert!(s.vault.try_deposit(&from, &s.token.address, &100).is_err());
    assert_eq!(s.token.balance(&from), 50);
}
