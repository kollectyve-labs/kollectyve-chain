//! Benchmarks for `pallet-kumulus`.

use super::*;
use frame::deps::frame_benchmarking::v2::*;
use frame::deps::frame_system::RawOrigin;
use kollectyve_primitives::IdentityProvisioner;

fn specs() -> ProviderSpecs {
	ProviderSpecs { cpu_cores: 64, memory_mb: 262_144, storage_gb: 8_192, region: 250 }
}

#[benchmarks]
mod benchmarks {
	use super::*;

	/// Worst case is the update branch: a record already exists, so the call pays for the
	/// `contains_key` read on top of the insert.
	#[benchmark]
	fn register_provider() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = whitelisted_caller();
		let id = T::BenchmarkHelper::provision(caller.clone());
		Providers::<T>::insert(id, ProviderRecord { specs: specs(), active: false });

		#[extrinsic_call]
		_(RawOrigin::Signed(caller), specs());

		let record = Providers::<T>::get(id).expect("record was written");
		assert_eq!(record.specs, specs());
		assert!(record.active);
		Ok(())
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
