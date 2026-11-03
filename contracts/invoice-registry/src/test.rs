extern crate std;

use soroban_sdk::{
    testutils::{Address as _, Events as _, MockAuth, MockAuthInvoke},
    token::{StellarAssetClient, TokenClient},
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

    fn mint(&self, to: &Address, amount: i128) {
        let args: soroban_sdk::Vec<Val> = (to.clone(), amount).into_val(&self.env);
        auth(
            &self.env,
            &self.token_admin,
            &self.token.address,
            "mint",
            args,
            &[],
        );
        StellarAssetClient::new(&self.env, &self.token.address).mint(to, &amount);
    }

    /// A fresh payer holding `balance` tokens.
    fn payer(&self, balance: i128) -> Address {
        let p = Address::generate(&self.env);
        self.mint(&p, balance);
        p
    }

    fn try_pay(
        &self,
        signer: &Address,
        payer: &Address,
        id: &BytesN<32>,
        amount: i128,
    ) -> Result<(), Option<Error>> {
        let args = (payer.clone(), id.clone(), amount).into_val(&self.env);
        let sub = [MockAuthInvoke {
            contract: &self.token.address,
            fn_name: "transfer",
            args: (payer.clone(), self.business.clone(), amount).into_val(&self.env),
            sub_invokes: &[],
        }];
        auth(&self.env, signer, &self.registry_id, "pay", args, &sub);
        match self.registry.try_pay(payer, id, &amount) {
            Ok(Ok(())) => Ok(()),
            Err(Ok(e)) => Err(Some(e)),
            _ => Err(None),
        }
    }

    fn pay(&self, payer: &Address, id: &BytesN<32>, amount: i128) {
        self.try_pay(payer, payer, id, amount).expect("pay ok");
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

// ---------------------------------------------------------------- pay

#[test]
fn pay_in_full_transfers_and_marks_paid() {
    let s = Setup::new();
    let id = s.register(1, 500);
    let payer = s.payer(1_000);
    s.pay(&payer, &id, 500);

    assert_eq!(s.token.balance(&payer), 500);
    assert_eq!(s.token.balance(&s.business), 500);
    assert_eq!(s.token.balance(&s.registry_id), 0);
    let inv = s.registry.get(&id);
    assert_eq!(inv.paid, 500);
    assert_eq!(inv.status, InvoiceStatus::Paid);
}

#[test]
fn pay_emits_paid_event() {
    let s = Setup::new();
    let id = s.register(1, 500);
    let payer = s.payer(1_000);
    s.pay(&payer, &id, 200);
    let expected = events::Paid {
        id,
        payer,
        amount: 200,
        total_paid: 200,
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
fn partial_payments_accumulate_until_paid() {
    let s = Setup::new();
    let id = s.register(1, 500);
    let payer = s.payer(1_000);

    s.pay(&payer, &id, 200);
    let inv = s.registry.get(&id);
    assert_eq!((inv.paid, inv.status), (200, InvoiceStatus::Open));

    s.pay(&payer, &id, 250);
    let inv = s.registry.get(&id);
    assert_eq!((inv.paid, inv.status), (450, InvoiceStatus::Open));

    s.pay(&payer, &id, 50);
    let expected = events::Paid {
        id: id.clone(),
        payer: payer.clone(),
        amount: 50,
        total_paid: 500,
    };
    assert_eq!(
        s.env
            .events()
            .all()
            .filter_by_contract(&s.registry_id)
            .events(),
        [expected.to_xdr(&s.env, &s.registry_id)]
    );
    let inv = s.registry.get(&id);
    assert_eq!((inv.paid, inv.status), (500, InvoiceStatus::Paid));
    assert_eq!(s.token.balance(&s.business), 500);
}

#[test]
fn multiple_payers_can_settle_one_invoice() {
    let s = Setup::new();
    let id = s.register(1, 300);
    let a = s.payer(100);
    let b = s.payer(200);
    s.pay(&a, &id, 100);
    s.pay(&b, &id, 200);
    assert_eq!(s.registry.get(&id).status, InvoiceStatus::Paid);
    assert_eq!(s.token.balance(&s.business), 300);
}

#[test]
fn pay_rejects_overpayment_in_one_go() {
    let s = Setup::new();
    let id = s.register(1, 500);
    let payer = s.payer(1_000);
    assert_eq!(
        s.try_pay(&payer, &payer, &id, 501),
        Err(Some(Error::Overpayment))
    );
    assert_eq!(s.token.balance(&payer), 1_000);
    assert_eq!(s.registry.get(&id).paid, 0);
}

#[test]
fn pay_rejects_overpayment_after_partial() {
    let s = Setup::new();
    let id = s.register(1, 500);
    let payer = s.payer(1_000);
    s.pay(&payer, &id, 400);
    assert_eq!(
        s.try_pay(&payer, &payer, &id, 101),
        Err(Some(Error::Overpayment))
    );
    assert_eq!(s.registry.get(&id).paid, 400);
}

#[test]
fn pay_rejects_paid_invoice() {
    let s = Setup::new();
    let id = s.register(1, 500);
    let payer = s.payer(1_000);
    s.pay(&payer, &id, 500);
    assert_eq!(s.try_pay(&payer, &payer, &id, 1), Err(Some(Error::NotOpen)));
}

#[test]
fn pay_rejects_zero_amount() {
    let s = Setup::new();
    let id = s.register(1, 500);
    let payer = s.payer(1_000);
    assert_eq!(
        s.try_pay(&payer, &payer, &id, 0),
        Err(Some(Error::InvalidAmount))
    );
}

#[test]
fn pay_rejects_negative_amount() {
    let s = Setup::new();
    let id = s.register(1, 500);
    let payer = s.payer(1_000);
    assert_eq!(
        s.try_pay(&payer, &payer, &id, -1),
        Err(Some(Error::InvalidAmount))
    );
}

#[test]
fn pay_rejects_unknown_invoice() {
    let s = Setup::new();
    let payer = s.payer(1_000);
    assert_eq!(
        s.try_pay(&payer, &payer, &s.hash(9), 1),
        Err(Some(Error::InvoiceNotFound))
    );
}

#[test]
fn pay_rejects_wrong_signer() {
    let s = Setup::new();
    let id = s.register(1, 500);
    let victim = s.payer(1_000);
    let attacker = Address::generate(&s.env);
    assert_eq!(s.try_pay(&attacker, &victim, &id, 500), Err(None));
    assert_eq!(s.token.balance(&victim), 1_000);
    assert_eq!(s.registry.get(&id).paid, 0);
}

#[test]
fn pay_fails_atomically_with_insufficient_funds() {
    let s = Setup::new();
    let id = s.register(1, 500);
    let payer = s.payer(100);
    assert_eq!(s.try_pay(&payer, &payer, &id, 200), Err(None));
    // State update is rolled back with the failed transfer.
    let inv = s.registry.get(&id);
    assert_eq!((inv.paid, inv.status), (0, InvoiceStatus::Open));
}

#[test]
fn pay_rejects_overflowing_total() {
    let s = Setup::new();
    let id = s.register(1, i128::MAX);
    let payer = s.payer(i128::MAX);
    s.pay(&payer, &id, i128::MAX - 1);
    assert_eq!(
        s.try_pay(&payer, &payer, &id, i128::MAX),
        Err(Some(Error::ArithmeticOverflow))
    );
}
