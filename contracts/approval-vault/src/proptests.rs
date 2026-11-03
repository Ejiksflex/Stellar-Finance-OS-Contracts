//! Property tests: M-of-N execution invariants over random policies.
#![allow(clippy::arithmetic_side_effects, clippy::unwrap_used, clippy::panic)]

extern crate std;

use proptest::prelude::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger as _, MockAuth, MockAuthInvoke},
    token::{StellarAssetClient, TokenClient},
    Address, BytesN, Env, IntoVal, Vec,
};

use crate::{types::Error, ApprovalVault, ApprovalVaultClient};

fn mock(env: &Env, signer: &Address, contract: &Address, f: &str, args: Vec<soroban_sdk::Val>) {
    env.mock_auths(&[MockAuth {
        address: signer,
        invoke: &MockAuthInvoke {
            contract,
            fn_name: f,
            args,
            sub_invokes: &[],
        },
    }]);
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 48, ..ProptestConfig::default() })]

    /// For any n approvers, threshold t <= n and any set of k distinct
    /// approvals (proposer included), execute succeeds iff k >= t, and funds
    /// move iff it succeeds.
    #[test]
    fn executes_iff_threshold_met(
        n in 1u32..7,
        t_seed in 0u32..100,
        extra_seed in 0u32..100,
    ) {
        let t = 1 + t_seed % n;              // 1..=n
        let k = 1 + extra_seed % n;          // approvals incl. proposer, 1..=n

        let env = Env::default();
        env.ledger().set_timestamp(1_000);
        let mut approvers = Vec::new(&env);
        for _ in 0..n {
            approvers.push_back(Address::generate(&env));
        }
        let vault_id = env.register(ApprovalVault, (approvers.clone(), t));
        let vault = ApprovalVaultClient::new(&env, &vault_id);

        let admin = Address::generate(&env);
        let sac = env.register_stellar_asset_contract_v2(admin.clone());
        let token = TokenClient::new(&env, &sac.address());
        mock(&env, &admin, &token.address, "mint", (vault_id.clone(), 50i128).into_val(&env));
        StellarAssetClient::new(&env, &token.address).mint(&vault_id, &50);

        let to = Address::generate(&env);
        let memo = BytesN::from_array(&env, &[9; 32]);
        let proposer = approvers.get(0).unwrap();
        mock(&env, &proposer, &vault_id, "propose",
             (proposer.clone(), token.address.clone(), to.clone(), 50i128, memo.clone(), 2_000u64).into_val(&env));
        let id = vault.propose(&proposer, &token.address, &to, &50, &memo, &2_000);

        for i in 1..k {
            let a = approvers.get(i).unwrap();
            mock(&env, &a, &vault_id, "approve", (a.clone(), id).into_val(&env));
            vault.approve(&a, &id);
        }

        env.mock_auths(&[]);
        let res = vault.try_execute(&id);
        if k >= t {
            prop_assert!(res.is_ok());
            prop_assert_eq!(token.balance(&to), 50);
        } else {
            prop_assert_eq!(res, Err(Ok(Error::ThresholdNotMet)));
            prop_assert_eq!(token.balance(&vault_id), 50);
        }
    }
}
