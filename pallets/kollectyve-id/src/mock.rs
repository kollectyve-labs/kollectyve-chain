use crate as pallet_kollectyve_id;
use frame::deps::frame_support::{ord_parameter_types, traits::EitherOfDiverse};
use frame::testing_prelude::*;
use frame_system::{EnsureRoot, EnsureSignedBy};

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
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
}

// Distinct accounts for the two origins, so tests can prove the split: the enrollment key must NOT
// be able to `set_tier`; only the compliance account (a stand-in for the 2-of-3 multisig address)
// may.
ord_parameter_types! {
	pub const EnrollmentKey: u64 = 10;
	pub const ComplianceKey: u64 = 20;
}

pub type EnrollmentOrigin = EitherOfDiverse<EnsureRoot<u64>, EnsureSignedBy<EnrollmentKey, u64>>;
pub type ComplianceOrigin = EitherOfDiverse<EnsureRoot<u64>, EnsureSignedBy<ComplianceKey, u64>>;

impl pallet_kollectyve_id::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type EnrollmentOrigin = EnrollmentOrigin;
	type ComplianceOrigin = ComplianceOrigin;
	type WeightInfo = ();
}

pub fn new_test_ext() -> TestState {
	frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
}
