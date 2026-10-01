//! Benchmarks for `pallet-kollectyve-credentials`.

use super::*;
use frame::deps::frame_benchmarking::v2::*;
use frame::deps::frame_system::RawOrigin;
use frame::deps::sp_core::H256;
use frame::prelude::EnsureOrigin;
use kollectyve_primitives::IdentityProvisioner;

/// A live identity controlled by a fresh account per seed.
fn identity<T: Config>(seed: u32) -> (T::AccountId, kollectyve_primitives::IdentityId) {
	let who: T::AccountId = account("controller", seed, 0);
	let id = T::BenchmarkHelper::provision(who.clone());
	(who, id)
}

fn record(issuer: u64, subject: u64, schema: H256) -> Credential {
	Credential {
		issuer,
		subject,
		schema,
		authorized_by: None,
		issued_at: 0,
		expires_at: Some(u64::MAX),
		revoked_at: None,
	}
}

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn register_schema() {
		let (who, id) = identity::<T>(0);
		let schema = H256::repeat_byte(1);

		#[extrinsic_call]
		_(RawOrigin::Signed(who), schema);

		assert_eq!(Schemas::<T>::get(schema), Some(id));
	}

	// Worst case: the issuance follows an authorising credential and carries an expiry.
	#[benchmark]
	fn issue() {
		let (_, anchor) = identity::<T>(0);
		let (issuer_acc, issuer) = identity::<T>(1);
		let (_, subject) = identity::<T>(2);
		let schema = H256::repeat_byte(1);
		Schemas::<T>::insert(schema, issuer);
		let parent = H256::repeat_byte(2);
		Credentials::<T>::insert(parent, record(anchor, issuer, schema));
		let digest = H256::repeat_byte(3);

		#[extrinsic_call]
		_(RawOrigin::Signed(issuer_acc), subject, schema, digest, Some(parent), Some(u64::MAX));

		assert_eq!(Credentials::<T>::get(digest).map(|c| c.authorized_by), Some(Some(parent)));
	}

	#[benchmark]
	fn revoke() {
		let (issuer_acc, issuer) = identity::<T>(0);
		let digest = H256::repeat_byte(3);
		Credentials::<T>::insert(digest, record(issuer, issuer, H256::repeat_byte(1)));

		#[extrinsic_call]
		_(RawOrigin::Signed(issuer_acc), digest);

		assert!(Credentials::<T>::get(digest).and_then(|c| c.revoked_at).is_some());
	}

	#[benchmark]
	fn set_trust_anchor() -> Result<(), BenchmarkError> {
		let (_, id) = identity::<T>(0);
		let origin =
			T::AnchorOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, id, true);

		assert!(TrustAnchors::<T>::contains_key(id));
		Ok(())
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
