extern crate std;

use soroban_sdk::{
    testutils::{Address as _, Events as _, Ledger as _, MockAuth, MockAuthInvoke},
    token::{StellarAssetClient, TokenClient},
    vec, Address, BytesN, Env, Event as _, IntoVal, Val,
};

use crate::{
    events,
    types::{Error, ProposalStatus},
    ApprovalVault, ApprovalVaultClient,
};

/// Ledger timestamp every fixture starts at.
const NOW: u64 = 1_000_000;
/// Default proposal lifetime used by the fixture.
const TTL: u64 = 3_600;

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
        env.ledger().set_timestamp(NOW);
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

impl Setup<'_> {
    fn memo(&self, n: u8) -> BytesN<32> {
        BytesN::from_array(&self.env, &[n; 32])
    }

    /// `proposer` proposes paying `amount` to `to`, expiring at `NOW + TTL`.
    fn propose(&self, proposer: &Address, to: &Address, amount: i128) -> u64 {
        self.try_propose(proposer, proposer, to, amount, NOW + TTL)
            .expect("propose ok")
    }

    /// Propose with full control: `signer` is who actually authorises.
    fn try_propose(
        &self,
        signer: &Address,
        proposer: &Address,
        to: &Address,
        amount: i128,
        expires_at: u64,
    ) -> Result<u64, Option<Error>> {
        let memo = self.memo(1);
        let args = (
            proposer.clone(),
            self.token.address.clone(),
            to.clone(),
            amount,
            memo.clone(),
            expires_at,
        )
            .into_val(&self.env);
        auth(&self.env, signer, &self.vault_id, "propose", args, &[]);
        match self.vault.try_propose(
            proposer,
            &self.token.address,
            to,
            &amount,
            &memo,
            &expires_at,
        ) {
            Ok(Ok(id)) => Ok(id),
            Err(Ok(e)) => Err(Some(e)),
            _ => Err(None),
        }
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

// ---------------------------------------------------------------- views

#[test]
fn balance_reports_vault_holdings() {
    let s = Setup::new();
    assert_eq!(s.vault.balance(&s.token.address), 0);
    s.fund_vault(250);
    s.fund_vault(750);
    assert_eq!(s.vault.balance(&s.token.address), 1_000);
}

// ---------------------------------------------------------------- propose

#[test]
fn propose_creates_pending_proposal_with_proposer_approval() {
    let s = Setup::new();
    let to = Address::generate(&s.env);
    let id = s.propose(&s.approvers[0], &to, 400);

    let p = s.vault.get_proposal(&id);
    assert_eq!(p.id, 0);
    assert_eq!(p.token, s.token.address);
    assert_eq!(p.to, to);
    assert_eq!(p.amount, 400);
    assert_eq!(p.memo_ref, s.memo(1));
    assert_eq!(p.created_by, s.approvers[0]);
    assert_eq!(p.approvals, vec![&s.env, s.approvers[0].clone()]);
    assert_eq!(p.status, ProposalStatus::Pending);
    assert_eq!(p.expires_at, NOW + TTL);
}

#[test]
fn propose_emits_proposed_event() {
    let s = Setup::new();
    let to = Address::generate(&s.env);
    let id = s.propose(&s.approvers[0], &to, 400);
    let expected = events::Proposed {
        proposal_id: id,
        proposer: s.approvers[0].clone(),
        token: s.token.address.clone(),
        to,
        amount: 400,
    };
    assert_eq!(
        s.env
            .events()
            .all()
            .filter_by_contract(&s.vault_id)
            .events(),
        [expected.to_xdr(&s.env, &s.vault_id)]
    );
}

#[test]
fn propose_assigns_sequential_ids() {
    let s = Setup::new();
    let to = Address::generate(&s.env);
    assert_eq!(s.propose(&s.approvers[0], &to, 1), 0);
    assert_eq!(s.propose(&s.approvers[1], &to, 1), 1);
    assert_eq!(s.propose(&s.approvers[2], &to, 1), 2);
}

#[test]
fn propose_rejects_non_approver() {
    let s = Setup::new();
    let outsider = Address::generate(&s.env);
    let to = Address::generate(&s.env);
    let res = s.try_propose(&outsider, &outsider, &to, 10, NOW + TTL);
    assert_eq!(res, Err(Some(Error::NotApprover)));
}

#[test]
fn propose_rejects_wrong_signer() {
    let s = Setup::new();
    let to = Address::generate(&s.env);
    // approvers[1] signs a proposal claiming to be from approvers[0].
    let res = s.try_propose(&s.approvers[1], &s.approvers[0], &to, 10, NOW + TTL);
    assert_eq!(res, Err(None));
}

#[test]
fn propose_rejects_zero_amount() {
    let s = Setup::new();
    let to = Address::generate(&s.env);
    let res = s.try_propose(&s.approvers[0], &s.approvers[0], &to, 0, NOW + TTL);
    assert_eq!(res, Err(Some(Error::InvalidAmount)));
}

#[test]
fn propose_rejects_negative_amount() {
    let s = Setup::new();
    let to = Address::generate(&s.env);
    let res = s.try_propose(&s.approvers[0], &s.approvers[0], &to, -1, NOW + TTL);
    assert_eq!(res, Err(Some(Error::InvalidAmount)));
}

#[test]
fn propose_rejects_expiry_in_past() {
    let s = Setup::new();
    let to = Address::generate(&s.env);
    let res = s.try_propose(&s.approvers[0], &s.approvers[0], &to, 10, NOW - 1);
    assert_eq!(res, Err(Some(Error::InvalidExpiry)));
}

#[test]
fn propose_rejects_expiry_equal_to_now() {
    let s = Setup::new();
    let to = Address::generate(&s.env);
    let res = s.try_propose(&s.approvers[0], &s.approvers[0], &to, 10, NOW);
    assert_eq!(res, Err(Some(Error::InvalidExpiry)));
}

#[test]
fn get_proposal_rejects_unknown_id() {
    let s = Setup::new();
    assert_eq!(
        s.vault.try_get_proposal(&99),
        Err(Ok(Error::ProposalNotFound))
    );
}
