extern crate std;

use soroban_sdk::{
    testutils::{Address as _, Events as _, MockAuth, MockAuthInvoke},
    token::TokenClient,
    Address, BytesN, Env, Event as _, IntoVal, Val,
};

use crate::{
    events,
    types::{Error, InvoiceStatus},
    InvoiceRegistry, InvoiceRegistryClient,
};

/// Test fixture: a registry, a SAC token and a business account.
///
/// Auth is mocked explicitly per call (AGENTS.md rule 2), never with
/// `mock_all_auths`.
#[allow(dead_code)]
struct Setup<'a> {
    env: Env,
    registry_id: Address,
    registry: InvoiceRegistryClient<'a>,
    business: Address,
    token: TokenClient<'a>,
    token_admin: Address,
}

impl Setup<'_> {
    fn new() -> Self {
        let env = Env::default();
        let registry_id = env.register(InvoiceRegistry, ());
        let registry = InvoiceRegistryClient::new(&env, &registry_id);
        let token_admin = Address::generate(&env);
        let sac = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token = TokenClient::new(&env, &sac.address());
        let business = Address::generate(&env);
        Setup {
            env,
            registry_id,
            registry,
            business,
            token,
            token_admin,
        }
    }

    fn hash(&self, n: u8) -> BytesN<32> {
        BytesN::from_array(&self.env, &[n; 32])
    }

    fn try_register(
        &self,
        signer: &Address,
        business: &Address,
        id: &BytesN<32>,
        token: &Address,
        amount: i128,
    ) -> Result<(), Option<Error>> {
        let content = self.hash(0xC0);
        let args = (
            business.clone(),
            id.clone(),
            token.clone(),
            amount,
            content.clone(),
        )
            .into_val(&self.env);
        auth(&self.env, signer, &self.registry_id, "register", args, &[]);
        match self
            .registry
            .try_register(business, id, token, &amount, &content)
        {
            Ok(Ok(())) => Ok(()),
            Err(Ok(e)) => Err(Some(e)),
            _ => Err(None),
        }
    }

    /// Register invoice `n` for `amount` from the fixture business.
    fn register(&self, n: u8, amount: i128) -> BytesN<32> {
        let id = self.hash(n);
        self.try_register(
            &self.business,
            &self.business,
            &id,
            &self.token.address,
            amount,
        )
        .expect("register ok");
        id
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

#[test]
fn get_unknown_invoice_returns_not_found() {
    let s = Setup::new();
    let res = s.registry.try_get(&s.hash(7));
    assert_eq!(res, Err(Ok(Error::InvoiceNotFound)));
}

// ---------------------------------------------------------------- register

#[test]
fn register_creates_open_invoice() {
    let s = Setup::new();
    let id = s.register(1, 500);
    let inv = s.registry.get(&id);
    assert_eq!(inv.id, id);
    assert_eq!(inv.business, s.business);
    assert_eq!(inv.token, s.token.address);
    assert_eq!(inv.amount, 500);
    assert_eq!(inv.paid, 0);
    assert_eq!(inv.status, InvoiceStatus::Open);
    assert_eq!(inv.content_hash, s.hash(0xC0));
}

#[test]
fn register_emits_registered_event() {
    let s = Setup::new();
    let id = s.register(1, 500);
    let expected = events::Registered {
        id,
        business: s.business.clone(),
        token: s.token.address.clone(),
        amount: 500,
    };
    assert_eq!(
        s.env
            .events()
            .all()
            .filter_by_contract(&s.registry_id)
            .events(),
        [expected.to_xdr(&s.env, &s.registry_id)]
    );
}

#[test]
fn register_rejects_duplicate_id() {
    let s = Setup::new();
    let id = s.register(1, 500);
    assert_eq!(
        s.try_register(&s.business, &s.business, &id, &s.token.address, 900),
        Err(Some(Error::InvoiceExists))
    );
    assert_eq!(s.registry.get(&id).amount, 500);
}

#[test]
fn register_rejects_duplicate_id_from_other_business() {
    let s = Setup::new();
    let id = s.register(1, 500);
    let other = Address::generate(&s.env);
    assert_eq!(
        s.try_register(&other, &other, &id, &s.token.address, 1),
        Err(Some(Error::InvoiceExists))
    );
    assert_eq!(s.registry.get(&id).business, s.business);
}

#[test]
fn register_rejects_zero_amount() {
    let s = Setup::new();
    let id = s.hash(1);
    assert_eq!(
        s.try_register(&s.business, &s.business, &id, &s.token.address, 0),
        Err(Some(Error::InvalidAmount))
    );
}

#[test]
fn register_rejects_negative_amount() {
    let s = Setup::new();
    let id = s.hash(1);
    assert_eq!(
        s.try_register(&s.business, &s.business, &id, &s.token.address, -10),
        Err(Some(Error::InvalidAmount))
    );
}

#[test]
fn register_rejects_wrong_signer() {
    let s = Setup::new();
    let id = s.hash(1);
    let attacker = Address::generate(&s.env);
    assert_eq!(
        s.try_register(&attacker, &s.business, &id, &s.token.address, 10),
        Err(None)
    );
    assert_eq!(s.registry.try_get(&id), Err(Ok(Error::InvoiceNotFound)));
}

#[test]
fn register_rejects_unknown_token() {
    let s = Setup::new();
    let id = s.hash(1);
    let not_a_token = Address::generate(&s.env);
    assert_eq!(
        s.try_register(&s.business, &s.business, &id, &not_a_token, 10),
        Err(Some(Error::InvalidToken))
    );
}

#[test]
fn register_rejects_non_token_contract() {
    let s = Setup::new();
    let id = s.hash(1);
    // A real contract that does not implement the token interface.
    let other_registry = s.env.register(InvoiceRegistry, ());
    assert_eq!(
        s.try_register(&s.business, &s.business, &id, &other_registry, 10),
        Err(Some(Error::InvalidToken))
    );
}
