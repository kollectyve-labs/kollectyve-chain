//! Weights for `pallet_kollectyve_credentials`.
//!
//! PLACEHOLDER — hand-estimated from the storage each call touches, NOT measured. Regenerate with
//! the benchmark CLI (see the repo README, "Benchmarks & weights"), on a cold CPU, before this
//! pallet reaches a live chain.

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame::deps::frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use core::marker::PhantomData;

/// Weight functions needed for `pallet_kollectyve_credentials`.
pub trait WeightInfo {
	fn register_schema() -> Weight;
	fn issue() -> Weight;
	fn revoke() -> Weight;
	fn set_trust_anchor() -> Weight;
}

/// Weights for `pallet_kollectyve_credentials` on the reference hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame::deps::frame_system::Config> WeightInfo for SubstrateWeight<T> {
	/// Storage: `KollectyveId::IdentityOf` (r:1), `Credentials::Schemas` (r:1 w:1)
	fn register_schema() -> Weight {
		Weight::from_parts(25_000_000, 0)
			.saturating_add(Weight::from_parts(0, 3600))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `KollectyveId::IdentityOf` (r:1), `KollectyveId::ControllerOf` (r:1),
	/// `Credentials::Schemas` (r:1), `Credentials::Credentials` (r:2 w:1), `Timestamp::Now` (r:1)
	fn issue() -> Weight {
		Weight::from_parts(50_000_000, 0)
			.saturating_add(Weight::from_parts(0, 3600))
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `KollectyveId::IdentityOf` (r:1), `Credentials::Credentials` (r:1 w:1),
	/// `Timestamp::Now` (r:1)
	fn revoke() -> Weight {
		Weight::from_parts(30_000_000, 0)
			.saturating_add(Weight::from_parts(0, 3600))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `KollectyveId::ControllerOf` (r:1), `Credentials::TrustAnchors` (w:1)
	fn set_trust_anchor() -> Weight {
		Weight::from_parts(20_000_000, 0)
			.saturating_add(Weight::from_parts(0, 3600))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	/// Storage: `KollectyveId::IdentityOf` (r:1), `Credentials::Schemas` (r:1 w:1)
	fn register_schema() -> Weight {
		Weight::from_parts(25_000_000, 0)
			.saturating_add(Weight::from_parts(0, 3600))
			.saturating_add(RocksDbWeight::get().reads(2))
			.saturating_add(RocksDbWeight::get().writes(1))
	}
	/// Storage: `KollectyveId::IdentityOf` (r:1), `KollectyveId::ControllerOf` (r:1),
	/// `Credentials::Schemas` (r:1), `Credentials::Credentials` (r:2 w:1), `Timestamp::Now` (r:1)
	fn issue() -> Weight {
		Weight::from_parts(50_000_000, 0)
			.saturating_add(Weight::from_parts(0, 3600))
			.saturating_add(RocksDbWeight::get().reads(6))
			.saturating_add(RocksDbWeight::get().writes(1))
	}
	/// Storage: `KollectyveId::IdentityOf` (r:1), `Credentials::Credentials` (r:1 w:1),
	/// `Timestamp::Now` (r:1)
	fn revoke() -> Weight {
		Weight::from_parts(30_000_000, 0)
			.saturating_add(Weight::from_parts(0, 3600))
			.saturating_add(RocksDbWeight::get().reads(3))
			.saturating_add(RocksDbWeight::get().writes(1))
	}
	/// Storage: `KollectyveId::ControllerOf` (r:1), `Credentials::TrustAnchors` (w:1)
	fn set_trust_anchor() -> Weight {
		Weight::from_parts(20_000_000, 0)
			.saturating_add(Weight::from_parts(0, 3600))
			.saturating_add(RocksDbWeight::get().reads(1))
			.saturating_add(RocksDbWeight::get().writes(1))
	}
}
