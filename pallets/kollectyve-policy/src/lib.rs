//! # Pallet Kollectyve Policy — regulated-asset compliance gate
//!
//! Holds, per identity, a KYC **claim** (tier, jurisdiction, accreditation, expiry — verdicts only,
//! **no raw PII**), and, per asset, a **policy** (minimum tier, accreditation requirement,
//! jurisdiction allow/block lists, freeze flag). It exposes
//! [`HoldingPolicy`](kollectyve_primitives::HoldingPolicy) so consumer pallets can ask `can_hold` /
//! `can_transfer` before moving a regulated position — compliance is enforced by the chain, not
//! merely promised by the backend.
//!
//! - All mutating extrinsics require **`ComplianceOrigin`** (a multisig / root in the runtime) —
//!   the hot enrollment key cannot touch compliance state.
//! - Claim expiry is a **unix-millis timestamp** checked against `pallet_timestamp`, not a block
//!   number, so expiries mean the same thing across runtime upgrades.
//! - [`set_claim`](pallet::Pallet::set_claim) rejects claims on non-existent identities via the
//!   [`IdentityLookup`](kollectyve_primitives::IdentityLookup) trait.

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
	use kollectyve_primitives::{HoldingPolicy, IdentityId, IdentityLookup, KycTier};
	use polkadot_sdk::pallet_timestamp;

	/// A per-identity KYC verdict. Carries no PII — only the coarse tier, a numeric jurisdiction,
	/// an accreditation flag, and an optional expiry.
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
	pub struct KycClaim {
		pub tier: KycTier,
		/// ISO 3166-1 numeric country code; 0 = unspecified.
		pub jurisdiction: u16,
		pub accredited: bool,
		/// Unix time in milliseconds after which the claim is stale. `None` = never expires.
		pub expires_at: Option<u64>,
	}

	/// Per-asset holding/transfer rules.
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
	pub struct AssetPolicy {
		pub min_tier: KycTier,
		pub require_accredited: bool,
		/// If non-empty, only these jurisdictions may hold. Empty = all except `blocked`.
		pub allowed_jurisdictions: BoundedVec<u16, ConstU32<64>>,
		pub blocked_jurisdictions: BoundedVec<u16, ConstU32<64>>,
		pub frozen: bool,
	}

	#[pallet::config]
	pub trait Config:
		frame::deps::frame_system::Config + pallet_timestamp::Config
	{
		/// The overarching runtime event type.
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as frame::deps::frame_system::Config>::RuntimeEvent>;
		/// Identifier of a regulated asset. `Default` lets benchmarks name an asset generically.
		type AssetId: Parameter + Member + Copy + Default + MaxEncodedLen;
		/// Origin allowed to set claims / policies / freeze. A multisig / root — never the hot
		/// enrollment key.
		type ComplianceOrigin: EnsureOrigin<
			<Self as frame::deps::frame_system::Config>::RuntimeOrigin,
		>;
		/// Read-only identity resolution, so `set_claim` can reject unknown identities.
		type Identity: IdentityLookup<Self::AccountId>;
		/// Weights for this pallet's dispatchables.
		type WeightInfo: WeightInfo;
		/// Creates the identity that `set_claim` needs; benchmarking only.
		#[cfg(feature = "runtime-benchmarks")]
		type BenchmarkHelper: kollectyve_primitives::IdentityProvisioner<Self::AccountId>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// `IdentityId -> current KYC claim`.
	#[pallet::storage]
	pub type Claims<T: Config> = StorageMap<_, Blake2_128Concat, IdentityId, KycClaim, OptionQuery>;

	/// `AssetId -> policy`.
	#[pallet::storage]
	pub type AssetPolicies<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AssetId, AssetPolicy, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		ClaimSet { id: IdentityId, tier: KycTier },
		ClaimRevoked { id: IdentityId },
		AssetPolicySet { asset: T::AssetId },
		AssetFrozen { asset: T::AssetId, frozen: bool },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The identity has no KYC claim.
		NotKyced,
		/// The identity's claim has expired.
		ClaimExpired,
		/// The identity's tier is below the asset's minimum.
		TierTooLow,
		/// The asset requires accreditation the identity lacks.
		NotAccredited,
		/// The identity's jurisdiction is blocked or not on the allow-list.
		JurisdictionNotAllowed,
		/// No policy is configured for the asset.
		UnknownAssetPolicy,
		/// The asset is frozen.
		AssetFrozen,
		/// No identity exists for the given id (claims may only target live identities).
		UnknownIdentity,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set (or replace) the KYC claim for an existing identity.
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config>::WeightInfo::set_claim())]
		pub fn set_claim(origin: OriginFor<T>, id: IdentityId, claim: KycClaim) -> DispatchResult {
			T::ComplianceOrigin::ensure_origin(origin)?;
			ensure!(T::Identity::is_active(id), Error::<T>::UnknownIdentity);
			let tier = claim.tier;
			Claims::<T>::insert(id, claim);
			Self::deposit_event(Event::ClaimSet { id, tier });
			Ok(())
		}

		/// Remove an identity's KYC claim.
		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::revoke_claim())]
		pub fn revoke_claim(origin: OriginFor<T>, id: IdentityId) -> DispatchResult {
			T::ComplianceOrigin::ensure_origin(origin)?;
			Claims::<T>::remove(id);
			Self::deposit_event(Event::ClaimRevoked { id });
			Ok(())
		}

		/// Set (or replace) an asset's policy.
		#[pallet::call_index(2)]
		#[pallet::weight(<T as Config>::WeightInfo::set_asset_policy())]
		pub fn set_asset_policy(
			origin: OriginFor<T>,
			asset: T::AssetId,
			policy: AssetPolicy,
		) -> DispatchResult {
			T::ComplianceOrigin::ensure_origin(origin)?;
			AssetPolicies::<T>::insert(asset, policy);
			Self::deposit_event(Event::AssetPolicySet { asset });
			Ok(())
		}

		/// Freeze or unfreeze an asset.
		#[pallet::call_index(3)]
		#[pallet::weight(<T as Config>::WeightInfo::set_frozen())]
		pub fn set_frozen(
			origin: OriginFor<T>,
			asset: T::AssetId,
			frozen: bool,
		) -> DispatchResult {
			T::ComplianceOrigin::ensure_origin(origin)?;
			AssetPolicies::<T>::try_mutate(asset, |m| -> DispatchResult {
				let p = m.as_mut().ok_or(Error::<T>::UnknownAssetPolicy)?;
				p.frozen = frozen;
				Ok(())
			})?;
			Self::deposit_event(Event::AssetFrozen { asset, frozen });
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Check that identity `id` satisfies `policy` (tier, accreditation, jurisdiction, expiry).
		fn eligible(id: IdentityId, policy: &AssetPolicy) -> DispatchResult {
			let c = Claims::<T>::get(id).ok_or(Error::<T>::NotKyced)?;
			if let Some(exp) = c.expires_at {
				let now: u64 = pallet_timestamp::Pallet::<T>::get().saturated_into();
				ensure!(now <= exp, Error::<T>::ClaimExpired);
			}
			ensure!(c.tier >= policy.min_tier, Error::<T>::TierTooLow);
			if policy.require_accredited {
				ensure!(c.accredited, Error::<T>::NotAccredited);
			}
			ensure!(
				!policy.blocked_jurisdictions.contains(&c.jurisdiction),
				Error::<T>::JurisdictionNotAllowed
			);
			if !policy.allowed_jurisdictions.is_empty() {
				ensure!(
					policy.allowed_jurisdictions.contains(&c.jurisdiction),
					Error::<T>::JurisdictionNotAllowed
				);
			}
			Ok(())
		}
	}

	impl<T: Config> HoldingPolicy<T::AssetId> for Pallet<T> {
		fn can_hold(who: IdentityId, asset: &T::AssetId) -> DispatchResult {
			let p = AssetPolicies::<T>::get(asset).ok_or(Error::<T>::UnknownAssetPolicy)?;
			ensure!(!p.frozen, Error::<T>::AssetFrozen);
			Self::eligible(who, &p)
		}
		fn can_transfer(from: IdentityId, to: IdentityId, asset: &T::AssetId) -> DispatchResult {
			let p = AssetPolicies::<T>::get(asset).ok_or(Error::<T>::UnknownAssetPolicy)?;
			ensure!(!p.frozen, Error::<T>::AssetFrozen);
			Self::eligible(from, &p)?;
			Self::eligible(to, &p)
		}
	}
}
