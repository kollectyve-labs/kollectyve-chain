//! # Pallet Kumulus — provider registry
//!
//! A provider logs in via the backend (custodial key), the backend submits `register_provider`, and
//! the record is stored keyed by the caller's stable [`IdentityId`] rather than the raw account.
//! There is no KYC gate: provider onboarding is coordination work.
//!
//! Loose coupling: depends only on the read-only [`kollectyve_primitives::IdentityLookup`] trait.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;
pub use weights::WeightInfo;

#[frame::pallet]
pub mod pallet {
	use super::WeightInfo;
	use frame::prelude::*;
	use kollectyve_primitives::{IdentityId, IdentityLookup};

	/// Hardware a provider declares when registering.
	#[derive(
		Clone,
		Encode,
		Decode,
		DecodeWithMemTracking,
		Eq,
		PartialEq,
		DebugNoBound,
		TypeInfo,
		MaxEncodedLen,
	)]
	pub struct ProviderSpecs {
		pub cpu_cores: u32,
		pub memory_mb: u32,
		pub storage_gb: u32,
		/// ISO 3166-1 numeric region code; 0 = unspecified.
		pub region: u16,
	}

	/// Stored provider record, keyed by the owner's identity.
	#[derive(
		Clone,
		Encode,
		Decode,
		DecodeWithMemTracking,
		Eq,
		PartialEq,
		DebugNoBound,
		TypeInfo,
		MaxEncodedLen,
	)]
	pub struct ProviderRecord {
		pub specs: ProviderSpecs,
		pub active: bool,
	}

	#[pallet::config]
	pub trait Config: frame::deps::frame_system::Config {
		/// The overarching runtime event type.
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as frame::deps::frame_system::Config>::RuntimeEvent>;
		/// Read-only identity resolution. Wired to `pallet-kollectyve-id` in the runtime.
		type Identity: IdentityLookup<Self::AccountId>;
		/// Weights for this pallet's dispatchables.
		type WeightInfo: WeightInfo;
		/// Gives the benchmark caller an identity; benchmarking only.
		#[cfg(feature = "runtime-benchmarks")]
		type BenchmarkHelper: kollectyve_primitives::IdentityProvisioner<Self::AccountId>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// `IdentityId -> provider record`.
	#[pallet::storage]
	pub type Providers<T: Config> =
		StorageMap<_, Blake2_128Concat, IdentityId, ProviderRecord, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		ProviderRegistered { id: IdentityId },
		ProviderUpdated { id: IdentityId },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The caller has no on-chain identity.
		NoIdentity,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Register (or overwrite) the caller's provider record, keyed by their identity.
		///
		/// The caller is a custodial account whose identity was minted by the backend registrar;
		/// we resolve it through the identity anchor rather than storing the raw account.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::register_provider())]
		pub fn register_provider(origin: OriginFor<T>, specs: ProviderSpecs) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let id = T::Identity::identity_of(&who).ok_or(Error::<T>::NoIdentity)?;
			let existed = Providers::<T>::contains_key(id);
			Providers::<T>::insert(id, ProviderRecord { specs, active: true });
			if existed {
				Self::deposit_event(Event::ProviderUpdated { id });
			} else {
				Self::deposit_event(Event::ProviderRegistered { id });
			}
			Ok(())
		}
	}
}
