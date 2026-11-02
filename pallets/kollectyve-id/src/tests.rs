use crate::{
	mock::*, ControllerOf, Error, Event, IdentityOf, NextControllerCommitment, TierOf,
	UsedCommitments,
};
use frame::deps::frame_support::{assert_noop, assert_ok};
use frame::deps::sp_core::H256;
use frame::deps::sp_runtime::DispatchError;
use kollectyve_primitives::{IdentityLookup, KycTier};

/// A distinct commitment per test subject. Carries no meaning beyond uniqueness.
fn commitment(n: u8) -> H256 {
	H256::repeat_byte(n)
}

// --- register_identity ------------------------------------------------------

#[test]
fn register_identity_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		// The enrollment key (account 10) enrolls a custodial identity for controller 1.
		assert_ok!(KollectyveId::register_identity(
			RuntimeOrigin::signed(10),
			1,
			commitment(0xaa)
		));

		assert_eq!(ControllerOf::<Test>::get(0), Some(1));
		assert_eq!(IdentityOf::<Test>::get(1), Some(0));
		// Tier is always `None` at registration — no tier argument exists.
		assert_eq!(TierOf::<Test>::get(0), KycTier::None);
		assert_eq!(UsedCommitments::<Test>::get(commitment(0xaa)), Some(0));
		System::assert_last_event(
			Event::IdentityRegistered { id: 0, controller: 1, commitment: commitment(0xaa) }.into(),
		);
	});
}

#[test]
fn register_identity_allocates_monotonic_ids() {
	new_test_ext().execute_with(|| {
		assert_ok!(KollectyveId::register_identity(RuntimeOrigin::root(), 1, commitment(1)));
		assert_ok!(KollectyveId::register_identity(RuntimeOrigin::root(), 2, commitment(2)));
		assert_eq!(IdentityOf::<Test>::get(1), Some(0));
		assert_eq!(IdentityOf::<Test>::get(2), Some(1));
	});
}

#[test]
fn register_identity_rejects_duplicate_controller() {
	new_test_ext().execute_with(|| {
		assert_ok!(KollectyveId::register_identity(RuntimeOrigin::root(), 1, commitment(1)));
		assert_noop!(
			KollectyveId::register_identity(RuntimeOrigin::root(), 1, commitment(2)),
			Error::<Test>::AlreadyRegistered
		);
	});
}

#[test]
fn register_identity_rejects_duplicate_commitment() {
	new_test_ext().execute_with(|| {
		assert_ok!(KollectyveId::register_identity(RuntimeOrigin::root(), 1, commitment(7)));
		assert_noop!(
			KollectyveId::register_identity(RuntimeOrigin::root(), 2, commitment(7)),
			Error::<Test>::CommitmentAlreadyUsed
		);
	});
}

#[test]
fn register_identity_requires_enrollment_origin() {
	new_test_ext().execute_with(|| {
		// A random signer cannot enroll.
		assert_noop!(
			KollectyveId::register_identity(RuntimeOrigin::signed(99), 2, commitment(1)),
			DispatchError::BadOrigin
		);
		// Nor can the compliance key — enrollment is a separate origin.
		assert_noop!(
			KollectyveId::register_identity(RuntimeOrigin::signed(20), 2, commitment(1)),
			DispatchError::BadOrigin
		);
	});
}

// --- set_tier ---------------------------------------------------------------

#[test]
fn set_tier_by_compliance_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(KollectyveId::register_identity(RuntimeOrigin::root(), 1, commitment(1)));
		// Compliance account (account 20, stand-in for the 2-of-3 multisig) raises the tier.
		assert_ok!(KollectyveId::set_tier(RuntimeOrigin::signed(20), 0, KycTier::Accredited));
		assert_eq!(TierOf::<Test>::get(0), KycTier::Accredited);
		System::assert_last_event(Event::TierSet { id: 0, tier: KycTier::Accredited }.into());
	});
}

#[test]
fn set_tier_rejects_enrollment_key() {
	new_test_ext().execute_with(|| {
		assert_ok!(KollectyveId::register_identity(RuntimeOrigin::root(), 1, commitment(1)));
		assert_noop!(
			KollectyveId::set_tier(RuntimeOrigin::signed(10), 0, KycTier::Verified),
			DispatchError::BadOrigin
		);
	});
}

#[test]
fn set_tier_rejects_unknown_identity() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			KollectyveId::set_tier(RuntimeOrigin::signed(20), 99, KycTier::Basic),
			Error::<Test>::UnknownIdentity
		);
	});
}

// --- rotate_controller (controller-only) ------------------------------------

