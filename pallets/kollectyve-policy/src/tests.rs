use crate::{mock::*, AssetPolicy, Error, Event, KycClaim};
use frame::deps::frame_support::{assert_noop, assert_ok, BoundedVec};
use frame::deps::sp_core::H256;
use frame::deps::sp_runtime::DispatchError;
use kollectyve_primitives::{HoldingPolicy, KycTier};
use polkadot_sdk::pallet_timestamp;

const ASSET: u32 = 42;

/// Register an identity via the anchor and return its id (ids are allocated 0, 1, 2, …).
fn new_identity(controller: u64, commitment: u8) -> u64 {
	let id = pallet_kollectyve_id::NextIdentityId::<Test>::get();
	assert_ok!(KollectyveId::register_identity(
		RuntimeOrigin::root(),
		controller,
		H256::repeat_byte(commitment)
	));
	id
}

fn claim(tier: KycTier, jurisdiction: u16, accredited: bool, expires_at: Option<u64>) -> KycClaim {
	KycClaim { tier, jurisdiction, accredited, expires_at }
}

fn policy(
	min_tier: KycTier,
	require_accredited: bool,
	allowed: &[u16],
	blocked: &[u16],
) -> AssetPolicy {
	AssetPolicy {
		min_tier,
		require_accredited,
		allowed_jurisdictions: BoundedVec::try_from(allowed.to_vec()).unwrap(),
		blocked_jurisdictions: BoundedVec::try_from(blocked.to_vec()).unwrap(),
		frozen: false,
	}
}

fn set_ts(ms: u64) {
	pallet_timestamp::Pallet::<Test>::set_timestamp(ms);
}

// --- set_claim / set_asset_policy happy paths + origin checks ----------------

#[test]
fn set_claim_and_policy_work() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let id = new_identity(1, 1);
		assert_ok!(KollectyvePolicy::set_claim(
			RuntimeOrigin::root(),
			id,
			claim(KycTier::Verified, 100, false, None)
		));
		System::assert_last_event(Event::ClaimSet { id, tier: KycTier::Verified }.into());

		assert_ok!(KollectyvePolicy::set_asset_policy(
			RuntimeOrigin::root(),
			ASSET,
			policy(KycTier::Verified, false, &[], &[])
		));
		System::assert_last_event(Event::AssetPolicySet { asset: ASSET }.into());
	});
}

#[test]
fn set_claim_rejects_unknown_identity() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			KollectyvePolicy::set_claim(
				RuntimeOrigin::root(),
				99,
				claim(KycTier::Verified, 100, false, None)
			),
			Error::<Test>::UnknownIdentity
		);
	});
}

#[test]
fn compliance_calls_require_compliance_origin() {
	new_test_ext().execute_with(|| {
		let id = new_identity(1, 1);
		assert_noop!(
			KollectyvePolicy::set_claim(
				RuntimeOrigin::signed(5),
				id,
				claim(KycTier::Verified, 100, false, None)
			),
			DispatchError::BadOrigin
		);
		assert_noop!(
			KollectyvePolicy::set_asset_policy(
				RuntimeOrigin::signed(5),
				ASSET,
				policy(KycTier::Verified, false, &[], &[])
			),
			DispatchError::BadOrigin
		);
	});
}

// --- can_hold / can_transfer eligibility ------------------------------------

#[test]
fn can_transfer_and_hold_succeed_for_eligible() {
	new_test_ext().execute_with(|| {
		let from = new_identity(1, 1);
		let to = new_identity(2, 2);
		assert_ok!(KollectyvePolicy::set_asset_policy(
			RuntimeOrigin::root(),
			ASSET,
			policy(KycTier::Verified, false, &[], &[])
		));
		let c = claim(KycTier::Verified, 100, false, None);
		assert_ok!(KollectyvePolicy::set_claim(RuntimeOrigin::root(), from, c.clone()));
		assert_ok!(KollectyvePolicy::set_claim(RuntimeOrigin::root(), to, c));

		assert_ok!(KollectyvePolicy::can_hold(from, &ASSET));
		assert_ok!(KollectyvePolicy::can_transfer(from, to, &ASSET));
	});
}

#[test]
fn unknown_asset_policy_fails() {
	new_test_ext().execute_with(|| {
		let id = new_identity(1, 1);
		assert_noop!(
			KollectyvePolicy::can_hold(id, &ASSET),
			Error::<Test>::UnknownAssetPolicy
		);
	});
}

#[test]
fn not_kyced_fails() {
	new_test_ext().execute_with(|| {
		let id = new_identity(1, 1);
		assert_ok!(KollectyvePolicy::set_asset_policy(
			RuntimeOrigin::root(),
			ASSET,
			policy(KycTier::Basic, false, &[], &[])
		));
		// Identity exists but has no claim.
		assert_noop!(KollectyvePolicy::can_hold(id, &ASSET), Error::<Test>::NotKyced);
	});
}

#[test]
fn sub_tier_fails() {
	new_test_ext().execute_with(|| {
		let id = new_identity(1, 1);
		assert_ok!(KollectyvePolicy::set_asset_policy(
			RuntimeOrigin::root(),
			ASSET,
			policy(KycTier::Verified, false, &[], &[])
		));
		assert_ok!(KollectyvePolicy::set_claim(
			RuntimeOrigin::root(),
			id,
			claim(KycTier::Basic, 100, false, None)
		));
		assert_noop!(KollectyvePolicy::can_hold(id, &ASSET), Error::<Test>::TierTooLow);
	});
}

