// `lib.rs`'s `use polkadot_sdk::*` does not reach child modules, so name the umbrella re-export
// here; without it `define_benchmarks!` cannot resolve and the whole benchmark API disappears.
use polkadot_sdk::frame_benchmarking;

frame_benchmarking::define_benchmarks!(
	[frame_benchmarking, BaselineBench::<Runtime>]
	[frame_system, SystemBench::<Runtime>]
	[frame_system_extensions, SystemExtensionsBench::<Runtime>]
	[pallet_balances, Balances]
	[pallet_timestamp, Timestamp]
	[pallet_sudo, Sudo]
	[pallet_kollectyve_id, KollectyveId]
	[pallet_kollectyve_policy, KollectyvePolicy]
	[pallet_kumulus, Kumulus]
);
