use crate::{mock::*, Error, Event, ProviderSpecs, Providers};
use frame::deps::frame_support::{assert_noop, assert_ok};
use frame::deps::sp_core::H256;

fn specs() -> ProviderSpecs {
	ProviderSpecs { cpu_cores: 8, memory_mb: 16_384, storage_gb: 512, region: 686 }
}

fn commitment(n: u8) -> H256 {
	H256::repeat_byte(n)
}

#[test]
fn register_provider_keys_record_by_identity() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		// Backend mints the provider's custodial identity (id 0 -> account 1).
		assert_ok!(KollectyveId::register_identity(RuntimeOrigin::root(), 1, commitment(1)));

		assert_ok!(Kumulus::register_provider(RuntimeOrigin::signed(1), specs()));

		let record = Providers::<Test>::get(0).expect("record stored under identity 0");
		assert_eq!(record.specs, specs());
		assert!(record.active);
		System::assert_last_event(Event::ProviderRegistered { id: 0 }.into());
	});
}

#[test]
fn register_provider_requires_an_identity() {
	new_test_ext().execute_with(|| {
		// Account 1 has no identity yet.
		assert_noop!(
			Kumulus::register_provider(RuntimeOrigin::signed(1), specs()),
			Error::<Test>::NoIdentity
		);
	});
}

#[test]
fn re_registering_updates_the_record() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(KollectyveId::register_identity(RuntimeOrigin::root(), 1, commitment(1)));
		assert_ok!(Kumulus::register_provider(RuntimeOrigin::signed(1), specs()));

		let bigger = ProviderSpecs { cpu_cores: 16, ..specs() };
		assert_ok!(Kumulus::register_provider(RuntimeOrigin::signed(1), bigger.clone()));

		assert_eq!(Providers::<Test>::get(0).unwrap().specs, bigger);
		System::assert_last_event(Event::ProviderUpdated { id: 0 }.into());
	});
}
