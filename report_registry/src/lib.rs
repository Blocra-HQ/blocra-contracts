#![no_std]
//! BlocRA report registry (Soroban).
//!
//! When an analyst generates a contract-analysis report in BlocRA, the app
//! hashes the file (sha256) and publishes the hash here with the author, the
//! chain and contract the report is about, and a title. Anyone holding a copy
//! can recompute its hash and call `verify` / `get_report_by_hash` to confirm
//! it is an unmodified BlocRA report, who published it and when.
//!
//! Built from the bounty/escrow crate (see git history): same constructor-set
//! admin, typed errors, events keyed by id, TTL handling and pause switch.

mod errors;
// Events use publish(); soroban-sdk 29 prefers #[contractevent] (a later migration).
#[allow(deprecated)]
mod events;

pub use errors::Error;
use events::*;
use soroban_sdk::{contract, contractimpl, contracttype, Address, BytesN, Env, String, Vec};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Ledgers per day at ~5s per ledger.
const DAY_IN_LEDGERS: u32 = 17_280;
const INSTANCE_TTL_EXTEND: u32 = 30 * DAY_IN_LEDGERS;
const INSTANCE_TTL_THRESHOLD: u32 = INSTANCE_TTL_EXTEND - DAY_IN_LEDGERS;

pub const MAX_TITLE_LEN: u32 = 120;
pub const MAX_CHAIN_LEN: u32 = 32;
pub const MAX_SUBJECT_LEN: u32 = 128;
pub const MAX_URI_LEN: u32 = 256;
/// Largest page `get_reports` / `get_author_reports` return.
pub const MAX_PAGE_SIZE: u32 = 50;

// ── Types ───────────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Report {
    pub id: u64,
    pub author: Address,
    /// sha256 of the report file exactly as downloaded.
    pub report_hash: BytesN<32>,
    pub title: String,
    /// Chain the analyzed contract lives on (BlocRA chain id, e.g. "starknet").
    pub chain: String,
    /// The analyzed contract's address.
    pub subject: String,
    /// Optional place to fetch the report from (https://, ipfs://).
    pub uri: Option<String>,
    pub published_at: u64,
    /// Set by the author if the report turns out to be wrong. Kept on record.
    pub revoked: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
enum DataKey {
    Admin,
    Paused,
    ReportCount,
    Report(u64),
    HashIndex(BytesN<32>),
    AuthorCount(Address),
    AuthorReport(Address, u64),
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn bump_instance(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND);
}

/// Published reports are meant to be permanent: extend to the network max.
fn bump_record(env: &Env, key: &DataKey) {
    let max = env.storage().max_ttl();
    env.storage()
        .persistent()
        .extend_ttl(key, max.saturating_sub(DAY_IN_LEDGERS), max);
}

fn set_record<V: soroban_sdk::IntoVal<Env, soroban_sdk::Val>>(env: &Env, key: &DataKey, value: &V) {
    env.storage().persistent().set(key, value);
    bump_record(env, key);
}

fn admin(env: &Env) -> Result<Address, Error> {
    env.storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(Error::Unauthorized)
}

fn check_text(s: &String, max: u32) -> Result<(), Error> {
    if s.is_empty() || s.len() > max {
        return Err(Error::InvalidText);
    }
    Ok(())
}

fn check_page(limit: u32) -> Result<(), Error> {
    if limit == 0 || limit > MAX_PAGE_SIZE {
        return Err(Error::InvalidPageSize);
    }
    Ok(())
}

fn load(env: &Env, id: u64) -> Result<Report, Error> {
    env.storage()
        .persistent()
        .get(&DataKey::Report(id))
        .ok_or(Error::ReportNotFound)
}

// ── Contract ────────────────────────────────────────────────────────────────

#[contract]
pub struct ReportRegistry;

