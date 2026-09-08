//! Benchmarks for `pallet-kollectyve-id`.

use super::*;
use frame::deps::frame_benchmarking::v2::*;
use frame::deps::frame_system::RawOrigin;
use frame::deps::sp_core::H256;
use frame::prelude::EnsureOrigin;
use kollectyve_primitives::KycTier;

/// A distinct controller account per seed, so no benchmark reuses an already-bound account.
fn controller<T: Config>(seed: u32) -> T::AccountId {
	account("controller", seed, 0)
}

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn register_identity() -> Result<(), BenchmarkError> {
		let origin = T::EnrollmentOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;
		let who = controller::<T>(0);
		let commitment = H256::repeat_byte(1);

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, who.clone(), commitment);

		let id = IdentityOf::<T>::get(&who).expect("identity was registered");
		assert_eq!(ControllerOf::<T>::get(id), Some(who));
		assert_eq!(UsedCommitments::<T>::get(commitment), Some(id));
		assert_eq!(TierOf::<T>::get(id), KycTier::None);
		Ok(())
	}

	#[benchmark]
	fn set_tier() -> Result<(), BenchmarkError> {
		let enrollment = T::EnrollmentOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;
		let who = controller::<T>(0);
		Pallet::<T>::register_identity(enrollment, who.clone(), H256::repeat_byte(1))?;
		let id = IdentityOf::<T>::get(&who).expect("identity was registered");

		let origin = T::ComplianceOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, id, KycTier::Accredited);

		assert_eq!(TierOf::<T>::get(id), KycTier::Accredited);
		Ok(())
	}

	#[benchmark]
	fn rotate_controller() -> Result<(), BenchmarkError> {
		let enrollment = T::EnrollmentOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;
		let old = controller::<T>(0);
		Pallet::<T>::register_identity(enrollment, old.clone(), H256::repeat_byte(1))?;
		let id = IdentityOf::<T>::get(&old).expect("identity was registered");
		let new = controller::<T>(1);

		#[extrinsic_call]
		_(RawOrigin::Signed(old.clone()), id, new.clone());

		assert_eq!(ControllerOf::<T>::get(id), Some(new.clone()));
		assert_eq!(IdentityOf::<T>::get(&new), Some(id));
		assert!(IdentityOf::<T>::get(&old).is_none());
		Ok(())
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