#[test]
fn rotate_controller_by_current_controller_works() {
	// Self-custody graduation: the controller themself rotates to a user-owned key.
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(KollectyveId::register_identity(RuntimeOrigin::root(), 1, commitment(1)));
		assert_ok!(KollectyveId::rotate_controller(RuntimeOrigin::signed(1), 0, 2));

		assert_eq!(ControllerOf::<Test>::get(0), Some(2));
		assert_eq!(IdentityOf::<Test>::get(2), Some(0));
		assert_eq!(IdentityOf::<Test>::get(1), None);
		System::assert_last_event(Event::ControllerRotated { id: 0, old: 1, new: 2 }.into());
	});
}

#[test]
fn rotate_controller_rejects_enrollment_key_and_root() {
	new_test_ext().execute_with(|| {
		assert_ok!(KollectyveId::register_identity(RuntimeOrigin::root(), 1, commitment(1)));
		// Enrollment key is a signer but not the controller.
		assert_noop!(
			KollectyveId::rotate_controller(RuntimeOrigin::signed(10), 0, 2),
			Error::<Test>::NotController
		);
		// Root is not a signed origin at all — rotation is controller-only.
		assert_noop!(
			KollectyveId::rotate_controller(RuntimeOrigin::root(), 0, 2),
			DispatchError::BadOrigin
		);
	});
}

#[test]
fn rotate_controller_rejects_non_controller() {
	new_test_ext().execute_with(|| {
		assert_ok!(KollectyveId::register_identity(RuntimeOrigin::root(), 1, commitment(1)));
		assert_noop!(
			KollectyveId::rotate_controller(RuntimeOrigin::signed(9), 0, 2),
			Error::<Test>::NotController
		);
	});
}

#[test]
fn rotate_controller_rejects_unknown_identity() {
	new_test_ext().execute_with(|| {
		// Account 1 has no identity, so `ensure_signed` passes but the id lookup fails.
		assert_noop!(
			KollectyveId::rotate_controller(RuntimeOrigin::signed(1), 99, 2),
			Error::<Test>::UnknownIdentity
		);
	});
}

#[test]
fn rotate_controller_rejects_controller_in_use() {
	new_test_ext().execute_with(|| {
		assert_ok!(KollectyveId::register_identity(RuntimeOrigin::root(), 1, commitment(1)));
		assert_ok!(KollectyveId::register_identity(RuntimeOrigin::root(), 2, commitment(2)));
		// Identity 0's controller (1) cannot rotate onto account 2, which already backs identity 1.
		assert_noop!(
			KollectyveId::rotate_controller(RuntimeOrigin::signed(1), 0, 2),
			Error::<Test>::ControllerInUse
		);
	});
}

// --- pre-rotation -----------------------------------------------------------

/// Identity 0 controlled by account 1, pre-committed to account 2 as its next controller.
fn pre_rotated() {
	assert_ok!(KollectyveId::register_identity(RuntimeOrigin::root(), 1, commitment(1)));
	assert_ok!(KollectyveId::commit_next_controller(
		RuntimeOrigin::signed(1),
		0,
		KollectyveId::controller_commitment(&2)
	));
}

#[test]
fn commit_next_controller_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		pre_rotated();
		let c = KollectyveId::controller_commitment(&2);
		assert_eq!(NextControllerCommitment::<Test>::get(0), Some(c));
		System::assert_last_event(Event::NextControllerCommitted { id: 0, commitment: c }.into());
	});
}

#[test]
fn commit_next_controller_is_controller_only() {
	new_test_ext().execute_with(|| {
		assert_ok!(KollectyveId::register_identity(RuntimeOrigin::root(), 1, commitment(1)));
		let c = KollectyveId::controller_commitment(&2);
		assert_noop!(
			KollectyveId::commit_next_controller(RuntimeOrigin::signed(10), 0, c),
			Error::<Test>::NotController
		);
		assert_noop!(
			KollectyveId::commit_next_controller(RuntimeOrigin::root(), 0, c),
			DispatchError::BadOrigin
		);
		assert_noop!(
			KollectyveId::commit_next_controller(RuntimeOrigin::signed(1), 99, c),
			Error::<Test>::UnknownIdentity
		);
	});
}

#[test]
fn stolen_current_key_cannot_replace_commitment_or_rotate() {
	new_test_ext().execute_with(|| {
		pre_rotated();
		// Whoever holds key 1 can no longer swap in their own successor...
		assert_noop!(
			KollectyveId::commit_next_controller(
				RuntimeOrigin::signed(1),
				0,
				KollectyveId::controller_commitment(&3)
			),
			Error::<Test>::AlreadyCommitted
		);
		// ...nor move control directly.
		assert_noop!(
			KollectyveId::rotate_controller(RuntimeOrigin::signed(1), 0, 3),
			Error::<Test>::PreRotationRequired
		);
		assert_eq!(ControllerOf::<Test>::get(0), Some(1));
	});
}

