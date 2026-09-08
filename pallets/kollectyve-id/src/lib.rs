//! # Pallet Kollectyve ID — stable on-chain identity anchor
//!
//! The foundation of Kollectyve account abstraction. It mints a stable
//! [`IdentityId`](kollectyve_primitives::IdentityId) for each user/provider/service, binds it to a
//! *controller* account (a KMS-held custodial key in v1, a user-owned key after self-custody
//! graduation), and records a coarse KYC tier.
//!
//! ## Origins (two, deliberately split)
//! - **`EnrollmentOrigin`** may call ONLY [`register_identity`](pallet::Pallet::register_identity),
//!   which always creates a tier-[`None`](kollectyve_primitives::KycTier::None) identity. In the
//!   runtime this is a single hot KMS enrollment key (or root).
//! - **`ComplianceOrigin`** may call [`set_tier`](pallet::Pallet::set_tier). In the runtime this is
//!   a 2-of-3 multisig (or root) — never the hot enrollment key. This keeps "who can enroll a user"
//!   and "who can raise their KYC tier" on different blast radii.
//!
//! The runtime resolves both origins against the [`EnrollmentKey`](pallet::EnrollmentKey) /
//! [`ComplianceKey`](pallet::ComplianceKey) accounts set at **genesis** (via the chain spec), so
//! operator keys live in the chain spec, never in runtime code. When unset (dev default), each
//! origin resolves to root/sudo only.
//!
//! [`rotate_controller`](pallet::Pallet::rotate_controller) is **controller-only**: neither origin
//! can rotate a user's key. Registrar-assisted recovery is deferred.
//!
//! Shared types and the read trait live in `kollectyve-primitives`; other pallets depend on that
//! crate, never on this one.

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
	use frame::deps::sp_core::H256;
	use frame::prelude::*;
	use kollectyve_primitives::{IdentityId, IdentityLookup, KycTier};

	#[pallet::config]
	pub trait Config: frame::deps::frame_system::Config {
		/// The overarching runtime event type.
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as frame::deps::frame_system::Config>::RuntimeEvent>;
		/// Origin allowed to enroll new identities (hot KMS key / root). May ONLY create tier-`None`
		/// identities via `register_identity`.
		type EnrollmentOrigin: EnsureOrigin<
			<Self as frame::deps::frame_system::Config>::RuntimeOrigin,
		>;
		/// Origin allowed to change compliance state (`set_tier`). A multisig / root — never the
		/// enrollment key.
		type ComplianceOrigin: EnsureOrigin<
			<Self as frame::deps::frame_system::Config>::RuntimeOrigin,
		>;
		/// Weights for this pallet's dispatchables.
		type WeightInfo: WeightInfo;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Monotonic identity-id allocator.
	#[pallet::storage]
	pub type NextIdentityId<T> = StorageValue<_, IdentityId, ValueQuery>;

	/// `IdentityId -> controller account`. Absence means the identity does not exist.
	#[pallet::storage]
	pub type ControllerOf<T: Config> =
		StorageMap<_, Blake2_128Concat, IdentityId, T::AccountId, OptionQuery>;

	/// Reverse index: `controller account -> IdentityId`. One account controls at most one identity.
	#[pallet::storage]
	pub type IdentityOf<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, IdentityId, OptionQuery>;

	/// `IdentityId -> KYC tier`. Defaults to [`KycTier::None`].
	#[pallet::storage]
	pub type TierOf<T: Config> = StorageMap<_, Blake2_128Concat, IdentityId, KycTier, ValueQuery>;

	/// `user_commitment -> IdentityId`. A commitment is a backend-computed hash of the stable auth
	/// subject (e.g. `blake2(betterauth_user_id || domain_tag)`) — carries **no PII**. Enforces
	/// one-identity-per-auth-subject on-chain, so the chain backstops the backend's idempotency.
	#[pallet::storage]
	pub type UsedCommitments<T: Config> =
		StorageMap<_, Blake2_128Concat, H256, IdentityId, OptionQuery>;

	/// The single hot enrollment key, set at genesis. The runtime's `EnrollmentOrigin` reads this so
	/// the privileged signer is wired via the **chain spec** at deploy time — no key is ever
	/// hardcoded in runtime code. `None` (the dev default) means only root/sudo may enroll.
	#[pallet::storage]
	pub type EnrollmentKey<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

	/// The compliance key / multisig address, set at genesis. The runtime's `ComplianceOrigin` reads
	/// this. `None` (the dev default) means only root/sudo may set tiers.
	#[pallet::storage]
	pub type ComplianceKey<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

	/// Genesis wiring for the two privileged origins. Both default to `None` (dev: root/sudo only);
	/// a production chain spec sets the real enrollment key and compliance address here — keeping
	/// operator keys in the chain spec, never in runtime code.
	#[pallet::genesis_config]
	#[derive(DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		pub enrollment_key: Option<T::AccountId>,
		pub compliance_key: Option<T::AccountId>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			if let Some(key) = &self.enrollment_key {
				EnrollmentKey::<T>::put(key.clone());
			}
			if let Some(key) = &self.compliance_key {
				ComplianceKey::<T>::put(key.clone());
			}
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		IdentityRegistered { id: IdentityId, controller: T::AccountId, commitment: H256 },
		TierSet { id: IdentityId, tier: KycTier },
		ControllerRotated { id: IdentityId, old: T::AccountId, new: T::AccountId },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The controller account already backs an identity.
		AlreadyRegistered,
		/// No identity exists for the given id.
		UnknownIdentity,
		/// Caller is not the current controller of the identity.
		NotController,
		/// The proposed new controller already backs another identity.
		ControllerInUse,
		/// This `user_commitment` was already used to enroll an identity.
		CommitmentAlreadyUsed,
		/// The identity-id allocator overflowed.
		IdOverflow,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Enroll a custodial identity bound to a KMS-held account.
		///
		/// Enrollment-only origin. The new identity is always tier `None`; raising the tier is a
		/// separate `set_tier` call under the compliance origin. `user_commitment` is a PII-free
		/// hash of the auth subject and must be unique across all identities.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::register_identity())]
		pub fn register_identity(
			origin: OriginFor<T>,
			controller: T::AccountId,
			user_commitment: H256,
		) -> DispatchResult {
			T::EnrollmentOrigin::ensure_origin(origin)?;
			ensure!(!IdentityOf::<T>::contains_key(&controller), Error::<T>::AlreadyRegistered);
			ensure!(
				!UsedCommitments::<T>::contains_key(user_commitment),
				Error::<T>::CommitmentAlreadyUsed
			);
			let id = NextIdentityId::<T>::get();
			let next = id.checked_add(1).ok_or(Error::<T>::IdOverflow)?;
			NextIdentityId::<T>::put(next);
			ControllerOf::<T>::insert(id, controller.clone());
			IdentityOf::<T>::insert(&controller, id);
			// Tier defaults to `None` (ValueQuery); no explicit insert needed.
			UsedCommitments::<T>::insert(user_commitment, id);
			Self::deposit_event(Event::IdentityRegistered {
				id,
				controller,
				commitment: user_commitment,
			});
			Ok(())
		}

		/// Compliance origin updates the KYC tier of an existing identity.
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::set_tier())]
		pub fn set_tier(origin: OriginFor<T>, id: IdentityId, tier: KycTier) -> DispatchResult {
			T::ComplianceOrigin::ensure_origin(origin)?;
			ensure!(ControllerOf::<T>::contains_key(id), Error::<T>::UnknownIdentity);
			TierOf::<T>::insert(id, tier);
			Self::deposit_event(Event::TierSet { id, tier });
			Ok(())
		}

		/// Self-custody graduation / key change. **Controller-only** — only the current controller
		/// can rotate, to an as-yet-unused account. Neither origin can rotate on a user's behalf.
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::rotate_controller())]
		pub fn rotate_controller(
			origin: OriginFor<T>,
			id: IdentityId,
			new: T::AccountId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let old = ControllerOf::<T>::get(id).ok_or(Error::<T>::UnknownIdentity)?;
			ensure!(who == old, Error::<T>::NotController);
			ensure!(!IdentityOf::<T>::contains_key(&new), Error::<T>::ControllerInUse);
			IdentityOf::<T>::remove(&old);
			ControllerOf::<T>::insert(id, new.clone());
			IdentityOf::<T>::insert(&new, id);
			Self::deposit_event(Event::ControllerRotated { id, old, new });
			Ok(())
		}
	}

	impl<T: Config> IdentityLookup<T::AccountId> for Pallet<T> {
		fn identity_of(who: &T::AccountId) -> Option<IdentityId> {
			IdentityOf::<T>::get(who)
		}
		fn controller_of(id: IdentityId) -> Option<T::AccountId> {
			ControllerOf::<T>::get(id)
		}
		fn tier(id: IdentityId) -> KycTier {
			TierOf::<T>::get(id)
		}
		fn is_active(id: IdentityId) -> bool {
			ControllerOf::<T>::contains_key(id)
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl<T: Config> kollectyve_primitives::IdentityProvisioner<T::AccountId> for Pallet<T> {
		fn provision(controller: T::AccountId) -> IdentityId {
			let id = NextIdentityId::<T>::get();
			NextIdentityId::<T>::put(id + 1);
			ControllerOf::<T>::insert(id, controller.clone());
			IdentityOf::<T>::insert(&controller, id);
			id
		}
	}
}
