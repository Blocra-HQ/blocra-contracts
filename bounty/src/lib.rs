#![no_std]
//! BlocRA bounty contract (Soroban).
//!
//! Port of the original Cairo `BountyContract` (tag `cairo-starknet-v0`) with
//! the same lifecycle: create → join → submit solution → select winner. Unlike
//! the Cairo version, which only recorded `reward_amount`, the reward is held
//! in escrow by this contract from creation until it is paid to the winner or
//! returned to the creator. The escrow mechanics (checks-effects-interactions,
//! reentrancy lock, TTL extension, pause, admin dispute resolution) come from
//! the stellar-insights escrow contract this crate was built from.

mod errors;
// Events use publish(); soroban-sdk 29 prefers #[contractevent] (a later migration).
#[allow(deprecated)]
mod events;

pub use errors::Error;
use events::*;
use soroban_sdk::{contract, contractimpl, contracttype, token, Address, BytesN, Env, String};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Ledgers per day at ~5s per ledger.
const DAY_IN_LEDGERS: u32 = 17_280;
const INSTANCE_TTL_EXTEND: u32 = 30 * DAY_IN_LEDGERS;
const INSTANCE_TTL_THRESHOLD: u32 = INSTANCE_TTL_EXTEND - DAY_IN_LEDGERS;
const RECORD_TTL_EXTEND: u32 = 90 * DAY_IN_LEDGERS;
const RECORD_TTL_THRESHOLD: u32 = RECORD_TTL_EXTEND - DAY_IN_LEDGERS;

pub const MAX_TITLE_LEN: u32 = 100;
pub const MAX_DESCRIPTION_LEN: u32 = 2_000;

