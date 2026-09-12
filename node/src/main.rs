//! Kollectyve Node CLI library.
#![warn(missing_docs)]
// Every CLI entry point returns `sc_cli::Result`, whose error type is large by
// construction. Boxing it here would fight the upstream signatures for no gain.
#![allow(clippy::result_large_err)]

use polkadot_sdk::*;

mod benchmarking;
mod chain_spec;
mod cli;
mod command;
mod rpc;
mod service;

fn main() -> sc_cli::Result<()> {
	command::run()
}
