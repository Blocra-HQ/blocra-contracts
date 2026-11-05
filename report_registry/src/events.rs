use soroban_sdk::{symbol_short, Address, BytesN, Env};

// Topics are (name, report_id) so indexers can follow a single report.

pub fn emit_report_published(env: &Env, id: u64, author: Address, report_hash: BytesN<32>) {
    env.events()
        .publish((symbol_short!("r_publish"), id), (author, report_hash));
}

pub fn emit_report_revoked(env: &Env, id: u64, author: Address) {
    env.events()
        .publish((symbol_short!("r_revoke"), id), author);
}

pub fn emit_paused(env: &Env, admin: Address, paused: bool) {
    env.events()
        .publish((symbol_short!("paused"),), (admin, paused));
}
