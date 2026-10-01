use crate::{mock::*, Credentials as CredentialsStore, Error, Event, Schemas, TrustAnchors};
use frame::deps::frame_support::{assert_noop, assert_ok};
use frame::deps::sp_core::H256;
use frame::deps::sp_runtime::DispatchError;

// Accounts 1..=3 control identities 0..=2: a ministry, a university, a graduate. Account 9 has
// no identity.
const MINISTRY: u64 = 1;
const UNIVERSITY: u64 = 2;
const GRADUATE: u64 = 3;
const MINISTRY_ID: u64 = 0;
const UNIVERSITY_ID: u64 = 1;
const GRADUATE_ID: u64 = 2;

fn h(n: u8) -> H256 {
	H256::repeat_byte(n)
}

const ACCREDITATION: u8 = 0xa0;
const DIPLOMA: u8 = 0xd0;

fn setup() {
	System::set_block_number(1);
	Timestamp::set_timestamp(1_000);
	for (who, c) in [(MINISTRY, 1u8), (UNIVERSITY, 2), (GRADUATE, 3)] {
		assert_ok!(KollectyveId::register_identity(RuntimeOrigin::root(), who, h(c)));
	}
	assert_ok!(Credentials::register_schema(RuntimeOrigin::signed(MINISTRY), h(ACCREDITATION)));
	assert_ok!(Credentials::register_schema(RuntimeOrigin::signed(UNIVERSITY), h(DIPLOMA)));
}

/// Ministry accredits the university (digest 0x11), university issues a diploma (0x22) under it.
fn accredit_and_graduate() {
	assert_ok!(Credentials::issue(
		RuntimeOrigin::signed(MINISTRY),
		UNIVERSITY_ID,
		h(ACCREDITATION),
		h(0x11),
		None,
		None
	));
	assert_ok!(Credentials::issue(
		RuntimeOrigin::signed(UNIVERSITY),
		GRADUATE_ID,
		h(DIPLOMA),
		h(0x22),
		Some(h(0x11)),
		None
	));
}

// --- schemas -----------------------------------------------------------------

#[test]
fn register_schema_works_and_rejects_duplicates() {
	new_test_ext().execute_with(|| {
		setup();
		assert_eq!(Schemas::<Test>::get(h(DIPLOMA)), Some(UNIVERSITY_ID));
		assert_noop!(
			Credentials::register_schema(RuntimeOrigin::signed(MINISTRY), h(DIPLOMA)),
			Error::<Test>::SchemaExists
		);
	});
}

#[test]
fn calls_require_an_identity() {
	new_test_ext().execute_with(|| {
		setup();
		assert_noop!(
			Credentials::register_schema(RuntimeOrigin::signed(9), h(0x99)),
			Error::<Test>::NoIdentity
		);
		assert_noop!(
			Credentials::issue(RuntimeOrigin::signed(9), GRADUATE_ID, h(DIPLOMA), h(1), None, None),
			Error::<Test>::NoIdentity
		);
		assert_noop!(
			Credentials::register_schema(RuntimeOrigin::root(), h(0x99)),
			DispatchError::BadOrigin
		);
	});
}

// --- issue -------------------------------------------------------------------

#[test]
fn issue_records_the_credential() {
	new_test_ext().execute_with(|| {
		setup();
		accredit_and_graduate();
		let c = CredentialsStore::<Test>::get(h(0x22)).expect("issued");
		assert_eq!(c.issuer, UNIVERSITY_ID);
		assert_eq!(c.subject, GRADUATE_ID);
		assert_eq!(c.schema, h(DIPLOMA));
		assert_eq!(c.authorized_by, Some(h(0x11)));
		assert_eq!(c.issued_at, 1_000);
		assert_eq!(c.revoked_at, None);
		assert!(Credentials::is_valid(h(0x22)));
		System::assert_last_event(
			Event::CredentialIssued {
				digest: h(0x22),
				issuer: UNIVERSITY_ID,
				subject: GRADUATE_ID,
				schema: h(DIPLOMA),
				authorized_by: Some(h(0x11)),
			}
			.into(),
		);
	});
}

#[test]
fn issue_rejects_bad_inputs() {
	new_test_ext().execute_with(|| {
		setup();
		let u = RuntimeOrigin::signed(UNIVERSITY);
		assert_noop!(
			Credentials::issue(u.clone(), 99, h(DIPLOMA), h(1), None, None),
			Error::<Test>::UnknownIdentity
		);
		assert_noop!(
			Credentials::issue(u.clone(), GRADUATE_ID, h(0x77), h(1), None, None),
			Error::<Test>::UnknownSchema
		);
		assert_noop!(
			Credentials::issue(u.clone(), GRADUATE_ID, h(DIPLOMA), h(1), None, Some(1_000)),
			Error::<Test>::AlreadyExpired
		);
		assert_ok!(Credentials::issue(u.clone(), GRADUATE_ID, h(DIPLOMA), h(1), None, None));
		assert_noop!(
			Credentials::issue(u, GRADUATE_ID, h(DIPLOMA), h(1), None, None),
			Error::<Test>::AlreadyIssued
		);
	});
}

// --- chaining (ACDC-style edges) ---------------------------------------------

