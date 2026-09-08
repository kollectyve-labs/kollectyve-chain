//! The Ethereum JSON-RPC server for the Kollectyve chain.
//!
//! Translates Ethereum JSON-RPC (`eth_*`) into Substrate RPC calls against a running
//! `kollectyve-node`, so MetaMask / Remix / Hardhat can talk to `pallet-revive`.
//!
//! Defaults: connects to `ws://127.0.0.1:9944`, serves on `127.0.0.1:8545`.
//! See `scripts/dev-chain.sh`, which starts the node and this proxy together.

use pallet_revive_eth_rpc::cli;

fn main() -> anyhow::Result<()> {
	let cmd = cli::CliCommand::parse_cli()?;
	cli::run(cmd)
}
