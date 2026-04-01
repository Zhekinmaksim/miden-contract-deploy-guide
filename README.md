# Deploying a Public Contract on Miden

Step-by-step Rust guide for deploying and calling a public contract on the Miden testnet.

This repository packages the official Miden Rust tutorial flow into one runnable example. It does not stop at compiling a contract. It also covers the account and note flow required to get a realistic end-to-end testnet run.

## Scope

This repo does the following:

- creates a regular wallet account for Alice
- deploys a fungible faucet account
- mints assets to Alice
- consumes Alice's notes
- sends public P2ID notes
- deploys a public counter contract
- calls the contract's `increment_count` procedure
- saves the resulting account IDs for later use

## Version Target

The project targets the official Miden Rust client flow based on `miden-client` `0.13.x`.

## Project Layout

```text
.
├── Cargo.toml
├── masm
│   ├── accounts
│   │   └── counter.masm
│   └── scripts
│       └── counter_script.masm
└── src
    └── main.rs
```

Files that matter:

- `masm/accounts/counter.masm`: the public counter contract
- `masm/scripts/counter_script.masm`: the transaction script that calls `increment_count`
- `src/main.rs`: the Rust deployer and runner

## How the Deployment Works

On Miden, a public contract is an account.

In this repo, the deploy flow is:

1. write the account logic in MASM
2. compile the MASM into an `AccountComponent`
3. build an immutable public account with `NoAuth`
4. register the account with the client
5. compile a transaction script that calls the contract
6. submit the transaction to increment the counter

The contract in [`masm/accounts/counter.masm`](./masm/accounts/counter.masm) exposes:

- `get_count`
- `increment_count`

The script in [`masm/scripts/counter_script.masm`](./masm/scripts/counter_script.masm) is intentionally small:

```masm
use external_contract::counter_contract

begin
    call.counter_contract::increment_count
end
```

## Prerequisites

You need:

- Rust and Cargo
- network access to the Miden testnet RPC
- system TLS certificates available to the Rust process

Check the local toolchain:

```bash
rustc --version
cargo --version
```

## Run the Full Flow

Run the full example with:

```bash
cargo run --release
```

The executable will:

1. create Alice
2. deploy a faucet
3. mint five notes of 100 tokens
4. consume those notes
5. create five public P2ID notes
6. deploy the counter contract
7. increment the counter once

## Expected Output

A successful run looks like this:

```text
Latest block: 1362924

[STEP 1] Creating a new account for Alice
Alice's account ID: mtst1...

[STEP 2] Deploying a new fungible faucet
Faucet account ID: mtst1...

[STEP 3] Minting 5 notes of 100 tokens each for Alice
Minted note #1 of 100 tokens. TX: 0x...
...

[STEP 4] Alice consumes all minted notes
Consumed 5 notes. TX: 0x...
Committed transaction 0x...

[STEP 5] Alice sends 5 notes of 50 tokens each to 5 different users
Submitted a transaction with 4 P2ID notes. TX: 0x...
Committed transaction 0x...
Submitted the final single P2ID transaction. TX: 0x...
Committed transaction 0x...

[STEP 6] Deploying and incrementing the official counter contract
Submitted counter increment transaction. TX: 0x...
Committed transaction 0x...
Counter contract ID: mtst1...
Counter value after deploy flow: 1
```

## Increment the Same Contract Later

After the first deployment run, the project saves data to `artifacts/official_activity.json`.

To increment the same counter again:

```bash
cargo run --release -- increment
```

To target a specific deployed contract:

```bash
cargo run --release -- increment mtst1...
```

## Output Files

After a successful run, you will have:

- `keystore/`
- `store.sqlite3`
- `artifacts/official_activity.json`

What they contain:

- `keystore/`: private key material used for signing
- `store.sqlite3`: local Miden client state
- `artifacts/official_activity.json`: useful public IDs and transaction metadata

## Backup and Recovery

If you want to recover the accounts later, keep these together:

- `keystore/`
- `store.sqlite3`
- `artifacts/official_activity.json`

Do not publish `keystore/`. It contains serialized secret keys.

## Real Testnet Example

This repo was executed successfully against the Miden testnet.

Example deployment transaction:

- [0x4fe51eb8765c4c8a64d6263576c6b942689d0a4beea49dc334bc7172dff7ec1d](https://testnet.midenscan.com/tx/0x4fe51eb8765c4c8a64d6263576c6b942689d0a4beea49dc334bc7172dff7ec1d)

Example output from one run:

- Alice: `mtst1ar6wcg2e3v0q7ypl2nhkktmudqqyw8fn`
- Faucet: `mtst1aqh330cvdudpvgp3fd0t22z29vppcxgu`
- Counter: `mtst1aq7dyzumafhnqqzgafyz32yzfcckmy8f`

Your run will generate different account IDs and transaction hashes.

## Official References

The implementation in this repo follows the official Miden documentation:

- [Mint, Consume, and Create Notes](https://0xmiden.github.io/miden-docs/imported/miden-tutorials/src/rust-client/mint_consume_create_tutorial.html)
- [Deploying a Counter Contract](https://0xmiden.github.io/miden-docs/imported/miden-tutorials/src/rust-client/counter_contract_tutorial.html)
- [Interacting with Public Smart Contracts](https://0xmiden.github.io/miden-docs/imported/miden-tutorials/src/rust-client/public_account_interaction_tutorial.html)
