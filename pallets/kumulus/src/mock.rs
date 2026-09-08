use crate as pallet_kumulus;
use frame::testing_prelude::*;

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
	pub type KollectyveId = pallet_kollectyve_id::Pallet<Test>;

	#[runtime::pallet_index(2)]
	pub type Kumulus = pallet_kumulus::Pallet<Test>;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
}

impl pallet_kollectyve_id::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	// Kumulus tests only need to mint identities, so both origins are root here.
	type EnrollmentOrigin = frame_system::EnsureRoot<u64>;
	type ComplianceOrigin = frame_system::EnsureRoot<u64>;
	type WeightInfo = ();
}

impl pallet_kumulus::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	// Resolve identities through the real identity anchor.
	type Identity = KollectyveId;
	type WeightInfo = ();
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = KollectyveId;
}

pub fn new_test_ext() -> TestState {
	frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
}