// ── Types ───────────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum BountyStatus {
    /// Accepting participants and solutions; reward held in escrow.
    Open = 0,
    /// Winner selected and paid.
    Completed = 1,
    /// Deadline passed with no submissions; reward returned to the creator.
    Refunded = 2,
    /// Cancelled by the creator before anyone joined; reward returned.
    Cancelled = 3,
    /// A participant disputed it; waiting for the admin.
    Disputed = 4,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Bounty {
    pub id: u64,
    pub creator: Address,
    pub title: String,
    pub description: String,
    /// Token the reward is paid in (e.g. the USDC or XLM Stellar Asset Contract).
    pub token: Address,
    pub reward_amount: i128,
    /// Unix timestamp; solutions must be submitted by then.
    pub deadline: u64,
    pub status: BountyStatus,
    pub max_participants: u32,
    pub current_participants: u32,
    pub submission_count: u32,
    pub winner: Option<Address>,
    pub created_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
enum DataKey {
    Admin,
    Paused,
    Locked,
    BountyCount,
    Bounty(u64),
    Participant(u64, Address),
    Submission(u64, Address),
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn bump_instance(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND);
}

fn bump_record(env: &Env, key: &DataKey) {
    env.storage()
        .persistent()
        .extend_ttl(key, RECORD_TTL_THRESHOLD, RECORD_TTL_EXTEND);
}

fn admin(env: &Env) -> Result<Address, Error> {
    env.storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(Error::Unauthorized)
}

fn require_not_paused(env: &Env) -> Result<(), Error> {
    if env
        .storage()
        .instance()
        .get(&DataKey::Paused)
        .unwrap_or(false)
    {
        return Err(Error::ContractPaused);
    }
    Ok(())
}

fn require_not_locked(env: &Env) -> Result<(), Error> {
    if env
        .storage()
        .instance()
        .get(&DataKey::Locked)
        .unwrap_or(false)
    {
        return Err(Error::Reentrancy);
    }
    Ok(())
}

fn load(env: &Env, bounty_id: u64) -> Result<Bounty, Error> {
    env.storage()
        .persistent()
        .get(&DataKey::Bounty(bounty_id))
        .ok_or(Error::BountyNotFound)
}

fn save(env: &Env, bounty: &Bounty) {
    let key = DataKey::Bounty(bounty.id);
    env.storage().persistent().set(&key, bounty);
    bump_record(env, &key);
    bump_instance(env);
}

/// Token transfer out of escrow under the reentrancy lock. State must already
/// be saved (checks-effects-interactions).
fn pay_out(env: &Env, token: &Address, to: &Address, amount: i128) {
    env.storage().instance().set(&DataKey::Locked, &true);
    token::Client::new(env, token).transfer(&env.current_contract_address(), to, &amount);
    env.storage().instance().set(&DataKey::Locked, &false);
}

fn check_text(s: &String, max: u32) -> Result<(), Error> {
    if s.is_empty() || s.len() > max {
        return Err(Error::InvalidText);
    }
    Ok(())
}

// ── Contract ────────────────────────────────────────────────────────────────

#[contract]
pub struct BountyContract;

#[contractimpl]
impl BountyContract {
    /// Runs once, atomically with deployment, so nobody can front-run an
    /// `initialize` call to claim the admin role.
    pub fn __constructor(env: Env, admin: Address) {
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::BountyCount, &0u64);
        bump_instance(&env);
    }

    #[allow(clippy::too_many_arguments)]
    /// Creates a bounty and locks `reward_amount` of `token` from the creator
    /// into this contract.
    pub fn create_bounty(
        env: Env,
        creator: Address,
        token: Address,
        title: String,
        description: String,
        reward_amount: i128,
        deadline: u64,
        max_participants: u32,
    ) -> Result<u64, Error> {
        require_not_paused(&env)?;
        require_not_locked(&env)?;
        creator.require_auth();

        if reward_amount <= 0 {
            return Err(Error::InvalidReward);
        }
        let now = env.ledger().timestamp();
        if deadline <= now {
            return Err(Error::DeadlineNotInFuture);
        }
        if max_participants == 0 {
            return Err(Error::InvalidMaxParticipants);
        }
        check_text(&title, MAX_TITLE_LEN)?;
        check_text(&description, MAX_DESCRIPTION_LEN)?;

        let id = Self::get_bounty_count(env.clone())
            .checked_add(1)
            .ok_or(Error::IdOverflow)?;
        let bounty = Bounty {
            id,
            creator: creator.clone(),
            title,
            description,
            token: token.clone(),
            reward_amount,
            deadline,
            status: BountyStatus::Open,
            max_participants,
            current_participants: 0,
            submission_count: 0,
            winner: None,
            created_at: now,
        };
        env.storage().instance().set(&DataKey::BountyCount, &id);
        save(&env, &bounty);

        env.storage().instance().set(&DataKey::Locked, &true);
        token::Client::new(&env, &token).transfer(
            &creator,
            env.current_contract_address(),
            &reward_amount,
        );
        env.storage().instance().set(&DataKey::Locked, &false);

        emit_bounty_created(&env, id, creator, reward_amount);
        Ok(id)
    }

    pub fn join_bounty(env: Env, bounty_id: u64, participant: Address) -> Result<(), Error> {
        require_not_paused(&env)?;
        participant.require_auth();
        let mut bounty = load(&env, bounty_id)?;

        if bounty.status != BountyStatus::Open {
            return Err(Error::BountyNotOpen);
        }
        if env.ledger().timestamp() > bounty.deadline {
            return Err(Error::DeadlinePassed);
        }
        if bounty.creator == participant {
            return Err(Error::CreatorCannotJoin);
        }
        let key = DataKey::Participant(bounty_id, participant.clone());
        if env.storage().persistent().has(&key) {
            return Err(Error::AlreadyJoined);
        }
        if bounty.current_participants >= bounty.max_participants {
            return Err(Error::BountyFull);
        }

        env.storage().persistent().set(&key, &true);
        bump_record(&env, &key);
        bounty.current_participants += 1;
        save(&env, &bounty);

        emit_participant_joined(&env, bounty_id, participant);
        Ok(())
    }

    /// Records (or replaces) a participant's solution hash, e.g. the sha256 of
    /// the analysis report or dashboard they built.
    pub fn submit_solution(
        env: Env,
        bounty_id: u64,
        participant: Address,
        solution_hash: BytesN<32>,
    ) -> Result<(), Error> {
        require_not_paused(&env)?;
        participant.require_auth();
        let mut bounty = load(&env, bounty_id)?;

        if bounty.status != BountyStatus::Open {
            return Err(Error::BountyNotOpen);
        }
        if !Self::is_participant(env.clone(), bounty_id, participant.clone()) {
            return Err(Error::NotParticipant);
        }
        if env.ledger().timestamp() > bounty.deadline {
            return Err(Error::DeadlinePassed);
        }

        let key = DataKey::Submission(bounty_id, participant.clone());
        if !env.storage().persistent().has(&key) {
            bounty.submission_count += 1;
            save(&env, &bounty);
        }
        env.storage().persistent().set(&key, &solution_hash);
        bump_record(&env, &key);

        emit_solution_submitted(&env, bounty_id, participant, solution_hash);
        Ok(())
    }

    /// Creator picks a winner among participants who submitted; the escrowed
    /// reward is paid to them immediately.
    pub fn select_winner(env: Env, bounty_id: u64, winner: Address) -> Result<(), Error> {
        require_not_paused(&env)?;
        require_not_locked(&env)?;
        let mut bounty = load(&env, bounty_id)?;
        bounty.creator.require_auth();

        if bounty.status != BountyStatus::Open {
            return Err(Error::BountyNotOpen);
        }
        if !Self::is_participant(env.clone(), bounty_id, winner.clone()) {
            return Err(Error::NotParticipant);
        }
        if Self::get_submission(env.clone(), bounty_id, winner.clone()).is_none() {
            return Err(Error::NoSubmission);
        }

        bounty.status = BountyStatus::Completed;
        bounty.winner = Some(winner.clone());
        save(&env, &bounty);
        pay_out(&env, &bounty.token, &winner, bounty.reward_amount);

        emit_winner_selected(&env, bounty_id, winner, bounty.reward_amount);
        Ok(())
    }

    /// Creator withdraws an open bounty before anyone has joined.
    pub fn cancel_bounty(env: Env, bounty_id: u64) -> Result<(), Error> {
        require_not_locked(&env)?;
        let mut bounty = load(&env, bounty_id)?;
        bounty.creator.require_auth();

        if bounty.status != BountyStatus::Open {
            return Err(Error::BountyNotOpen);
        }
        if bounty.current_participants > 0 {
            return Err(Error::HasParticipants);
        }

        bounty.status = BountyStatus::Cancelled;
        save(&env, &bounty);
        pay_out(&env, &bounty.token, &bounty.creator, bounty.reward_amount);

        emit_cancelled(&env, bounty_id, bounty.creator, bounty.reward_amount);
        Ok(())
    }

    /// After the deadline, the creator recovers the reward if nobody submitted
    /// a solution. With submissions on the table, a winner must be chosen (or
    /// the admin decides through a dispute).
    pub fn refund(env: Env, bounty_id: u64) -> Result<(), Error> {
        require_not_locked(&env)?;
        let mut bounty = load(&env, bounty_id)?;
        bounty.creator.require_auth();

        if bounty.status != BountyStatus::Open {
            return Err(Error::BountyNotOpen);
        }
        if env.ledger().timestamp() <= bounty.deadline {
            return Err(Error::DeadlineNotPassed);
        }
        if bounty.submission_count > 0 {
            return Err(Error::HasSubmissions);
        }

        bounty.status = BountyStatus::Refunded;
        save(&env, &bounty);
        pay_out(&env, &bounty.token, &bounty.creator, bounty.reward_amount);

        emit_refunded(&env, bounty_id, bounty.creator, bounty.reward_amount);
        Ok(())
    }

    /// A participant who submitted can escalate to the admin once the deadline
    /// has passed without the creator choosing a winner.
    pub fn raise_dispute(env: Env, bounty_id: u64, participant: Address) -> Result<(), Error> {
        participant.require_auth();
        let mut bounty = load(&env, bounty_id)?;

        if bounty.status != BountyStatus::Open {
            return Err(Error::BountyNotOpen);
        }
        if env.ledger().timestamp() <= bounty.deadline {
            return Err(Error::DeadlineNotPassed);
        }
        if Self::get_submission(env.clone(), bounty_id, participant.clone()).is_none() {
            return Err(Error::NoSubmission);
        }

        bounty.status = BountyStatus::Disputed;
        save(&env, &bounty);

        emit_dispute_raised(&env, bounty_id, participant);
        Ok(())
    }

    /// Admin settles a dispute: pay `winner` (who must have submitted), or
    /// pass `None` to return the reward to the creator.
    pub fn resolve_dispute(env: Env, bounty_id: u64, winner: Option<Address>) -> Result<(), Error> {
        require_not_locked(&env)?;
        admin(&env)?.require_auth();
        let mut bounty = load(&env, bounty_id)?;

        if bounty.status != BountyStatus::Disputed {
            return Err(Error::NotDisputed);
        }

        let recipient = match winner {
            Some(w) => {
                if Self::get_submission(env.clone(), bounty_id, w.clone()).is_none() {
                    return Err(Error::NoSubmission);
                }
                bounty.status = BountyStatus::Completed;
                bounty.winner = Some(w.clone());
                w
            }
            None => {
                bounty.status = BountyStatus::Refunded;
                bounty.creator.clone()
            }
        };
        save(&env, &bounty);
        pay_out(&env, &bounty.token, &recipient, bounty.reward_amount);

        emit_dispute_resolved(&env, bounty_id, recipient, bounty.reward_amount);
        Ok(())
    }

    /// Admin: stop new bounties, joins, submissions and winner selection.
    /// Refunds, cancellations and dispute resolution keep working so funds
    /// are never trapped.
    pub fn set_paused(env: Env, paused: bool) -> Result<(), Error> {
        let admin = admin(&env)?;
        admin.require_auth();
        env.storage().instance().set(&DataKey::Paused, &paused);
        bump_instance(&env);
        emit_paused(&env, admin, paused);
        Ok(())
    }

    // ── Reads ───────────────────────────────────────────────────────────────

    pub fn get_bounty(env: Env, bounty_id: u64) -> Result<Bounty, Error> {
        load(&env, bounty_id)
    }

    /// Number of bounties ever created (ids are `1..=count`).
    pub fn get_bounty_count(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::BountyCount)
            .unwrap_or(0)
    }

    pub fn is_participant(env: Env, bounty_id: u64, participant: Address) -> bool {
        env.storage()
            .persistent()
            .has(&DataKey::Participant(bounty_id, participant))
    }

    pub fn get_submission(env: Env, bounty_id: u64, participant: Address) -> Option<BytesN<32>> {
        env.storage()
            .persistent()
            .get(&DataKey::Submission(bounty_id, participant))
    }

    pub fn get_winner(env: Env, bounty_id: u64) -> Option<Address> {
        load(&env, bounty_id).ok().and_then(|b| b.winner)
    }

    pub fn get_admin(env: Env) -> Result<Address, Error> {
        admin(&env)
    }

    pub fn is_paused(env: Env) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false)
    }

    pub fn get_version(env: Env) -> String {
        String::from_str(&env, VERSION)
    }
}

#[cfg(test)]
mod test;
