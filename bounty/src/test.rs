#![cfg(test)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token::{Client as TokenClient, StellarAssetClient},
    Address, BytesN, Env, String,
};

use crate::{BountyContract, BountyContractClient, BountyStatus, Error};

const REWARD: i128 = 5_000_000;
const HOUR: u64 = 3_600;

struct Setup<'a> {
    env: &'a Env,
    client: BountyContractClient<'a>,
    token: TokenClient<'a>,
    admin: Address,
    creator: Address,
}

fn setup(env: &Env) -> Setup<'_> {
    env.mock_all_auths();
    env.ledger().with_mut(|l| l.timestamp = 1_000_000);

    let admin = Address::generate(env);
    let creator = Address::generate(env);
    let token_id = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    StellarAssetClient::new(env, &token_id).mint(&creator, &(REWARD * 10));

    let id = env.register(BountyContract, (admin.clone(),));
    Setup {
        env,
        client: BountyContractClient::new(env, &id),
        token: TokenClient::new(env, &token_id),
        admin,
        creator,
    }
}

fn s(env: &Env, v: &str) -> String {
    String::from_str(env, v)
}

fn hash(env: &Env, b: u8) -> BytesN<32> {
    BytesN::from_array(env, &[b; 32])
}

fn create(t: &Setup, max_participants: u32) -> u64 {
    let deadline = t.env.ledger().timestamp() + HOUR;
    t.client.create_bounty(
        &t.creator,
        &t.token.address,
        &s(t.env, "Top DEX by volume"),
        &s(t.env, "Rank Stellar DEX pairs by 30-day volume"),
        &REWARD,
        &deadline,
        &max_participants,
    )
}

fn after_deadline(t: &Setup) {
    t.env.ledger().with_mut(|l| l.timestamp += HOUR + 1);
}

#[test]
fn create_locks_the_reward_in_escrow() {
    let env = Env::default();
    let t = setup(&env);
    let id = create(&t, 3);
    assert_eq!(id, 1);
    assert_eq!(t.client.get_bounty_count(), 1);
    assert_eq!(t.token.balance(&t.client.address), REWARD);
    let b = t.client.get_bounty(&id);
    assert_eq!(b.status, BountyStatus::Open);
    assert_eq!(b.creator, t.creator);
    assert_eq!(b.reward_amount, REWARD);
}

#[test]
fn create_validates_inputs() {
    let env = Env::default();
    let t = setup(&env);
    let now = t.env.ledger().timestamp();
    let title = s(t.env, "t");
    let desc = s(t.env, "d");
    let tok = &t.token.address;
    let c = &t.creator;
    assert_eq!(
        t.client
            .try_create_bounty(c, tok, &title, &desc, &0, &(now + HOUR), &1),
        Err(Ok(Error::InvalidReward))
    );
    assert_eq!(
        t.client
            .try_create_bounty(c, tok, &title, &desc, &REWARD, &now, &1),
        Err(Ok(Error::DeadlineNotInFuture))
    );
    assert_eq!(
        t.client
            .try_create_bounty(c, tok, &title, &desc, &REWARD, &(now + HOUR), &0),
        Err(Ok(Error::InvalidMaxParticipants))
    );
    assert_eq!(
        t.client
            .try_create_bounty(c, tok, &s(t.env, ""), &desc, &REWARD, &(now + HOUR), &1),
        Err(Ok(Error::InvalidText))
    );
}

#[test]
fn full_lifecycle_pays_the_winner() {
    let env = Env::default();
    let t = setup(&env);
    let id = create(&t, 2);
    let alice = Address::generate(t.env);
    let bob = Address::generate(t.env);

    t.client.join_bounty(&id, &alice);
    t.client.join_bounty(&id, &bob);
    assert!(t.client.is_participant(&id, &alice));
    t.client.submit_solution(&id, &alice, &hash(t.env, 1));
    t.client.submit_solution(&id, &bob, &hash(t.env, 2));
    assert_eq!(t.client.get_submission(&id, &alice), Some(hash(t.env, 1)));

    t.client.select_winner(&id, &alice);
    assert_eq!(t.token.balance(&alice), REWARD);
    assert_eq!(t.token.balance(&t.client.address), 0);
    assert_eq!(t.client.get_winner(&id), Some(alice));
    assert_eq!(t.client.get_bounty(&id).status, BountyStatus::Completed);
}

#[test]
fn join_rules_match_the_cairo_contract() {
    let env = Env::default();
    let t = setup(&env);
    let id = create(&t, 1);
    let alice = Address::generate(t.env);
    let bob = Address::generate(t.env);

    assert_eq!(
        t.client.try_join_bounty(&id, &t.creator),
        Err(Ok(Error::CreatorCannotJoin))
    );
    t.client.join_bounty(&id, &alice);
    assert_eq!(
        t.client.try_join_bounty(&id, &alice),
        Err(Ok(Error::AlreadyJoined))
    );
    assert_eq!(
        t.client.try_join_bounty(&id, &bob),
        Err(Ok(Error::BountyFull))
    );
    assert_eq!(
        t.client.try_join_bounty(&99, &bob),
        Err(Ok(Error::BountyNotFound))
    );
}

