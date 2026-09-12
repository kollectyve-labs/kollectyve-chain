# kollectyve-chain
Kollectyve Network Blockchain node based on the [Polkadot SDK Solochain Template](https://github.com/paritytech/polkadot-sdk/tree/master/templates/solochain).

## Getting Started

Depending on your operating system and Rust version, there might be additional
packages required to compile this node. Check the
[Install](https://docs.substrate.io/install/) instructions for your platform for
the most common dependencies. Alternatively, you can use one of the [alternative
installation](#alternatives-installations) options.

### Build

Use the following command to build the node without launching it:

```sh
cargo build --release --workspace
```

This produces two binaries:

- `target/release/kollectyve-node` — the chain itself (substrate RPC on `:9944`).
- `target/release/eth-rpc` — the Ethereum JSON-RPC proxy for `pallet-revive` (`:8545`).

The runtime wasm is built for the `wasm32v1-none` target:

```sh
rustup target add wasm32v1-none
```

If you add the target after a failed build, delete `target/release/wbuild` before retrying —
wasm-builder does not rebuild across a target change.

Depending on your Rust version, linking the runtime wasm may also fail with
`undefined symbol: ext_storage_get_version_1` (and every other `ext_*` host function).
sp-runtime-interface emits those as bare `extern "C"` blocks and expects the linker to turn
them into wasm imports; newer `wasm-ld` rejects them instead. Fix by extending the wasm
rustflags — the `.cargo/config.toml` below already does this:

```sh
export WASM_BUILD_RUSTFLAGS="-C link-arg=--import-undefined"
```

Beyond a Rust toolchain, the build needs `protoc` (litep2p) and `libclang` (rocksdb
bindings). On Debian/Ubuntu:

```sh
sudo apt install protobuf-compiler libclang-dev
```

If you cannot install system packages, drop a `protoc` release binary and a `libclang.so`
symlink under `~/.local`, then point cargo at them from a **`.cargo/config.toml`** in this
directory (git-ignored, since the paths are machine-specific):

```toml
[env]
PROTOC = "/home/you/.local/bin/protoc"
LIBCLANG_PATH = "/home/you/.local/lib"
BINDGEN_EXTRA_CLANG_ARGS = "-I/usr/lib/gcc/x86_64-linux-gnu/14/include"
```

Prefer this over `export`ing the variables: cargo applies them on every invocation, whereas
exports are lost between shells. Entries in `[env]` never override a variable that is
already set, so the file is harmless once the real packages are installed.

### Embedded Docs

After you build the project, you can use the following command to explore its
parameters and subcommands:

```sh
./target/release/kollectyve-node -h
```

You can generate and view the [Rust
Docs](https://doc.rust-lang.org/cargo/commands/cargo-doc.html) for this project
with this command:

```sh
cargo +nightly doc --open
```

### Single-Node Development Chain

The following command starts a single-node development chain that doesn't
persist state:

```sh
./target/release/kollectyve-node --dev
```

To purge the development chain's state, run the following command:

```sh
./target/release/kollectyve-node purge-chain --dev
```

To start the development chain with detailed logging, run the following command:

```sh
RUST_BACKTRACE=1 ./target/release/kollectyve-node -ldebug --dev
```

Development chains:

- Maintain state in a `tmp` folder while the node is running.
- Use the **Alice** and **Bob** accounts as default validator authorities.
- Use the **Alice** account as the default `sudo` account.
- Are preconfigured with a genesis state (`/node/src/chain_spec.rs`) that
  includes several pre-funded development accounts.

### Tests

```sh
cargo test --release --workspace       # 39 pallet unit tests, against mock runtimes
```

Those use each pallet's own mock runtime, so they say nothing about how the pallets are
wired into the real one. `tests/e2e` covers that gap — it drives all eight extrinsics and
their failure paths over RPC against a live dev chain, which is what catches a mis-wired
origin, a changed pallet index or a genesis preset that does not fund who it claims to:

```sh
./scripts/dev-chain.sh                 # in another shell
cd tests/e2e && npm install && npm test
```

Start it against a **fresh** chain: the suite registers an identity for `//Bob`, so a
second run on the same state fails with `AlreadyRegistered`. See `tests/e2e/README.md`.

There is no CI pipeline; run the suites above locally before opening a pull request.

### Benchmarks & weights

Every Kollectyve extrinsic carries a benchmarked weight; the values live in each pallet's
`src/weights.rs` and are wired into the runtime as `SubstrateWeight<Runtime>`. Regenerate them on
new hardware with a benchmark-enabled build:

```sh
# `pallet-revive-fixtures` shells out to `solc` to build fixture contracts for pallet-revive's own
# benchmarks. We do not benchmark that pallet, so skip them rather than installing solc.
SKIP_PALLET_REVIVE_FIXTURES=1 cargo build --release --features runtime-benchmarks -p kollectyve-node

WASM=target/release/wbuild/kollectyve-runtime/kollectyve_runtime.compact.compressed.wasm

for p in kollectyve-id kollectyve-policy kumulus; do
  ./target/release/kollectyve-node benchmark pallet \
    --runtime "$WASM" --genesis-builder=runtime --genesis-builder-preset=development \
    --pallet "pallet_${p//-/_}" --extrinsic '*' \
    --steps 50 --repeat 20 --wasm-execution compiled \
    --template .maintain/frame-weight-template.hbs \
    --output "pallets/$p/src/weights.rs"
done
```

Take the runtime blob from the *same* `runtime-benchmarks` build — the default one has no
benchmark code and the run fails. Passing `--chain dev` instead of `--runtime` still works, but
builds genesis from the chain spec, which the CLI deprecated: it warns that it "may become a hard
error any time after December 2024". `development` is `sp_genesis_builder::DEV_RUNTIME_PRESET`, the
same preset `--chain dev` resolves to, so the measurements are unchanged; generated headers just
record `CHAIN: None` rather than `Some("dev")`.

`.maintain/frame-weight-template.hbs` emits `frame::deps::*` paths, since the pallets depend on the
`frame` omnibus rather than on `frame-support` directly. The `runtime-benchmarks` feature changes
the feature set for the whole dependency graph, so the first such build is a cold one.

Each generated file records the `STEPS`/`REPEAT` and the CPU it was measured on in its header. The
currently committed values were taken at reduced fidelity (`--steps 20 --repeat 5`) on a developer
laptop; re-run the loop above at `50`/`20` on the target hardware before a production release.

**Let the machine go idle first.** Benchmarking straight after the cold build above measures a hot,
downclocked CPU and inflates every weight — on a throttled laptop (85 °C, ~2.1 GHz against a
4.4 GHz max) all eight extrinsics came out 42-94% over their committed values, reproducing them to
within 1% once the machine had settled. The inflation is uniform and leaves storage reads/writes
and PoV sizes byte-identical, so it reads exactly like a real regression and nothing in the
generated file flags it. Always diff a fresh run against the committed values before adopting it:
a large drift with unchanged reads/writes and PoV is thermal, not code.

Two things that look like they should help but do not:

- **Do not pin with `taskset`.** On a hybrid CPU it is tempting to pin to the P-cores, but
  confining the benchmark and its wasm-compilation threads to a couple of physical cores makes them
  contend: pinning to `0-3` on an i5-1235U read ~34% *higher* than leaving the scheduler alone,
  and far less repeatably.
- **Keep `--extrinsic '*'`.** Re-running one extrinsic on its own measures a colder cache than it
  sees in the full-pallet sweep — `rotate_controller` alone reads ~24.5us against ~17.0us in the
  sweep, reproducibly in both cases. They are measuring different things; the sweep is the
  reference, and it is what the committed values come from.

`cargo test --features runtime-benchmarks` runs each benchmark once and checks its post-state,
which catches a benchmark that silently stops exercising its extrinsic.

## Smart Contracts (EVM)

The runtime includes [`pallet-revive`](https://docs.rs/pallet-revive) with the REVM backend
enabled (`AllowEVMBytecode`), so **contracts compiled by plain `solc` run unmodified** —
whatever Remix, Hardhat or Foundry produce. PolkaVM blobs from
[`resolc`](https://github.com/paritytech/revive) are accepted too, but are not required.

Ethereum tooling does not speak the substrate RPC, so a proxy translates between the two:

```
MetaMask / Remix / Hardhat  ──http :8545──▶  eth-rpc  ──ws :9944──▶  kollectyve-node
```

### Start a local chain

```sh
./scripts/dev-chain.sh
```

That runs `kollectyve-node --dev` and `eth-rpc` together and tails both logs; Ctrl-C stops
the pair. Check it is alive:

```sh
curl -s -X POST http://127.0.0.1:8545 -H 'content-type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"eth_chainId","params":[]}'
# {"jsonrpc":"2.0","id":1,"result":"0x6d60"}   # 28000
```

### Development accounts

The `dev` genesis preset funds the well-known Ethereum dev accounts and pre-registers their
`AccountId32 <-> H160` mapping, so they can deploy immediately:

| Account | Address | Private key (public, dev only) |
|---|---|---|
| alith | `0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac` | `0x5fb92d6e98884f76de468fa3f6278f8807c48bebc13595d45af5bdc4da702133` |
| baltathar | `0x3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0` | `0x8075991ce870b93a8870eca0c0f91913d12f47948ca0fd25b49c6fa7cdbeee8b` |

`charleth`, `dorothy` and `ethan` are funded as well — see
`runtime/src/genesis_config_presets.rs`. Any other account works too: `AutoMap` maps an
unknown Ethereum address on its first transaction.

### Test locally with Hardhat

```sh
cd contracts
npm install
npm test          # hardhat test --network kollectyveLocal
npm run deploy    # deploy + exercise Storage.sol
```

The project targets solc `0.8.28` with `evmVersion: "cancun"`, which is the instruction set
the pallet's REVM backend implements.

### Deploy from Remix

1. Add the chain to MetaMask: RPC `http://127.0.0.1:8545`, chain ID `28000`, symbol `KOL`.
2. Import the `alith` private key above, so you have a funded account.
3. Open [remix.ethereum.org](https://remix.ethereum.org), paste `contracts/src/Storage.sol`.
4. Compile with 0.8.28 (Advanced Configurations → EVM version `cancun`).
5. Deploy & Run → Environment **Injected Provider – MetaMask** → Deploy.

Blocks are authored every 6s, so a deploy confirms in a few seconds.

### Connect with Polkadot-JS Apps Front-End

After you start the node locally, you can interact with it using the
hosted version of the [Polkadot/Substrate
Portal](https://polkadot.js.org/apps/#/explorer?rpc=ws://localhost:9944)
front-end by connecting to the local node endpoint.