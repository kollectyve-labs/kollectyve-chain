use crate::{
	AccountId, BalancesConfig, KollectyveIdConfig, ReviveConfig, Runtime, RuntimeGenesisConfig,
	SudoConfig,
};
use alloc::{vec, vec::Vec};
use polkadot_sdk::*;
use frame_support::build_struct_json_patch;
use pallet_revive::AddressMapper;
use serde_json::Value;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_genesis_builder::{self, PresetId};
use sp_keyring::Sr25519Keyring;

/// Well-known Ethereum development accounts, as the `AccountId32` they map to under
/// [`pallet_revive::AccountId32Mapper`] (the 20-byte address padded with `0xee`).
///
/// Endowing these is what lets you import one of the matching (public, dev-only) private keys
/// into MetaMask and immediately deploy from Remix or Hardhat against `--dev`. Same set the
/// upstream `revive-dev-node` funds; see `contracts/.env.example` for the keys.
fn eth_dev_accounts() -> Vec<AccountId> {
	[
		// alith
		"f24ff3a9cf04c71dbc94d0b566f7a27b94566caceeeeeeeeeeeeeeeeeeeeeeee",
		// baltathar
		"3cd0a705a2dc65e5b1e1205896baa2be8a07c6e0eeeeeeeeeeeeeeeeeeeeeeee",
		// charleth
		"798d4ba9baf0064ec19eb4f0a1a45785ae9d6dfceeeeeeeeeeeeeeeeeeeeeeee",
		// dorothy
		"773539d4ac0e786233d90a233654ccee26a613d9eeeeeeeeeeeeeeeeeeeeeeee",
		// ethan
		"ff64d3f6efe2317ee2807d223a0bdc4c0c49dfdbeeeeeeeeeeeeeeeeeeeeeeee",
	]
	.iter()
	.map(array_bytes::hex_n_into_unchecked::<_, AccountId, 32>)
	.collect()
}

// Returns the genesis config presets populated with given parameters.
fn testnet_genesis(
	initial_authorities: Vec<(AuraId, GrandpaId)>,
	endowed_accounts: Vec<AccountId>,
	root: AccountId,
) -> Value {
	// Fund the Ethereum dev accounts alongside the substrate ones so contract deployment works
	// out of the box.
	let endowed_accounts: Vec<AccountId> =
		endowed_accounts.into_iter().chain(eth_dev_accounts()).collect();

	build_struct_json_patch!(RuntimeGenesisConfig {
		balances: BalancesConfig {
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, 1u128 << 60))
				.collect::<Vec<_>>(),
		},
		// Pre-register the `AccountId32 <-> H160` mapping for every endowed account, so the
		// first transaction from one of them doesn't have to pay for its own mapping.
		revive: ReviveConfig {
			mapped_accounts: endowed_accounts
				.iter()
				.filter(|a| !<Runtime as pallet_revive::Config>::AddressMapper::is_mapped(a))
				.cloned()
				.collect::<Vec<_>>(),
		},
		aura: pallet_aura::GenesisConfig {
			authorities: initial_authorities.iter().map(|x| x.0.clone()).collect::<Vec<_>>(),
		},
		grandpa: pallet_grandpa::GenesisConfig {
			authorities: initial_authorities.iter().map(|x| (x.1.clone(), 1)).collect::<Vec<_>>(),
		},
		// Both privileged accounts default to the sudo key here, so `register_identity` / `set_tier`
		// can be exercised either via sudo or signed directly by the root key on a `--dev` chain. A
		// production chain spec overrides these with the real hot enrollment key and the 2-of-3
		// compliance multisig.
		kollectyve_id: KollectyveIdConfig {
			enrollment_key: Some(root.clone()),
			compliance_key: Some(root.clone()),
		},
		sudo: SudoConfig { key: Some(root) },
	})
}

/// Return the development genesis config.
pub fn development_config_genesis() -> Value {
	testnet_genesis(
		vec![(
			sp_keyring::Sr25519Keyring::Alice.public().into(),
			sp_keyring::Ed25519Keyring::Alice.public().into(),
		)],
		vec![
			Sr25519Keyring::Alice.to_account_id(),
			Sr25519Keyring::Bob.to_account_id(),
			Sr25519Keyring::AliceStash.to_account_id(),
			Sr25519Keyring::BobStash.to_account_id(),
		],
		sp_keyring::Sr25519Keyring::Alice.to_account_id(),
	)
}

/// Return the local genesis config preset.
pub fn local_config_genesis() -> Value {
	testnet_genesis(
		vec![
			(
				sp_keyring::Sr25519Keyring::Alice.public().into(),
				sp_keyring::Ed25519Keyring::Alice.public().into(),
			),
			(
				sp_keyring::Sr25519Keyring::Bob.public().into(),
				sp_keyring::Ed25519Keyring::Bob.public().into(),
			),
		],
		Sr25519Keyring::iter()
			.filter(|v| v != &Sr25519Keyring::One && v != &Sr25519Keyring::Two)
			.map(|v| v.to_account_id())
			.collect::<Vec<_>>(),
		Sr25519Keyring::Alice.to_account_id(),
	)
}

/// Provides the JSON representation of predefined genesis config for given `id`.
pub fn get_preset(id: &PresetId) -> Option<Vec<u8>> {
	let patch = match id.as_ref() {
		sp_genesis_builder::DEV_RUNTIME_PRESET => development_config_genesis(),
		sp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET => local_config_genesis(),
		_ => return None,
	};
	Some(
		serde_json::to_string(&patch)
			.expect("serialization to json is expected to work. qed.")
			.into_bytes(),
	)
}

/// List of supported presets.
pub fn preset_names() -> Vec<PresetId> {
	vec![
		PresetId::from(sp_genesis_builder::DEV_RUNTIME_PRESET),
		PresetId::from(sp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET),
	]
}
