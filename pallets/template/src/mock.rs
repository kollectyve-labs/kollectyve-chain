use crate as pallet_template;
use frame::deps::frame_support::derive_impl;
use frame::deps::sp_runtime::BuildStorage;

type Block = frame::deps::frame_system::mocking::MockBlock<Test>;

#[frame::deps::frame_support::runtime]
mod runtime {
	// The main runtime
	#[runtime::runtime]
	// Runtime Types to be generated
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
	pub type System = frame::deps::frame_system::Pallet<Test>;

	#[runtime::pallet_index(1)]
	pub type Template = pallet_template::Pallet<Test>;
}

#[derive_impl(frame::deps::frame_system::config_preludes::TestDefaultConfig)]
impl frame::deps::frame_system::Config for Test {
	type Block = Block;
}

impl pallet_template::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> frame::deps::sp_io::TestExternalities {
	frame::deps::frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
}
