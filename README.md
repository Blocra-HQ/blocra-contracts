# blocra-contracts

[![Contracts](https://github.com/Blocra-HQ/blocra-contracts/actions/workflows/ci.yml/badge.svg)](https://github.com/Blocra-HQ/blocra-contracts/actions/workflows/ci.yml)
![Stellar](https://img.shields.io/badge/Stellar-Soroban-blue?logo=stellar)
![Rust](https://img.shields.io/badge/Rust-1.96-orange?logo=rust)

The on-chain half of [BlocRA](https://github.com/Blocra-HQ)'s analytics bounties: a Soroban smart
contract on Stellar that holds a bounty's reward in escrow and pays it to the analyst whose work is chosen.

A team that needs an on-chain question answered ("which DEX pairs grew fastest this month?") posts a
bounty with a reward. Analysts join, do the work in the [BlocRA app](https://github.com/Blocra-HQ/blocra-frontend),
and submit a hash of their report. The creator picks a winner and the contract pays them.

## Live deployment (testnet)

| | |
|---|---|
| Contract | [`CARMRKA4AWVZUFZKT34BZLHMBFJ3KNOAZL7BP3T2RJSFA26FETITDTZU`](https://stellar.expert/explorer/testnet/contract/CARMRKA4AWVZUFZKT34BZLHMBFJ3KNOAZL7BP3T2RJSFA26FETITDTZU) |
| Admin | `GBHIYPPIBKVKSLPSQKIUHL3YLCAVYDM2WSHVAEQ3XB4BZM7HS2QM5NF7` |
| Network | Stellar testnet |

Bounty #1 on it has gone through the full flow (created with 10 XLM, joined, submitted, paid out).

## Lifecycle

```
              create_bounty  (reward moves creator → contract)
                     │
                     ▼
   ┌──────────────► Open ◄──── join_bounty / submit_solution (until deadline)
   │                 │
   │   ┌─────────────┼──────────────────┬───────────────────────────┐
   │   ▼             ▼                  ▼                           ▼
   │ cancel_bounty  select_winner     refund                    raise_dispute
   │ (nobody joined) (creator picks   (after deadline,          (after deadline,
   │                  a submitter)     no submissions)           by a submitter)
   │   │             │                  │                           │
   │   ▼             ▼                  ▼                           ▼
   │ Cancelled     Completed          Refunded                   Disputed
   │ reward →      reward →           reward →                      │ resolve_dispute (admin)
   │ creator       winner             creator                       ├──► Completed (pay a submitter)
   └───────────────────────────────────────────────────────────────┘└──► Refunded  (back to creator)
```

The reward can never get stuck: there is always a way out of `Open` and `Disputed`, and pausing the
contract blocks new activity but never refunds, cancellations or dispute resolution.

## Functions

| Function | Who | What it does |
|----------|-----|--------------|
| `__constructor(admin)` | deployer | Sets the admin in the deploy transaction, so it can't be front-run |
| `create_bounty(creator, token, title, description, reward_amount, deadline, max_participants) → id` | creator | Opens a bounty and transfers the reward into escrow |
| `join_bounty(bounty_id, participant)` | analyst | Joins before the deadline (not the creator, no duplicates, up to `max_participants`) |
| `submit_solution(bounty_id, participant, solution_hash)` | participant | Records the sha256 of their report; resubmitting replaces it |
| `select_winner(bounty_id, winner)` | creator | Pays the reward to a participant who submitted |
| `cancel_bounty(bounty_id)` | creator | Withdraws the bounty while nobody has joined |
| `refund(bounty_id)` | creator | After the deadline, if nobody submitted |
| `raise_dispute(bounty_id, participant)` | submitter | After the deadline, if no winner was picked |
| `resolve_dispute(bounty_id, winner?)` | admin | Pays a submitter, or `None` to return the reward |
| `set_paused(bool)` | admin | Stops new bounties, joins, submissions and payouts to winners |
| `get_bounty`, `get_bounty_count`, `is_participant`, `get_submission`, `get_winner`, `get_admin`, `is_paused`, `get_version` | anyone | Reads |

Events (`b_create`, `b_join`, `b_submit`, `b_winner`, `b_refund`, `b_cancel`, `b_dispute`, `b_resolve`) carry
the bounty id as their second topic, so indexers can follow a single bounty.

Rewards can be in any Stellar Asset Contract token (XLM, USDC, ...). Amounts are in the token's smallest
unit (stroops for XLM: 1 XLM = 10,000,000).

## Build and test

Requires Rust (the version is pinned in `rust-toolchain.toml`) and [stellar-cli](https://github.com/stellar/stellar-cli) 25.2 or newer.

```bash
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test                    # 12 tests covering every path above
stellar contract build        # → target/wasm32v1-none/release/bounty.wasm (~14 KB)
```

soroban-sdk 29 refuses plain `cargo build` for WASM, so always build through `stellar contract build`.

## Deploy

```bash
stellar keys generate deployer --network testnet --fund
./scripts/deploy-testnet.sh --source deployer          # admin defaults to the deployer
./scripts/deploy-testnet.sh --source deployer --admin G...   # or a separate admin
```

The contract ID is written to `.env.testnet`. Set it as `VITE_BOUNTY_CONTRACT_ID` in
[blocra-frontend](https://github.com/Blocra-HQ/blocra-frontend).

## Layout

```
bounty/
  src/lib.rs      contract
  src/errors.rs   typed errors (Error(Contract, #n))
  src/events.rs   events
  src/test.rs     tests
scripts/deploy-testnet.sh
```

## History

- **Escrow base:** the `bounty` crate started as the escrow contract from
  [stellar-insights](https://github.com/Ndifreke000/stellar-insights), imported with its commit history.
  That's where the checks-effects-interactions ordering, the reentrancy lock, TTL extension and pause come from.
- **Cairo predecessor:** BlocRA's first bounty contract was written in Cairo for Starknet, deployed at
  `0x07070d91…1eee`. It recorded rewards but held no funds. Its source is preserved at tag
  [`cairo-starknet-v0`](https://github.com/Blocra-HQ/blocra-contracts/tree/cairo-starknet-v0).

## License

MIT
