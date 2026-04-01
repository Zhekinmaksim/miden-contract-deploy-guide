# X Thread: Deploying a Public Contract on Miden

Use this thread with the rendered PNG slide deck in `social/slides/png/`.

## Post 1

If you want a minimal, working example of deploying a public contract on Miden, this repo runs the full flow end to end.

It does more than compile MASM. It creates accounts, handles notes, deploys the contract, and submits a real increment transaction on testnet.

Repo:
https://github.com/Zhekinmaksim/miden-contract-deploy-guide

Attach: `slides/png/01-cover.png`

## Post 2

On Miden, a public contract is an account.

The deploy path in this repo is:

1. write MASM
2. compile an `AccountComponent`
3. build an immutable public account with `NoAuth`
4. submit a tx script that calls the contract

Attach: `slides/png/02-model.png`

## Post 3

The contract example is a simple public counter:

- `get_count`
- `increment_count`

The transaction script is intentionally small:

`call.counter_contract::increment_count`

The useful part is the surrounding Rust client flow that makes deployment reproducible.

Attach: `slides/png/03-flow.png`

## Post 4

One command runs the full testnet flow:

`cargo run --release`

That creates Alice, deploys a faucet, mints and consumes notes, sends public P2ID notes, deploys the counter contract, and increments it once.

Attach: `slides/png/04-run.png`

## Post 5

This run produced a real public counter deployment on Miden testnet:

Counter:
`mtst1aq7dyzumafhnqqzgafyz32yzfcckmy8f`

TX:
https://testnet.midenscan.com/tx/0x4fe51eb8765c4c8a64d6263576c6b942689d0a4beea49dc334bc7172dff7ec1d

Attach: `slides/png/05-proof.png`

## Post 6

If you want account recovery later, back up these three things together:

- `keystore/`
- `store.sqlite3`
- `artifacts/official_activity.json`

`keystore/` contains the private key material. Do not publish it.

Attach: `slides/png/06-backup.png`

## Post 7

Code:
https://github.com/Zhekinmaksim/miden-contract-deploy-guide

Official Miden sources:
- Mint/consume notes
- Counter deployment
- Public contract interaction

This repo turns those docs into one runnable path.

Attach: `slides/png/07-links.png`
