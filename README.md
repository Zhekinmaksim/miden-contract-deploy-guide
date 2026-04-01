# Deploying a Contract on Miden

A step-by-step Rust guide to deploying and interacting with a public contract on the Miden testnet.

This repository packages the official Miden Rust tutorial flow into one runnable example. Instead of jumping between several documentation pages, you can clone this repo, run one command, and inspect a working deployment end to end.

The example deploys a simple public counter contract written in MASM, submits an increment transaction, and saves the resulting account IDs so you can keep interacting with the contract later.

## What This Repository Covers

- Creating a regular wallet account for Alice
- Deploying a fungible faucet account
- Minting test assets to Alice
- Consuming Alice's notes
- Sending public P2ID notes
- Deploying a public counter contract
- Calling the contract's `increment_count` procedure
- Saving deployment artifacts for later use

## Why This Guide Exists

The official Miden tutorials are correct, but the deployment flow is split across multiple pages:

- minting and consuming notes
- deploying a counter contract
- interacting with a public contract later

This repo turns those pieces into one practical workflow and keeps the implementation close to the official examples.

## Prerequisites

Before running the project, make sure you have:

- Rust and Cargo installed
- network access to the Miden testnet RPC
- system certificates available for TLS

You can verify your toolchain with:

```bash
rustc --version
cargo --version
```

## Project Structure

```text
.
├── Cargo.toml
├── README.md
├── masm
│   ├── accounts
│   │   └── counter.masm
│   └── scripts
│       └── counter_script.masm
└── src
    └── main.rs
```

Key files:

- `src/main.rs`: Rust client that runs the full deployment flow
- `masm/accounts/counter.masm`: the public counter contract
- `masm/scripts/counter_script.masm`: transaction script that calls `increment_count`

## Step 1: Clone the Repository

```bash
git clone <your-repo-url>
cd "<repo-directory>"
```

If you are starting from scratch instead of cloning this repo:

```bash
cargo new miden-contract-deploy-guide
cd miden-contract-deploy-guide
```

Then add the dependencies from [Cargo.toml](./Cargo.toml).

## Step 2: Understand the Contract

The contract in [`masm/accounts/counter.masm`](./masm/accounts/counter.masm) exposes two public procedures:

- `get_count`: reads the current counter value from storage
- `increment_count`: increments the value stored in `miden::tutorials::counter`

The storage slot is defined once:

```masm
const COUNTER_SLOT = word("miden::tutorials::counter")
```

This keeps the storage layout explicit and easy to reuse from Rust.

## Step 3: Understand the Transaction Script

The file [`masm/scripts/counter_script.masm`](./masm/scripts/counter_script.masm) is intentionally minimal:

```masm
use external_contract::counter_contract

begin
    call.counter_contract::increment_count
end
```

At runtime, the Rust client compiles this script and links it against the compiled account component for the counter contract.

## Step 4: Understand the Rust Client

The executable in [`src/main.rs`](./src/main.rs) does three important things:

1. Initializes a Miden client backed by:
   - `store.sqlite3` for local state
   - `keystore/` for private keys
2. Runs the official account and note flow:
   - create Alice
   - deploy a faucet
   - mint and consume notes
3. Deploys and calls the counter contract:
   - compiles `counter.masm`
   - builds an immutable public account with `NoAuth`
   - submits an increment transaction

The deployer saves account IDs and transaction metadata to:

```text
artifacts/official_activity.json
```

That file lets you increment the same deployed contract later without manually copying IDs.

## Step 5: Run the Full Deployment Flow

Run the project with:

```bash
cargo run --release
```

On a successful run, you should see output shaped like this:

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

## Step 6: Inspect the Saved Artifacts

After a successful run, the project writes:

- `artifacts/official_activity.json`
- `store.sqlite3`
- `keystore/`

`artifacts/official_activity.json` contains:

- Alice's account ID
- faucet account ID
- counter contract ID
- the latest counter value
- the deployment transaction ID

This file is safe to publish only if you are comfortable exposing the public account IDs and transaction IDs. It does not contain private keys.

## Step 7: Increment the Same Contract Later

Once the contract has been deployed, you can increment it again with:

```bash
cargo run --release -- increment
```

This uses the saved counter account ID from `artifacts/official_activity.json`.

You can also pass a contract ID explicitly:

```bash
cargo run --release -- increment mtst1...
```

## Account Backup and Recovery

If you want to recover the accounts later, back up these items together:

- `keystore/`
- `store.sqlite3`
- `artifacts/official_activity.json`

What each one does:

- `keystore/` stores the serialized secret keys used by the accounts
- `store.sqlite3` stores the local Miden client state
- `artifacts/official_activity.json` stores the useful public IDs for the deployed accounts

Do not publish `keystore/`. It contains the private key material required to sign transactions.

## Example Deployment From This Repository

This repository was exercised successfully against Miden testnet. One successful counter deployment transaction was:

- [0x4fe51eb8765c4c8a64d6263576c6b942689d0a4beea49dc334bc7172dff7ec1d](https://testnet.midenscan.com/tx/0x4fe51eb8765c4c8a64d6263576c6b942689d0a4beea49dc334bc7172dff7ec1d)

That run produced:

- Alice: `mtst1ar6wcg2e3v0q7ypl2nhkktmudqqyw8fn`
- Faucet: `mtst1aqh330cvdudpvgp3fd0t22z29vppcxgu`
- Counter: `mtst1aq7dyzumafhnqqzgafyz32yzfcckmy8f`

These values are examples from one real run. Your deployment will create different IDs and transaction hashes.

## Official References

This repo is based on the official Miden documentation:

- [Mint, Consume, and Create Notes](https://0xmiden.github.io/miden-docs/imported/miden-tutorials/src/rust-client/mint_consume_create_tutorial.html)
- [Deploying a Counter Contract](https://0xmiden.github.io/miden-docs/imported/miden-tutorials/src/rust-client/counter_contract_tutorial.html)
- [Interacting with Public Smart Contracts](https://0xmiden.github.io/miden-docs/imported/miden-tutorials/src/rust-client/public_account_interaction_tutorial.html)

## Publishing This Guide to GitHub

If you want to publish this repository as-is:

```bash
git init -b main
git add .
git commit -m "Add Miden contract deployment guide"
gh repo create miden-contract-deploy-guide --public --source=. --remote=origin --push
```

If `gh auth status` reports an invalid token, fix authentication first:

```bash
gh auth login -h github.com
```

Then rerun the `gh repo create ... --push` command.