#[test]
fn submissions_need_a_participant_before_the_deadline() {
    let env = Env::default();
    let t = setup(&env);
    let id = create(&t, 2);
    let alice = Address::generate(t.env);
    assert_eq!(
        t.client.try_submit_solution(&id, &alice, &hash(t.env, 1)),
        Err(Ok(Error::NotParticipant))
    );
    t.client.join_bounty(&id, &alice);
    after_deadline(&t);
    assert_eq!(
        t.client.try_submit_solution(&id, &alice, &hash(t.env, 1)),
        Err(Ok(Error::DeadlinePassed))
    );
}

#[test]
fn winner_must_have_submitted() {
    let env = Env::default();
    let t = setup(&env);
    let id = create(&t, 2);
    let alice = Address::generate(t.env);
    t.client.join_bounty(&id, &alice);
    assert_eq!(
        t.client.try_select_winner(&id, &alice),
        Err(Ok(Error::NoSubmission))
    );
}

#[test]
fn resubmitting_counts_once() {
    let env = Env::default();
    let t = setup(&env);
    let id = create(&t, 2);
    let alice = Address::generate(t.env);
    t.client.join_bounty(&id, &alice);
    t.client.submit_solution(&id, &alice, &hash(t.env, 1));
    t.client.submit_solution(&id, &alice, &hash(t.env, 2));
    assert_eq!(t.client.get_bounty(&id).submission_count, 1);
    assert_eq!(t.client.get_submission(&id, &alice), Some(hash(t.env, 2)));
}

#[test]
fn refund_after_deadline_only_without_submissions() {
    let env = Env::default();
    let t = setup(&env);
    let id = create(&t, 2);
    let before = t.token.balance(&t.creator);
    assert_eq!(t.client.try_refund(&id), Err(Ok(Error::DeadlineNotPassed)));
    after_deadline(&t);
    t.client.refund(&id);
    assert_eq!(t.token.balance(&t.creator), before + REWARD);
    assert_eq!(t.client.get_bounty(&id).status, BountyStatus::Refunded);

    let id2 = create(&t, 2);
    let alice = Address::generate(t.env);
    t.client.join_bounty(&id2, &alice);
    t.client.submit_solution(&id2, &alice, &hash(t.env, 1));
    after_deadline(&t);
    assert_eq!(t.client.try_refund(&id2), Err(Ok(Error::HasSubmissions)));
}

#[test]
fn cancel_only_before_anyone_joins() {
    let env = Env::default();
    let t = setup(&env);
    let id = create(&t, 2);
    t.client.cancel_bounty(&id);
    assert_eq!(t.client.get_bounty(&id).status, BountyStatus::Cancelled);
    assert_eq!(t.token.balance(&t.client.address), 0);

    let id2 = create(&t, 2);
    t.client.join_bounty(&id2, &Address::generate(t.env));
    assert_eq!(
        t.client.try_cancel_bounty(&id2),
        Err(Ok(Error::HasParticipants))
    );
}

#[test]
fn dispute_lets_the_admin_pay_a_submitter() {
    let env = Env::default();
    let t = setup(&env);
    let id = create(&t, 2);
    let alice = Address::generate(t.env);
    t.client.join_bounty(&id, &alice);
    t.client.submit_solution(&id, &alice, &hash(t.env, 1));

    assert_eq!(
        t.client.try_raise_dispute(&id, &alice),
        Err(Ok(Error::DeadlineNotPassed))
    );
    after_deadline(&t);
    t.client.raise_dispute(&id, &alice);
    assert_eq!(t.client.get_bounty(&id).status, BountyStatus::Disputed);
    // Creator can't refund or pick a winner while disputed.
    assert_eq!(
        t.client.try_select_winner(&id, &alice),
        Err(Ok(Error::BountyNotOpen))
    );

    t.client.resolve_dispute(&id, &Some(alice.clone()));
    assert_eq!(t.token.balance(&alice), REWARD);
    assert_eq!(t.client.get_winner(&id), Some(alice));
}

#[test]
fn dispute_can_return_the_reward_to_the_creator() {
    let env = Env::default();
    let t = setup(&env);
    let id = create(&t, 2);
    let alice = Address::generate(t.env);
    t.client.join_bounty(&id, &alice);
    t.client.submit_solution(&id, &alice, &hash(t.env, 1));
    after_deadline(&t);
    t.client.raise_dispute(&id, &alice);
    let before = t.token.balance(&t.creator);
    t.client.resolve_dispute(&id, &None);
    assert_eq!(t.token.balance(&t.creator), before + REWARD);
    assert_eq!(t.client.get_bounty(&id).status, BountyStatus::Refunded);
}

#[test]
fn pause_blocks_new_activity_but_not_refunds() {
    let env = Env::default();
    let t = setup(&env);
    let id = create(&t, 2);
    t.client.set_paused(&true);
    assert!(t.client.is_paused());
    let now = t.env.ledger().timestamp();
    assert_eq!(
        t.client.try_create_bounty(
            &t.creator,
            &t.token.address,
            &s(t.env, "t"),
            &s(t.env, "d"),
            &REWARD,
            &(now + HOUR),
            &1
        ),
        Err(Ok(Error::ContractPaused))
    );
    assert_eq!(
        t.client.try_join_bounty(&id, &Address::generate(t.env)),
        Err(Ok(Error::ContractPaused))
    );
    t.client.cancel_bounty(&id);
    assert_eq!(t.client.get_admin(), t.admin);
}
