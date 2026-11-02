use crate as pallet_kollectyve_credentials;
use frame::testing_prelude::*;
use polkadot_sdk::pallet_timestamp;

type Block = MockBlock<Test>;

#[frame::deps::frame_support::runtime]
mod runtime {
	#[runtime::runtime]
	#[runtime::derive(
		RuntimeCall,
		RuntimeEvent,
		RuntimeError,
		RuntimeOrigin,
		RuntimeFreezeReason,
		RuntimeHoldReason,
		RuntimeSlashReason,
		RuntimeLockId,
		RuntimeTask,
		RuntimeViewFunction
	)]
	pub struct Test;

	#[runtime::pallet_index(0)]
	pub type System = frame_system::Pallet<Test>;

	#[runtime::pallet_index(1)]
	pub type Timestamp = pallet_timestamp::Pallet<Test>;

	#[runtime::pallet_index(2)]
	pub type KollectyveId = pallet_kollectyve_id::Pallet<Test>;

	#[runtime::pallet_index(3)]
	pub type Credentials = pallet_kollectyve_credentials::Pallet<Test>;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
}

#[derive_impl(pallet_timestamp::config_preludes::TestDefaultConfig)]
impl pallet_timestamp::Config for Test {}

impl pallet_kollectyve_id::Config for Test {
	type EnrollmentOrigin = frame_system::EnsureRoot<u64>;
	type ComplianceOrigin = frame_system::EnsureRoot<u64>;
	type WeightInfo = ();
}

impl pallet_kollectyve_credentials::Config for Test {
	// Resolve identities through the real anchor, so key rotation is exercised end to end.
	type Identity = KollectyveId;
	type AnchorOrigin = frame_system::EnsureRoot<u64>;
	type WeightInfo = ();
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = KollectyveId;
}

pub fn new_test_ext() -> TestState {
	frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
}
