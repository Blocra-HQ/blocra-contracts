use soroban_sdk::{symbol_short, Address, BytesN, Env};

// Topics are (name, bounty_id) so indexers can filter on a single bounty.

pub fn emit_bounty_created(env: &Env, bounty_id: u64, creator: Address, reward_amount: i128) {
    env.events().publish(
        (symbol_short!("b_create"), bounty_id),
        (creator, reward_amount),
    );
}

pub fn emit_participant_joined(env: &Env, bounty_id: u64, participant: Address) {
    env.events()
        .publish((symbol_short!("b_join"), bounty_id), participant);
}

pub fn emit_solution_submitted(
    env: &Env,
    bounty_id: u64,
    participant: Address,
    solution_hash: BytesN<32>,
) {
    env.events().publish(
        (symbol_short!("b_submit"), bounty_id),
        (participant, solution_hash),
    );
}

pub fn emit_winner_selected(env: &Env, bounty_id: u64, winner: Address, amount: i128) {
    env.events()
        .publish((symbol_short!("b_winner"), bounty_id), (winner, amount));
}

pub fn emit_refunded(env: &Env, bounty_id: u64, creator: Address, amount: i128) {
    env.events()
        .publish((symbol_short!("b_refund"), bounty_id), (creator, amount));
}

pub fn emit_cancelled(env: &Env, bounty_id: u64, creator: Address, amount: i128) {
    env.events()
        .publish((symbol_short!("b_cancel"), bounty_id), (creator, amount));
}

pub fn emit_dispute_raised(env: &Env, bounty_id: u64, raised_by: Address) {
    env.events()
        .publish((symbol_short!("b_dispute"), bounty_id), raised_by);
}

pub fn emit_dispute_resolved(env: &Env, bounty_id: u64, recipient: Address, amount: i128) {
    env.events()
        .publish((symbol_short!("b_resolve"), bounty_id), (recipient, amount));
}

pub fn emit_paused(env: &Env, admin: Address, paused: bool) {
    env.events()
        .publish((symbol_short!("paused"),), (admin, paused));
}