#[test]
fn authorization_must_exist_and_be_held_by_issuer() {
	new_test_ext().execute_with(|| {
		setup();
		accredit_and_graduate();
		assert_noop!(
			Credentials::issue(
				RuntimeOrigin::signed(UNIVERSITY),
				GRADUATE_ID,
				h(DIPLOMA),
				h(0x33),
				Some(h(0x55)),
				None
			),
			Error::<Test>::UnknownAuthorization
		);
		// The graduate cannot borrow the university's accreditation.
		assert_noop!(
			Credentials::issue(
				RuntimeOrigin::signed(GRADUATE),
				GRADUATE_ID,
				h(DIPLOMA),
				h(0x33),
				Some(h(0x11)),
				None
			),
			Error::<Test>::AuthorizationNotHeld
		);
	});
}

#[test]
fn revoked_or_expired_authorization_blocks_new_issuance() {
	new_test_ext().execute_with(|| {
		setup();
		assert_ok!(Credentials::issue(
			RuntimeOrigin::signed(MINISTRY),
			UNIVERSITY_ID,
			h(ACCREDITATION),
			h(0x11),
			None,
			Some(5_000)
		));
		Timestamp::set_timestamp(6_000);
		assert_noop!(
			Credentials::issue(
				RuntimeOrigin::signed(UNIVERSITY),
				GRADUATE_ID,
				h(DIPLOMA),
				h(0x22),
				Some(h(0x11)),
				None
			),
			Error::<Test>::AuthorizationExpired
		);

		assert_ok!(Credentials::issue(
			RuntimeOrigin::signed(MINISTRY),
			UNIVERSITY_ID,
			h(ACCREDITATION),
			h(0x12),
			None,
			None
		));
		assert_ok!(Credentials::revoke(RuntimeOrigin::signed(MINISTRY), h(0x12)));
		assert_noop!(
			Credentials::issue(
				RuntimeOrigin::signed(UNIVERSITY),
				GRADUATE_ID,
				h(DIPLOMA),
				h(0x22),
				Some(h(0x12)),
				None
			),
			Error::<Test>::AuthorizationRevoked
		);
	});
}

// --- revoke ------------------------------------------------------------------

#[test]
fn revoke_is_issuer_only_and_keeps_the_record() {
	new_test_ext().execute_with(|| {
		setup();
		accredit_and_graduate();
		// Neither the subject nor the accrediting ministry can revoke the diploma.
		for who in [GRADUATE, MINISTRY] {
			assert_noop!(
				Credentials::revoke(RuntimeOrigin::signed(who), h(0x22)),
				Error::<Test>::NotIssuer
			);
		}
		Timestamp::set_timestamp(2_000);
		assert_ok!(Credentials::revoke(RuntimeOrigin::signed(UNIVERSITY), h(0x22)));
		assert_eq!(CredentialsStore::<Test>::get(h(0x22)).unwrap().revoked_at, Some(2_000));
		assert!(!Credentials::is_valid(h(0x22)));
		System::assert_last_event(
			Event::CredentialRevoked { digest: h(0x22), issuer: UNIVERSITY_ID }.into(),
		);
		assert_noop!(
			Credentials::revoke(RuntimeOrigin::signed(UNIVERSITY), h(0x22)),
			Error::<Test>::AlreadyRevoked
		);
		assert_noop!(
			Credentials::revoke(RuntimeOrigin::signed(UNIVERSITY), h(0x99)),
			Error::<Test>::UnknownCredential
		);
	});
}

#[test]
fn is_valid_tracks_expiry() {
	new_test_ext().execute_with(|| {
		setup();
		assert_ok!(Credentials::issue(
			RuntimeOrigin::signed(UNIVERSITY),
			GRADUATE_ID,
			h(DIPLOMA),
			h(0x22),
			None,
			Some(5_000)
		));
		assert!(Credentials::is_valid(h(0x22)));
		Timestamp::set_timestamp(5_001);
		assert!(!Credentials::is_valid(h(0x22)));
		assert!(!Credentials::is_valid(h(0x99)));
	});
}

// --- identity, not key -------------------------------------------------------

#[test]
fn issuer_key_rotation_keeps_the_right_to_revoke() {
	new_test_ext().execute_with(|| {
		setup();
		accredit_and_graduate();
		// The university rotates its controller from account 2 to account 20.
		assert_ok!(KollectyveId::rotate_controller(
			RuntimeOrigin::signed(UNIVERSITY),
			UNIVERSITY_ID,
			20
		));
		assert_noop!(
			Credentials::revoke(RuntimeOrigin::signed(UNIVERSITY), h(0x22)),
			Error::<Test>::NoIdentity
		);
		assert_ok!(Credentials::revoke(RuntimeOrigin::signed(20), h(0x22)));
	});
}

// --- trust anchors -----------------------------------------------------------

#[test]
fn trust_anchors_are_governance_only() {
	new_test_ext().execute_with(|| {
		setup();
		assert_noop!(
			Credentials::set_trust_anchor(RuntimeOrigin::signed(MINISTRY), MINISTRY_ID, true),
			DispatchError::BadOrigin
		);
		assert_noop!(
			Credentials::set_trust_anchor(RuntimeOrigin::root(), 99, true),
			Error::<Test>::UnknownIdentity
		);
		assert_ok!(Credentials::set_trust_anchor(RuntimeOrigin::root(), MINISTRY_ID, true));
		assert!(TrustAnchors::<Test>::contains_key(MINISTRY_ID));
		System::assert_last_event(Event::TrustAnchorSet { id: MINISTRY_ID, anchored: true }.into());
		assert_ok!(Credentials::set_trust_anchor(RuntimeOrigin::root(), MINISTRY_ID, false));
		assert!(!TrustAnchors::<Test>::contains_key(MINISTRY_ID));
	});
}