#[test]
fn non_accredited_fails() {
	new_test_ext().execute_with(|| {
		let id = new_identity(1, 1);
		assert_ok!(KollectyvePolicy::set_asset_policy(
			RuntimeOrigin::root(),
			ASSET,
			policy(KycTier::Verified, true, &[], &[])
		));
		assert_ok!(KollectyvePolicy::set_claim(
			RuntimeOrigin::root(),
			id,
			claim(KycTier::Verified, 100, false, None)
		));
		assert_noop!(KollectyvePolicy::can_hold(id, &ASSET), Error::<Test>::NotAccredited);
	});
}

#[test]
fn blocked_jurisdiction_fails() {
	new_test_ext().execute_with(|| {
		let id = new_identity(1, 1);
		assert_ok!(KollectyvePolicy::set_asset_policy(
			RuntimeOrigin::root(),
			ASSET,
			policy(KycTier::Basic, false, &[], &[100])
		));
		assert_ok!(KollectyvePolicy::set_claim(
			RuntimeOrigin::root(),
			id,
			claim(KycTier::Verified, 100, false, None)
		));
		assert_noop!(
			KollectyvePolicy::can_hold(id, &ASSET),
			Error::<Test>::JurisdictionNotAllowed
		);
	});
}

#[test]
fn not_in_allowed_jurisdictions_fails() {
	new_test_ext().execute_with(|| {
		let id = new_identity(1, 1);
		assert_ok!(KollectyvePolicy::set_asset_policy(
			RuntimeOrigin::root(),
			ASSET,
			policy(KycTier::Basic, false, &[200], &[])
		));
		assert_ok!(KollectyvePolicy::set_claim(
			RuntimeOrigin::root(),
			id,
			claim(KycTier::Verified, 100, false, None)
		));
		assert_noop!(
			KollectyvePolicy::can_hold(id, &ASSET),
			Error::<Test>::JurisdictionNotAllowed
		);
	});
}

#[test]
fn expired_claim_fails_by_timestamp() {
	new_test_ext().execute_with(|| {
		let id = new_identity(1, 1);
		assert_ok!(KollectyvePolicy::set_asset_policy(
			RuntimeOrigin::root(),
			ASSET,
			policy(KycTier::Basic, false, &[], &[])
		));
		assert_ok!(KollectyvePolicy::set_claim(
			RuntimeOrigin::root(),
			id,
			claim(KycTier::Verified, 100, false, Some(500_000))
		));
		// Before expiry: fine.
		set_ts(400_000);
		assert_ok!(KollectyvePolicy::can_hold(id, &ASSET));
		// After expiry: rejected.
		set_ts(600_000);
		assert_noop!(KollectyvePolicy::can_hold(id, &ASSET), Error::<Test>::ClaimExpired);
	});
}

#[test]
fn frozen_asset_fails() {
	new_test_ext().execute_with(|| {
		let id = new_identity(1, 1);
		assert_ok!(KollectyvePolicy::set_asset_policy(
			RuntimeOrigin::root(),
			ASSET,
			policy(KycTier::Basic, false, &[], &[])
		));
		assert_ok!(KollectyvePolicy::set_claim(
			RuntimeOrigin::root(),
			id,
			claim(KycTier::Verified, 100, false, None)
		));
		assert_ok!(KollectyvePolicy::set_frozen(RuntimeOrigin::root(), ASSET, true));
		System::set_block_number(1);
		assert_noop!(KollectyvePolicy::can_hold(id, &ASSET), Error::<Test>::AssetFrozen);
	});
}

#[test]
fn can_transfer_checks_both_ends() {
	new_test_ext().execute_with(|| {
		let from = new_identity(1, 1);
		let to = new_identity(2, 2);
		assert_ok!(KollectyvePolicy::set_asset_policy(
			RuntimeOrigin::root(),
			ASSET,
			policy(KycTier::Verified, false, &[], &[])
		));
		// `from` is eligible, `to` is under-tier.
		assert_ok!(KollectyvePolicy::set_claim(
			RuntimeOrigin::root(),
			from,
			claim(KycTier::Verified, 100, false, None)
		));
		assert_ok!(KollectyvePolicy::set_claim(
			RuntimeOrigin::root(),
			to,
			claim(KycTier::Basic, 100, false, None)
		));
		assert_noop!(
			KollectyvePolicy::can_transfer(from, to, &ASSET),
			Error::<Test>::TierTooLow
		);
	});
}

#[test]
fn revoke_claim_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let id = new_identity(1, 1);
		assert_ok!(KollectyvePolicy::set_claim(
			RuntimeOrigin::root(),
			id,
			claim(KycTier::Verified, 100, false, None)
		));
		assert_ok!(KollectyvePolicy::revoke_claim(RuntimeOrigin::root(), id));
		System::assert_last_event(Event::ClaimRevoked { id }.into());
		assert!(crate::Claims::<Test>::get(id).is_none());
	});
}
