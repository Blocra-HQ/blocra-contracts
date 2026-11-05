#![cfg(test)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use soroban_sdk::{
    testutils::{storage::Persistent as _, Address as _, Ledger},
    Address, BytesN, Env, String,
};

use crate::{DataKey, Error, ReportRegistry, ReportRegistryClient};

struct Setup<'a> {
    env: &'a Env,
    client: ReportRegistryClient<'a>,
    admin: Address,
}

fn setup(env: &Env) -> Setup<'_> {
    env.mock_all_auths();
    env.ledger().with_mut(|l| l.timestamp = 1_000_000);
    let admin = Address::generate(env);
    let id = env.register(ReportRegistry, (admin.clone(),));
    Setup {
        env,
        client: ReportRegistryClient::new(env, &id),
        admin,
    }
}

fn s(env: &Env, v: &str) -> String {
    String::from_str(env, v)
}

fn hash(env: &Env, b: u8) -> BytesN<32> {
    BytesN::from_array(env, &[b; 32])
}

fn publish(t: &Setup, author: &Address, b: u8) -> u64 {
    t.client.publish_report(
        author,
        &hash(t.env, b),
        &s(t.env, "USDC transfer activity, Q4"),
        &s(t.env, "starknet"),
        &s(
            t.env,
            "0x053c91253bc9682c04929ca02ed00b3e423f6710d2ee7e0d5ebb06f3ecf368a8",
        ),
        &None,
    )
}

#[test]
fn publish_and_verify() {
    let env = Env::default();
    let t = setup(&env);
    let alice = Address::generate(&env);

    let id = publish(&t, &alice, 1);
    assert_eq!(id, 1);
    assert_eq!(t.client.get_report_count(), 1);
    assert!(t.client.verify(&hash(&env, 1)));
    assert!(!t.client.verify(&hash(&env, 2)));

    let r = t.client.get_report(&id);
    assert_eq!(r.author, alice);
    assert_eq!(r.chain, s(&env, "starknet"));
    assert_eq!(r.published_at, 1_000_000);
    assert!(!r.revoked);
    assert_eq!(t.client.get_report_by_hash(&hash(&env, 1)), Some(r));
    assert_eq!(t.client.get_report_by_hash(&hash(&env, 9)), None);
}

#[test]
fn a_hash_can_only_be_published_once() {
    let env = Env::default();
    let t = setup(&env);
    let alice = Address::generate(&env);
    let mallory = Address::generate(&env);
    publish(&t, &alice, 1);
    let again = t.client.try_publish_report(
        &mallory,
        &hash(&env, 1),
        &s(&env, "Stolen"),
        &s(&env, "starknet"),
        &s(&env, "0x1"),
        &None,
    );
    assert_eq!(again, Err(Ok(Error::AlreadyPublished)));
    assert_eq!(
        t.client.get_report_by_hash(&hash(&env, 1)).unwrap().author,
        alice
    );
}

#[test]
fn validates_text_fields() {
    let env = Env::default();
    let t = setup(&env);
    let a = Address::generate(&env);
    let long = String::from_str(&env, &"x".repeat(300));
    let try_pub = |title: &String, chain: &String, uri: &Option<String>| {
        t.client
            .try_publish_report(&a, &hash(&env, 1), title, chain, &s(&env, "0x1"), uri)
    };
    assert_eq!(
        try_pub(&s(&env, ""), &s(&env, "base"), &None),
        Err(Ok(Error::InvalidText))
    );
    assert_eq!(
        try_pub(&s(&env, "t"), &long, &None),
        Err(Ok(Error::InvalidText))
    );
    assert_eq!(
        try_pub(&s(&env, "t"), &s(&env, "base"), &Some(long.clone())),
        Err(Ok(Error::InvalidText))
    );
}

#[test]
fn revoke_keeps_the_record_but_fails_verification() {
    let env = Env::default();
    let t = setup(&env);
    let alice = Address::generate(&env);
    let id = publish(&t, &alice, 1);
    t.client.revoke_report(&id);
    assert!(!t.client.verify(&hash(&env, 1)));
    assert!(t.client.get_report(&id).revoked);
    assert_eq!(
        t.client.try_revoke_report(&id),
        Err(Ok(Error::AlreadyRevoked))
    );
    assert_eq!(
        t.client.try_revoke_report(&99),
        Err(Ok(Error::ReportNotFound))
    );
}

#[test]
fn pages_all_reports_and_per_author() {
    let env = Env::default();
    let t = setup(&env);
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    for b in 1..=3 {
        publish(&t, &alice, b);
    }
    publish(&t, &bob, 10);

    let all = t.client.get_reports(&1, &10);
    assert_eq!(all.len(), 4);
    assert_eq!(t.client.get_reports(&0, &2).len(), 2);
    assert_eq!(t.client.get_reports(&5, &10).len(), 0);
    assert_eq!(
        t.client.try_get_reports(&1, &0),
        Err(Ok(Error::InvalidPageSize))
    );
    assert_eq!(
        t.client.try_get_reports(&1, &51),
        Err(Ok(Error::InvalidPageSize))
    );

    assert_eq!(t.client.get_author_report_count(&alice), 3);
    let mine = t.client.get_author_reports(&alice, &1, &10);
    assert_eq!(mine.len(), 2);
    assert_eq!(mine.get(0).unwrap().id, 2);
    assert_eq!(
        t.client
            .get_author_reports(&bob, &0, &10)
            .get(0)
            .unwrap()
            .id,
        4
    );
}

#[test]
fn records_live_for_the_network_maximum() {
    let env = Env::default();
    let t = setup(&env);
    let id = publish(&t, &Address::generate(&env), 1);
    env.as_contract(&t.client.address, || {
        let max = env.storage().max_ttl();
        assert_eq!(
            env.storage().persistent().get_ttl(&DataKey::Report(id)),
            max
        );
        assert_eq!(
            env.storage()
                .persistent()
                .get_ttl(&DataKey::HashIndex(hash(&env, 1))),
            max
        );
    });
}

#[test]
fn pause_blocks_publishing_only() {
    let env = Env::default();
    let t = setup(&env);
    let alice = Address::generate(&env);
    let id = publish(&t, &alice, 1);
    t.client.set_paused(&true);
    assert!(t.client.is_paused());
    let paused = t.client.try_publish_report(
        &alice,
        &hash(&env, 2),
        &s(&env, "t"),
        &s(&env, "base"),
        &s(&env, "0x1"),
        &None,
    );
    assert_eq!(paused, Err(Ok(Error::ContractPaused)));
    t.client.revoke_report(&id);
    assert_eq!(t.client.get_admin(), t.admin);
}
