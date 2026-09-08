//! # Kollectyve primitives
//!
//! The shared vocabulary of the Kollectyve account-abstraction layer. Both anchor pallets
//! (`pallet-kollectyve-id`, `pallet-kollectyve-policy`) and every consumer pallet depend on
//! **this** crate for the common types and read traits — never on each other. That keeps the
//! coupling a shallow star instead of a chain of pallet-to-pallet dependencies.
//!
//! Contents:
//! - [`IdentityId`] — the stable, network-wide identity handle.
//! - [`KycTier`] — coarse KYC tier, ordered lowest-to-highest.
//! - [`IdentityLookup`] — read interface exposed by the identity anchor.
//! - [`HoldingPolicy`] — read interface exposed by the regulated-asset policy pallet.

#![cfg_attr(not(feature = "std"), no_std)]

use frame::prelude::*;

/// Stable, network-wide identity handle. Reused across every Kollectyve vertical.
pub type IdentityId = u64;

/// Coarse KYC tier attached to an identity. Ordering matters: policy checks compare
/// `tier >= min_tier`, so variants are declared lowest-to-highest.
#[derive(
	Clone,
	Copy,
	Encode,
	Decode,
	DecodeWithMemTracking,
	Default,
	Eq,
	PartialEq,
	PartialOrd,
	Ord,
	DebugNoBound,
	TypeInfo,
	MaxEncodedLen,
)]
pub enum KycTier {
	#[default]
	None,
	Basic,
	Verified,
	Accredited,
}

/// Read interface the identity anchor (`pallet-kollectyve-id`) exposes to other pallets.
///
/// Loose coupling: consumers depend on this trait, not on the anchor pallet's extrinsics.
pub trait IdentityLookup<AccountId> {
	/// The identity controlled by `who`, if any.
	fn identity_of(who: &AccountId) -> Option<IdentityId>;
	/// The controller account of `id`, if the identity exists.
	fn controller_of(id: IdentityId) -> Option<AccountId>;
	/// The KYC tier of `id` (defaults to [`KycTier::None`] for unknown ids).
	fn tier(id: IdentityId) -> KycTier;
	/// Whether `id` names a live identity.
	fn is_active(id: IdentityId) -> bool;
}

/// Read interface the regulated-asset gate (`pallet-kollectyve-policy`) exposes to consumer
/// pallets. A consumer calls these before moving a regulated position; a failing `DispatchResult`
/// carries the exact compliance reason.
pub trait HoldingPolicy<AssetId> {
	/// May identity `who` hold `asset`?
	fn can_hold(who: IdentityId, asset: &AssetId) -> DispatchResult;
	/// May `asset` move from identity `from` to identity `to`? Both ends must be eligible.
	fn can_transfer(from: IdentityId, to: IdentityId, asset: &AssetId) -> DispatchResult;
}

/// Lets a benchmark materialise an identity, which [`IdentityLookup`] deliberately cannot do.
///
/// A pallet benchmarking a call that requires a live identity is generic over `IdentityLookup`
/// and so has no way to create one. The runtime and the test mocks satisfy this with the real
/// anchor pallet; nothing outside benchmarking can reach it.
#[cfg(feature = "runtime-benchmarks")]
pub trait IdentityProvisioner<AccountId> {
	/// Create an identity controlled by `controller` and return its id.
	fn provision(controller: AccountId) -> IdentityId;
}

