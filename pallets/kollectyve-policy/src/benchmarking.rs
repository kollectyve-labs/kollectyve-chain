//! Benchmarks for `pallet-kollectyve-policy`.

use super::*;
use frame::deps::frame_benchmarking::v2::*;
use frame::prelude::{BoundedVec, ConstU32, EnsureOrigin};
use kollectyve_primitives::{IdentityProvisioner, KycTier};

/// A jurisdiction list filled to its bound.
fn full_list(offset: u16) -> BoundedVec<u16, ConstU32<64>> {
	let mut list = BoundedVec::default();
	for i in 0..64u16 {
		list.try_push(i + offset).expect("64 entries fit the ConstU32<64> bound");
	}
	list
}

/// Worst case: both jurisdiction lists filled to their bound, so the stored value is as large as
/// the type allows.
fn worst_case_policy() -> AssetPolicy {
	AssetPolicy {
		min_tier: KycTier::Accredited,
		require_accredited: true,
		allowed_jurisdictions: full_list(0),
		blocked_jurisdictions: full_list(1_000),
		frozen: false,
	}
}

fn worst_case_claim() -> KycClaim {
	KycClaim {
		tier: KycTier::Accredited,
		jurisdiction: 250,
		accredited: true,
		// `Some` is the worst case: `eligible` then has to read the timestamp.
		expires_at: Some(u64::MAX),
	}
}

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn set_claim() -> Result<(), BenchmarkError> {
		let id = T::BenchmarkHelper::provision(account("controller", 0, 0));
		let origin = T::ComplianceOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;
		let claim = worst_case_claim();

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, id, claim.clone());

		assert_eq!(Claims::<T>::get(id), Some(claim));
		Ok(())
	}

	#[benchmark]
	fn revoke_claim() -> Result<(), BenchmarkError> {
		let id = T::BenchmarkHelper::provision(account("controller", 0, 0));
		Claims::<T>::insert(id, worst_case_claim());
		let origin = T::ComplianceOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, id);

		assert!(Claims::<T>::get(id).is_none());
		Ok(())
	}

	#[benchmark]
	fn set_asset_policy() -> Result<(), BenchmarkError> {
		let asset = T::AssetId::default();
		let origin = T::ComplianceOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;
		let policy = worst_case_policy();

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, asset, policy.clone());

		assert_eq!(AssetPolicies::<T>::get(asset), Some(policy));
		Ok(())
	}

	#[benchmark]
	fn set_frozen() -> Result<(), BenchmarkError> {
		let asset = T::AssetId::default();
		AssetPolicies::<T>::insert(asset, worst_case_policy());
		let origin = T::ComplianceOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, asset, true);

		assert!(AssetPolicies::<T>::get(asset).expect("policy is set").frozen);
		Ok(())
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