#[test]
fn rotate_to_committed_by_next_key_works_and_chains() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		pre_rotated();
		let next = KollectyveId::controller_commitment(&3);
		assert_ok!(KollectyveId::rotate_to_committed(RuntimeOrigin::signed(2), 0, Some(next)));

		assert_eq!(ControllerOf::<Test>::get(0), Some(2));
		assert_eq!(IdentityOf::<Test>::get(2), Some(0));
		assert!(IdentityOf::<Test>::get(1).is_none());
		assert_eq!(NextControllerCommitment::<Test>::get(0), Some(next));
		System::assert_has_event(Event::ControllerRotated { id: 0, old: 1, new: 2 }.into());

		// The chain continues: only key 3 can take it next.
		assert_noop!(
			KollectyveId::rotate_to_committed(RuntimeOrigin::signed(2), 0, None),
			Error::<Test>::CommitmentMismatch
		);
		assert_ok!(KollectyveId::rotate_to_committed(RuntimeOrigin::signed(3), 0, None));
		assert_eq!(ControllerOf::<Test>::get(0), Some(3));
	});
}

#[test]
fn rotate_to_committed_rejects_wrong_signer() {
	new_test_ext().execute_with(|| {
		pre_rotated();
		for who in [1u64, 3, 10, 20] {
			assert_noop!(
				KollectyveId::rotate_to_committed(RuntimeOrigin::signed(who), 0, None),
				Error::<Test>::CommitmentMismatch
			);
		}
		assert_noop!(
			KollectyveId::rotate_to_committed(RuntimeOrigin::root(), 0, None),
			DispatchError::BadOrigin
		);
	});
}

#[test]
fn rotate_to_committed_requires_commitment() {
	new_test_ext().execute_with(|| {
		assert_ok!(KollectyveId::register_identity(RuntimeOrigin::root(), 1, commitment(1)));
		assert_noop!(
			KollectyveId::rotate_to_committed(RuntimeOrigin::signed(2), 0, None),
			Error::<Test>::NoCommitment
		);
		assert_noop!(
			KollectyveId::rotate_to_committed(RuntimeOrigin::signed(2), 99, None),
			Error::<Test>::UnknownIdentity
		);
	});
}

#[test]
fn rotate_to_committed_rejects_key_in_use() {
	new_test_ext().execute_with(|| {
		pre_rotated();
		// The committed key 2 meanwhile backs another identity.
		assert_ok!(KollectyveId::register_identity(RuntimeOrigin::root(), 2, commitment(2)));
		assert_noop!(
			KollectyveId::rotate_to_committed(RuntimeOrigin::signed(2), 0, None),
			Error::<Test>::ControllerInUse
		);
	});
}

#[test]
fn ending_pre_rotation_restores_plain_rotation() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		pre_rotated();
		assert_ok!(KollectyveId::rotate_to_committed(RuntimeOrigin::signed(2), 0, None));
		assert!(NextControllerCommitment::<Test>::get(0).is_none());
		System::assert_last_event(Event::PreRotationEnded { id: 0 }.into());
		assert_ok!(KollectyveId::rotate_controller(RuntimeOrigin::signed(2), 0, 4));
		assert_eq!(ControllerOf::<Test>::get(0), Some(4));
	});
}

// --- IdentityLookup read interface -----------------------------------------

#[test]
fn identity_lookup_trait_reads() {
	new_test_ext().execute_with(|| {
		assert_ok!(KollectyveId::register_identity(RuntimeOrigin::root(), 1, commitment(1)));
		assert_ok!(KollectyveId::set_tier(RuntimeOrigin::root(), 0, KycTier::Verified));

		assert_eq!(<KollectyveId as IdentityLookup<u64>>::identity_of(&1), Some(0));
		assert_eq!(<KollectyveId as IdentityLookup<u64>>::controller_of(0), Some(1));
		assert_eq!(<KollectyveId as IdentityLookup<u64>>::tier(0), KycTier::Verified);
		assert!(<KollectyveId as IdentityLookup<u64>>::is_active(0));

		// Unknown identity / account return defaults / None.
		assert_eq!(<KollectyveId as IdentityLookup<u64>>::identity_of(&99), None);
		assert_eq!(<KollectyveId as IdentityLookup<u64>>::controller_of(99), None);
		assert_eq!(<KollectyveId as IdentityLookup<u64>>::tier(99), KycTier::None);
		assert!(!<KollectyveId as IdentityLookup<u64>>::is_active(99));
	});
}
