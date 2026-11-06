# blocra-contracts

[![Contracts](https://github.com/Blocra-HQ/blocra-contracts/actions/workflows/ci.yml/badge.svg)](https://github.com/Blocra-HQ/blocra-contracts/actions/workflows/ci.yml)
![Stellar](https://img.shields.io/badge/Stellar-Soroban-blue?logo=stellar)
![Rust](https://img.shields.io/badge/Rust-1.96-orange?logo=rust)
![License](https://img.shields.io/badge/license-MIT-green)

The on-chain part of [BlocRA](https://github.com/Blocra-HQ): a Soroban smart contract on Stellar that
registers BlocRA analysis reports so anyone can verify them.

When an analyst generates a contract-analysis report in the BlocRA app, they can publish it: the app hashes
the file (sha256) and records the hash here with the author, the chain and contract the report is about, a
title and an optional URI. Anyone holding a copy recomputes its hash and calls `verify` or
`get_report_by_hash` to confirm it is an unmodified BlocRA report, who published it and when.

The [BlocRA web app](https://github.com/Blocra-HQ/blocra-frontend) uses it through the **Publish on Stellar**
button on the contract-analysis report and the **/verify-report** page.

## Live deployment (testnet)

| | |
|---|---|
| Contract | [`CD7UABTKAOO2EZ2A543GIIHQED6PPMIPK2NU47W5GKCRDJ26UPZ45CPW`](https://stellar.expert/explorer/testnet/contract/CD7UABTKAOO2EZ2A543GIIHQED6PPMIPK2NU47W5GKCRDJ26UPZ45CPW) |
| Admin | `GBHIYPPIBKVKSLPSQKIUHL3YLCAVYDM2WSHVAEQ3XB4BZM7HS2QM5NF7` |
| Network | Stellar testnet |

## Rules

- **One publication per hash.** A given report hash can be published once; its first publisher is the author
  of record. Republishing the same file fails with `AlreadyPublished`.
- **Revocation keeps the record.** An author can revoke their report (for example, if it contained an error).
  The record stays, marked `revoked`, and `verify` returns `false` for it.
- **Records are permanent.** Report records, the hash index and per-author indexes are extended to the
  network's maximum TTL when written.
- **Limited admin.** The admin is set by the constructor in the deployment transaction (it can't be
  front-run). Its only power is pausing new publications; revocation and reads keep working while paused.

## Data model

```rust
pub struct Report {
    pub id: u64,                 // 1, 2, 3, ...
    pub author: Address,
    pub report_hash: BytesN<32>, // sha256 of the file exactly as downloaded
    pub title: String,           // 1-120 chars
    pub chain: String,           // BlocRA chain id, e.g. "stellar", "starknet" (1-32 chars)
    pub subject: String,         // analyzed contract address(es) (1-128 chars)
    pub uri: Option<String>,     // optional https:// or ipfs:// location (1-256 chars)
    pub published_at: u64,       // ledger timestamp
    pub revoked: bool,
}
```

## Functions

| Function | Auth | Description |
|----------|------|-------------|
| `__constructor(admin)` | deployer | Sets the admin at deployment |
| `publish_report(author, report_hash, title, chain, subject, uri) -> u64` | `author` | Registers a report hash and returns its id |
| `revoke_report(id)` | report author | Marks a report revoked |
| `set_paused(paused)` | admin | Pauses or resumes new publications |
| `verify(report_hash) -> bool` | none | `true` if published and not revoked |
| `get_report_by_hash(report_hash) -> Option<Report>` | none | Report for a hash, if any |
| `get_report(id) -> Report` | none | Report by id |
| `get_report_count() -> u64` | none | Reports published (ids are `1..=count`) |
| `get_reports(start, limit) -> Vec<Report>` | none | Page of reports by id, `limit` 1-50 |
| `get_author_report_count(author) -> u64` | none | Reports by one author |
| `get_author_reports(author, start, limit) -> Vec<Report>` | none | An author's reports in publication order (`start` from 0), `limit` 1-50 |
| `get_admin() -> Address` | none | Admin address |
| `is_paused() -> bool` | none | Whether publishing is paused |
| `get_version() -> String` | none | Crate version |

## Errors

Returned to clients as `Error(Contract, #n)`.

| Code | Name | Meaning |
|------|------|---------|
| 1 | `Unauthorized` | Caller is not allowed to do this |
| 2 | `ReportNotFound` | No report with this id |
| 3 | `AlreadyPublished` | This exact hash was already published |
| 4 | `InvalidText` | A text field is empty or too long |
| 5 | `InvalidPageSize` | Page `limit` is 0 or above 50 |
| 6 | `AlreadyRevoked` | The report is already revoked |
| 7 | `IdOverflow` | Report id counter overflowed |
| 8 | `ContractPaused` | Publishing is paused |

## Events

| Topics | Data | When |
|--------|------|------|
| `("r_publish", id)` | `(author, report_hash)` | A report is published |
| `("r_revoke", id)` | `author` | A report is revoked |
| `("paused",)` | `(admin, paused)` | Publishing is paused or resumed |

The report id is the second topic, so indexers can follow a single report.

## Build and test

Requires Rust (pinned to 1.96.0 with the `wasm32v1-none` target in `rust-toolchain.toml`) and
[stellar-cli](https://github.com/stellar/stellar-cli) 25.2 or newer.

```bash
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
stellar contract build    # -> target/wasm32v1-none/release/report_registry.wasm (~9 KB)
```

soroban-sdk 29 refuses a plain `cargo build` for WASM targets, so always build the contract with
`stellar contract build`.

## Deploy

```bash
stellar keys generate deployer --network testnet --fund
./scripts/deploy-testnet.sh --source deployer                # admin defaults to the deployer
./scripts/deploy-testnet.sh --source deployer --admin G...   # or a separate admin (also REGISTRY_ADMIN)
```

The script builds the contract, deploys it with the admin set in the constructor, and writes
`REPORT_REGISTRY_CONTRACT_ID` to `.env.testnet`. Set that id as `VITE_REPORT_REGISTRY_CONTRACT_ID` in
[blocra-frontend](https://github.com/Blocra-HQ/blocra-frontend).

## Using it from the CLI

```bash
ID=CD7UABTKAOO2EZ2A543GIIHQED6PPMIPK2NU47W5GKCRDJ26UPZ45CPW
HASH=$(sha256sum report.pdf | cut -d' ' -f1)

# Publish (signed by the author)
stellar contract invoke --id $ID --source author --network testnet -- publish_report \
  --author $(stellar keys address author) \
  --report_hash $HASH \
  --title '"USDC transfer activity, Q4"' \
  --chain '"stellar"' \
  --subject '"CCW67TSZV3SSS2HXMBQ5JFGCKJNXKZM7UQUWUZPUTHXSTZLEO7SJMI75"'

# Verify a copy (read-only, no fee)
stellar contract invoke --id $ID --source author --network testnet --send=no -- verify --report_hash $HASH
stellar contract invoke --id $ID --source author --network testnet --send=no -- get_report_by_hash --report_hash $HASH
```

## CI

[`.github/workflows/ci.yml`](.github/workflows/ci.yml) runs on every push and pull request: rustfmt, clippy
with warnings as errors, the tests, and `stellar contract build`, then uploads `report_registry.wasm` as an
artifact. Actions are pinned to commit SHAs and the token is read-only. Dependabot keeps Cargo and Actions
dependencies current.

## Layout

```
report_registry/
  src/lib.rs       contract
  src/errors.rs    typed errors
  src/events.rs    events
  src/test.rs      tests
scripts/deploy-testnet.sh
Cargo.toml         workspace (soroban-sdk 29, strict clippy, size-optimized release profile)
rust-toolchain.toml
```

## History

1. The crate started as the Soroban escrow contract from
   [stellar-insights](https://github.com/Ndifreke000/stellar-insights), imported with its commit history.
2. It became BlocRA's bounty contract: the original Cairo `BountyContract` ported to Rust, with the reward
   held in escrow. Bounty #1 ran end to end on an earlier testnet deployment
   (`CARMRKA4…TDTZU`).
3. Bounties were removed from BlocRA, and the crate became the report registry.

BlocRA's first contract was written in Cairo for Starknet; its source is preserved at tag
[`cairo-starknet-v0`](https://github.com/Blocra-HQ/blocra-contracts/tree/cairo-starknet-v0).

## License

[MIT](LICENSE)
