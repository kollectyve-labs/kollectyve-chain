//! # Pallet Kollectyve Credentials — format-neutral credential registry
//!
//! Records the **existence, provenance and status** of verifiable credentials. The credential
//! document itself (a W3C VC today, an ACDC later) lives off-chain with its holder; the chain holds
//! only its digest and a small record keyed to Kollectyve identities. One on-chain model, several
//! export formats — that is what lets the KILT-style and KERI-style ways coexist.
//!
//! ## The model
//! - A **schema** is the hash of a credential type's definition (a KILT "CType", a JSON Schema,
//!   an ACDC schema SAID). Any identity may register one; the registry only stops duplicates.
//! - A **credential** is keyed by its `digest` — the hash of the canonical off-chain document —
//!   and records issuer, subject, schema, validity window and revocation. No claim content and
//!   no PII ever reach the chain.
//! - **Chaining** (the ACDC idea): `authorized_by` may name a credential *held by the issuer*
//!   that entitles it to issue — a ministry accredits a university, the university issues a
//!   diploma. The chain checks the edge is live at issuance; a verifier walks the edges back to a
//!   **trust anchor**, an identity that governance has marked as a root of trust.
//! - Issuer and subject are [`IdentityId`]s, never accounts: rotating a controller key (including
//!   a pre-rotated one) leaves every credential and the right to revoke it intact.
//!
//! ## Origins
//! - Schemas, issuance and revocation are **signed by an identity's controller**. Only the issuer
//!   may revoke, and nobody can edit a credential — reissue under a new digest instead.
//! - [`set_trust_anchor`](pallet::Pallet::set_trust_anchor) needs **`AnchorOrigin`** (governance /
//!   root). Anchoring is advisory: it tells verifiers where chains may end, it gates nothing.
//!
//! Deferred (see the credentials design doc): refundable storage deposits, linking KERI
//! identifiers to an identity, and acting as a KERI witness.

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
	use kollectyve_primitives::{IdentityId, IdentityLookup};
	use polkadot_sdk::pallet_timestamp;

	/// On-chain record of one credential. Times are unix milliseconds.
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
	pub struct Credential {
		pub issuer: IdentityId,
		pub subject: IdentityId,
		pub schema: H256,
		/// Digest of a credential held by `issuer` that authorised this issuance, if any.
		pub authorized_by: Option<H256>,
		pub issued_at: u64,
		/// After this instant the credential is stale. `None` = never expires.
		pub expires_at: Option<u64>,
		/// Set once, by the issuer. Revoked records are kept so verifiers can tell "revoked"
		/// from "never issued".
		pub revoked_at: Option<u64>,
	}

	#[pallet::config]
	pub trait Config:
		frame::deps::frame_system::Config<RuntimeEvent: From<Event<Self>>>
		+ pallet_timestamp::Config
	{
		/// Resolves signers to identities and checks subjects exist.
		type Identity: IdentityLookup<Self::AccountId>;
		/// Origin allowed to mark or unmark trust anchors. Governance / root.
		type AnchorOrigin: EnsureOrigin<
			<Self as frame::deps::frame_system::Config>::RuntimeOrigin,
		>;
		/// Weights for this pallet's dispatchables.
		type WeightInfo: WeightInfo;
		/// Creates the identities benchmarks need; benchmarking only.
		#[cfg(feature = "runtime-benchmarks")]
		type BenchmarkHelper: kollectyve_primitives::IdentityProvisioner<Self::AccountId>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// `schema hash -> identity that registered it`.
	#[pallet::storage]
	pub type Schemas<T: Config> = StorageMap<_, Blake2_128Concat, H256, IdentityId, OptionQuery>;

	/// `credential digest -> record`.
	#[pallet::storage]
	pub type Credentials<T: Config> =
		StorageMap<_, Blake2_128Concat, H256, Credential, OptionQuery>;

	/// Identities governance recognises as roots of trust. Presence means anchored.
	#[pallet::storage]
	pub type TrustAnchors<T: Config> =
		StorageMap<_, Blake2_128Concat, IdentityId, (), OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		SchemaRegistered { schema: H256, creator: IdentityId },
		CredentialIssued {
			digest: H256,
			issuer: IdentityId,
			subject: IdentityId,
			schema: H256,
			authorized_by: Option<H256>,
		},
		CredentialRevoked { digest: H256, issuer: IdentityId },
		TrustAnchorSet { id: IdentityId, anchored: bool },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The signer controls no identity.
		NoIdentity,
		/// No identity exists for the given id.
		UnknownIdentity,
		/// The schema is already registered.
		SchemaExists,
		/// The schema is not registered.
		UnknownSchema,
		/// A credential with this digest already exists.
		AlreadyIssued,
		/// The expiry is not in the future.
		AlreadyExpired,
		/// No credential exists for the given digest.
		UnknownCredential,
		/// Only the issuer may revoke.
		NotIssuer,
		/// The credential is already revoked.
		AlreadyRevoked,
		/// The authorising credential does not exist.
		UnknownAuthorization,
		/// The authorising credential is not held by the issuer.
		AuthorizationNotHeld,
		/// The authorising credential is revoked.
		AuthorizationRevoked,
		/// The authorising credential has expired.
		AuthorizationExpired,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Register a credential schema by its hash. Any identity may do so.
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config>::WeightInfo::register_schema())]
		pub fn register_schema(origin: OriginFor<T>, schema: H256) -> DispatchResult {
			let creator = Self::signer_identity(origin)?;
			ensure!(!Schemas::<T>::contains_key(schema), Error::<T>::SchemaExists);
			Schemas::<T>::insert(schema, creator);
			Self::deposit_event(Event::SchemaRegistered { schema, creator });
			Ok(())
		}

		/// Anchor a credential issued by the signer's identity to `subject`.
		///
		/// `digest` is the hash of the canonical off-chain credential and becomes its id.
		/// `authorized_by`, when set, must be a live, unexpired credential whose subject is the
		/// issuer — the edge a verifier follows back to a trust anchor.
		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::issue())]
		pub fn issue(
			origin: OriginFor<T>,
			subject: IdentityId,
			schema: H256,
			digest: H256,
			authorized_by: Option<H256>,
			expires_at: Option<u64>,
		) -> DispatchResult {
			let issuer = Self::signer_identity(origin)?;
			ensure!(T::Identity::is_active(subject), Error::<T>::UnknownIdentity);
			ensure!(Schemas::<T>::contains_key(schema), Error::<T>::UnknownSchema);
			ensure!(!Credentials::<T>::contains_key(digest), Error::<T>::AlreadyIssued);
			let now = Self::now();
			if let Some(exp) = expires_at {
				ensure!(exp > now, Error::<T>::AlreadyExpired);
			}
			if let Some(parent) = authorized_by {
				let p = Credentials::<T>::get(parent).ok_or(Error::<T>::UnknownAuthorization)?;
				ensure!(p.subject == issuer, Error::<T>::AuthorizationNotHeld);
				ensure!(p.revoked_at.is_none(), Error::<T>::AuthorizationRevoked);
				if let Some(exp) = p.expires_at {
					ensure!(now <= exp, Error::<T>::AuthorizationExpired);
				}
			}
			Credentials::<T>::insert(
				digest,
				Credential {
					issuer,
					subject,
					schema,
					authorized_by,
					issued_at: now,
					expires_at,
					revoked_at: None,
				},
			);
			Self::deposit_event(Event::CredentialIssued {
				digest,
				issuer,
				subject,
				schema,
				authorized_by,
			});
			Ok(())
		}

		/// Revoke a credential. **Issuer-only**; the record stays, stamped with the time.
		///
		/// Credentials that list this one in `authorized_by` are not touched: a verifier walking
		/// the chain finds the revoked edge and rejects them.
		#[pallet::call_index(2)]
		#[pallet::weight(<T as Config>::WeightInfo::revoke())]
		pub fn revoke(origin: OriginFor<T>, digest: H256) -> DispatchResult {
			let who = Self::signer_identity(origin)?;
			let now = Self::now();
			Credentials::<T>::try_mutate(digest, |c| -> DispatchResult {
				let c = c.as_mut().ok_or(Error::<T>::UnknownCredential)?;
				ensure!(c.issuer == who, Error::<T>::NotIssuer);
				ensure!(c.revoked_at.is_none(), Error::<T>::AlreadyRevoked);
				c.revoked_at = Some(now);
				Ok(())
			})?;
			Self::deposit_event(Event::CredentialRevoked { digest, issuer: who });
			Ok(())
		}

		/// Mark (or unmark) an identity as a root of trust for verifiers.
		#[pallet::call_index(3)]
		#[pallet::weight(<T as Config>::WeightInfo::set_trust_anchor())]
		pub fn set_trust_anchor(
			origin: OriginFor<T>,
			id: IdentityId,
			anchored: bool,
		) -> DispatchResult {
			T::AnchorOrigin::ensure_origin(origin)?;
			if anchored {
				ensure!(T::Identity::is_active(id), Error::<T>::UnknownIdentity);
				TrustAnchors::<T>::insert(id, ());
			} else {
				TrustAnchors::<T>::remove(id);
			}
			Self::deposit_event(Event::TrustAnchorSet { id, anchored });
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Whether `digest` names an issued, unrevoked, unexpired credential. Checks this record
		/// only — verifiers still walk `authorized_by` to a trust anchor.
		pub fn is_valid(digest: H256) -> bool {
			Credentials::<T>::get(digest).is_some_and(|c| {
				c.revoked_at.is_none() && c.expires_at.map_or(true, |exp| Self::now() <= exp)
			})
		}

		fn signer_identity(origin: OriginFor<T>) -> Result<IdentityId, DispatchError> {
			let who = ensure_signed(origin)?;
			T::Identity::identity_of(&who).ok_or_else(|| Error::<T>::NoIdentity.into())
		}

		fn now() -> u64 {
			pallet_timestamp::Pallet::<T>::get().saturated_into()
		}
	}
}
