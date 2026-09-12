# End-to-end extrinsic tests

Exercises all eight Kollectyve extrinsics against a **live dev chain** over RPC, including
the failure paths: `BadOrigin` on both privileged origins, `CommitmentAlreadyUsed`,
`UnknownIdentity`, `NotController`, `NoIdentity` and `UnknownAssetPolicy`.

## Why this exists alongside the pallet unit tests

Each pallet's `tests.rs` runs against a **mock** runtime it defines itself, so those tests
say nothing about how the pallets are wired into the real one. They cannot catch:

- an origin bound to the wrong account in `runtime/src/configs`,
- a pallet index changed in `construct_runtime`,
- a genesis preset that does not fund, or does not privilege, who it claims to,
- an extrinsic whose encoding changed under a dependency bump.

This suite talks to the actual runtime, so it does. It is the reason the "Charlie is not
endowed in the dev genesis" trap below was found at all.

## Running

```sh
./scripts/dev-chain.sh          # in another shell; leave it running
cd tests/e2e
npm install
npm test
```

Point it elsewhere with `WS=ws://host:9944 npm test`. Exits non-zero on any failure.

**Start from a fresh chain.** The suite registers an identity for `//Bob`, so a second run
against the same chain state fails with `AlreadyRegistered`. `--dev` keeps state only
while the node runs, so restarting `dev-chain.sh` is enough.

## Dev-chain facts worth knowing

- `enrollment_key`, `compliance_key` and `sudo` are all wired to **Alice** in the `dev`
  preset, so `//Alice` signs every privileged call directly.
- Only **Alice, Bob and the two stashes** are endowed. `//Charlie` is not, so a call he
  signs is rejected by the transaction pool on the fee check (`1010`) *before* reaching
  dispatch — you never see the pallet error you were asserting. The suite funds him first
  for exactly this reason.