#[contractimpl]
impl ReportRegistry {
    /// Runs once, atomically with deployment, so the admin can't be front-run.
    pub fn __constructor(env: Env, admin: Address) {
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::ReportCount, &0u64);
        bump_instance(&env);
    }

    /// Publishes a report hash on behalf of `author`. Each hash can be
    /// published once, so a report's first publisher is its author on record.
    pub fn publish_report(
        env: Env,
        author: Address,
        report_hash: BytesN<32>,
        title: String,
        chain: String,
        subject: String,
        uri: Option<String>,
    ) -> Result<u64, Error> {
        if env
            .storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false)
        {
            return Err(Error::ContractPaused);
        }
        author.require_auth();

        check_text(&title, MAX_TITLE_LEN)?;
        check_text(&chain, MAX_CHAIN_LEN)?;
        check_text(&subject, MAX_SUBJECT_LEN)?;
        if let Some(u) = &uri {
            check_text(u, MAX_URI_LEN)?;
        }
        let hash_key = DataKey::HashIndex(report_hash.clone());
        if env.storage().persistent().has(&hash_key) {
            return Err(Error::AlreadyPublished);
        }

        let id = Self::get_report_count(env.clone())
            .checked_add(1)
            .ok_or(Error::IdOverflow)?;
        let report = Report {
            id,
            author: author.clone(),
            report_hash: report_hash.clone(),
            title,
            chain,
            subject,
            uri,
            published_at: env.ledger().timestamp(),
            revoked: false,
        };
        set_record(&env, &DataKey::Report(id), &report);
        set_record(&env, &hash_key, &id);

        let n = Self::get_author_report_count(env.clone(), author.clone());
        set_record(&env, &DataKey::AuthorReport(author.clone(), n), &id);
        set_record(&env, &DataKey::AuthorCount(author.clone()), &(n + 1));

        env.storage().instance().set(&DataKey::ReportCount, &id);
        bump_instance(&env);

        emit_report_published(&env, id, author, report_hash);
        Ok(id)
    }

    /// The author withdraws a report (e.g. it contained an error). The record
    /// stays, marked revoked, so old copies can still be identified.
    pub fn revoke_report(env: Env, id: u64) -> Result<(), Error> {
        let mut report = load(&env, id)?;
        report.author.require_auth();
        if report.revoked {
            return Err(Error::AlreadyRevoked);
        }
        report.revoked = true;
        set_record(&env, &DataKey::Report(id), &report);
        bump_instance(&env);
        emit_report_revoked(&env, id, report.author);
        Ok(())
    }

    /// Admin: stop new publications (revocations and reads keep working).
    pub fn set_paused(env: Env, paused: bool) -> Result<(), Error> {
        let admin = admin(&env)?;
        admin.require_auth();
        env.storage().instance().set(&DataKey::Paused, &paused);
        bump_instance(&env);
        emit_paused(&env, admin, paused);
        Ok(())
    }

    // ── Reads ───────────────────────────────────────────────────────────────

    pub fn get_report(env: Env, id: u64) -> Result<Report, Error> {
        load(&env, id)
    }

    pub fn get_report_by_hash(env: Env, report_hash: BytesN<32>) -> Option<Report> {
        let id: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::HashIndex(report_hash))?;
        load(&env, id).ok()
    }

    /// True if this exact file was published and has not been revoked.
    pub fn verify(env: Env, report_hash: BytesN<32>) -> bool {
        Self::get_report_by_hash(env, report_hash).is_some_and(|r| !r.revoked)
    }

    /// Number of reports published (ids are `1..=count`).
    pub fn get_report_count(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::ReportCount)
            .unwrap_or(0)
    }

    /// Up to `limit` reports with ids `start, start+1, …` (newest last).
    pub fn get_reports(env: Env, start: u64, limit: u32) -> Result<Vec<Report>, Error> {
        check_page(limit)?;
        let count = Self::get_report_count(env.clone());
        let mut out = Vec::new(&env);
        let first = start.max(1);
        let end = first
            .saturating_add(limit as u64)
            .min(count.saturating_add(1));
        for id in first..end {
            if let Ok(r) = load(&env, id) {
                out.push_back(r);
            }
        }
        Ok(out)
    }

    pub fn get_author_report_count(env: Env, author: Address) -> u64 {
        env.storage()
            .persistent()
            .get(&DataKey::AuthorCount(author))
            .unwrap_or(0)
    }

    /// An author's reports in publication order, `start` counted from 0.
    pub fn get_author_reports(
        env: Env,
        author: Address,
        start: u64,
        limit: u32,
    ) -> Result<Vec<Report>, Error> {
        check_page(limit)?;
        let count = Self::get_author_report_count(env.clone(), author.clone());
        let mut out = Vec::new(&env);
        let end = start.saturating_add(limit as u64).min(count);
        for n in start..end {
            let id: Option<u64> = env
                .storage()
                .persistent()
                .get(&DataKey::AuthorReport(author.clone(), n));
            if let Some(Ok(r)) = id.map(|id| load(&env, id)) {
                out.push_back(r);
            }
        }
        Ok(out)
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
