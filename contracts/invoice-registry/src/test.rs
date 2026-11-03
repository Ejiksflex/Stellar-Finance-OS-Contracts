extern crate std;

use soroban_sdk::{BytesN, Env};

use crate::{types::Error, InvoiceRegistry, InvoiceRegistryClient};

#[test]
fn get_unknown_invoice_returns_not_found() {
    let env = Env::default();
    let id = env.register(InvoiceRegistry, ());
    let client = InvoiceRegistryClient::new(&env, &id);

    let res = client.try_get(&BytesN::from_array(&env, &[7u8; 32]));
    assert_eq!(res, Err(Ok(Error::InvoiceNotFound)));
}
