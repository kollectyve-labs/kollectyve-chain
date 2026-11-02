use polkadot_sdk::*;

use alloc::{vec, vec::Vec};
use frame_support::{
	derive_impl,
	dispatch::DispatchClass,
	parameter_types,
	traits::{
		ConstBool, ConstU128, ConstU32, ConstU64, ConstU8, EitherOfDiverse, SortedMembers,
		VariantCountOf,
	},
	weights::{
		constants::{RocksDbWeight, WEIGHT_REF_TIME_PER_SECOND},
		ConstantMultiplier, Weight,
	},
};
use frame_system::limits::{BlockLength, BlockWeights};
use frame_system::{EnsureRoot, EnsureSigned, EnsureSignedBy};
use pallet_transaction_payment::{ConstFeeMultiplier, FungibleAdapter, Multiplier};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_runtime::{traits::One, Perbill};
use sp_version::RuntimeVersion;

use super::{
	AccountId, Address, Aura, Balance, Balances, Block, BlockNumber, EthExtraImpl, Hash,
	KollectyveId, Nonce, PalletInfo, Runtime, RuntimeCall, RuntimeEvent, RuntimeFreezeReason,
	RuntimeHoldReason, RuntimeOrigin, RuntimeTask, Signature, System, Timestamp,
	EXISTENTIAL_DEPOSIT, MICRO_UNIT, MILLI_UNIT, SLOT_DURATION, UNIT, VERSION,
};

const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

parameter_types! {
	pub const BlockHashCount: BlockNumber = 2400;
	pub const Version: RuntimeVersion = VERSION;

	/// We allow for 2 seconds of compute with a 6 second average block time.
	pub RuntimeBlockWeights: BlockWeights = BlockWeights::with_sensible_defaults(
		Weight::from_parts(2u64 * WEIGHT_REF_TIME_PER_SECOND, u64::MAX),
		NORMAL_DISPATCH_RATIO,
	);
	pub RuntimeBlockLength: BlockLength = BlockLength::builder()
		.max_length(5 * 1024 * 1024)
		.modify_max_length_for_class(DispatchClass::Normal, |m| *m = NORMAL_DISPATCH_RATIO * *m)
		.build();
	pub const SS58Prefix: u8 = 42;
}

/// All single-block migrations of the runtime, aside from the ones declared in the pallets.
///
/// This can be a tuple of types, each implementing `OnRuntimeUpgrade`.
#[allow(unused_parens)]
pub type SingleBlockMigrations = ();

/// The default types are being injected by [`derive_impl`](`frame_support::derive_impl`) from
/// [`SoloChainDefaultConfig`](`struct@frame_system::config_preludes::SolochainDefaultConfig`),
/// but overridden as needed.
#[derive_impl(frame_system::config_preludes::SolochainDefaultConfig)]
impl frame_system::Config for Runtime {
	/// The block type for the runtime.
	type Block = Block;
	/// Block & extrinsics weights: base values and limits.
	type BlockWeights = RuntimeBlockWeights;
	/// The maximum length of a block (in bytes).
	type BlockLength = RuntimeBlockLength;
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The type for storing how many extrinsics an account has signed.
	type Nonce = Nonce;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	/// The weight of database operations that the runtime can invoke.
	type DbWeight = RocksDbWeight;
	/// Version of the runtime.
	type Version = Version;
	/// The data to be stored in an account.
	type AccountData = pallet_balances::AccountData<Balance>;
	/// This is used as an identifier of the chain. 42 is the generic substrate prefix.
	type SS58Prefix = SS58Prefix;
	type MaxConsumers = frame_support::traits::ConstU32<16>;
	type SingleBlockMigrations = SingleBlockMigrations;
	/// Keeps the pallet-revive address map in sync as accounts appear and disappear, which is
	/// what makes `pallet_revive::Config::AutoMap` work: an Ethereum account transacting for
	/// the first time gets its `AccountId32 <-> H160` mapping created automatically.
	type OnNewAccount = pallet_revive::AutoMapper<Runtime>;
	type OnKilledAccount = pallet_revive::AutoMapper<Runtime>;
}

impl pallet_aura::Config for Runtime {
	type AuthorityId = AuraId;
	type DisabledValidators = ();
	type MaxAuthorities = ConstU32<32>;
	type AllowMultipleBlocksPerSlot = ConstBool<false>;
	type SlotDuration = pallet_aura::MinimumPeriodTimesTwo<Runtime>;
}

impl pallet_grandpa::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;

	type WeightInfo = ();
	type MaxAuthorities = ConstU32<32>;
	type MaxNominators = ConstU32<0>;
	type MaxSetIdSessionEntries = ConstU64<0>;

	type KeyOwnerProof = sp_core::Void;
	type EquivocationReportSystem = ();
}

impl pallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = Aura;
	type MinimumPeriod = ConstU64<{ SLOT_DURATION / 2 }>;
	type WeightInfo = ();
}

impl pallet_balances::Config for Runtime {
	type MaxLocks = ConstU32<50>;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	/// The type for recording an account's balance.
	type Balance = Balance;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ConstU128<EXISTENTIAL_DEPOSIT>;
	type AccountStore = System;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
	type FreezeIdentifier = RuntimeFreezeReason;
	type MaxFreezes = VariantCountOf<RuntimeFreezeReason>;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type DoneSlashHandler = ();
}

parameter_types! {
	pub FeeMultiplier: Multiplier = Multiplier::one();
	pub const TransactionByteFee: Balance = 10 * MICRO_UNIT;
}

impl pallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction = FungibleAdapter<Balances, ()>;
	type OperationalFeeMultiplier = ConstU8<5>;
	/// `BlockRatioFee` prices a unit of weight as a fraction of the block, which is what gives
	/// pallet-revive a sane weight <-> gas conversion. With the previous `IdentityFee` the
	/// figures `eth_estimateGas` and MetaMask show are meaningless. Mirrors the upstream
	/// `revive-dev-node` runtime.
	type WeightToFee = pallet_revive::evm::fees::BlockRatioFee<1, 1, Self, Balance>;
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type FeeMultiplierUpdate = ConstFeeMultiplier<FeeMultiplier>;
	type WeightInfo = pallet_transaction_payment::weights::SubstrateWeight<Runtime>;
}

impl pallet_sudo::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type WeightInfo = pallet_sudo::weights::SubstrateWeight<Runtime>;
}

// --- Multisig (backs the 2-of-3 compliance origin) ---

parameter_types! {
	// Deposits held while a multisig operation is pending. Sized like the Substrate node defaults.
	pub const MultisigDepositBase: Balance = deposit(1, 88);
	pub const MultisigDepositFactor: Balance = deposit(0, 32);
}

impl pallet_multisig::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type DepositBase = MultisigDepositBase;
	type DepositFactor = MultisigDepositFactor;
	type MaxSignatories = ConstU32<100>;
	type WeightInfo = pallet_multisig::weights::SubstrateWeight<Runtime>;
	type BlockNumberProvider = System;
}

// --- Kollectyve account abstraction ---
//
// Two separate privileged origins, each `Root OR a specific signer`:
//   * `EnrollmentOrigin`  — a single hot KMS key; may ONLY `register_identity` (tier `None`).
//   * `ComplianceOrigin`  — the 2-of-3 multisig address; may set tiers / claims / policies / freeze.
// Keeping them distinct means a compromised enrollment key cannot alter compliance state.
//
// The privileged accounts are NOT hardcoded here — they are read from `pallet_kollectyve_id`
// genesis storage, so an operator wires the real enrollment key and 2-of-3 multisig address via the
// chain spec (`kollectyveId.enrollmentKey` / `.complianceKey`) at deploy time. When unset (the dev
// default) the member set is empty, so each origin resolves to root/sudo only.
pub struct EnrollmentMembers;
impl SortedMembers<AccountId> for EnrollmentMembers {
	fn sorted_members() -> Vec<AccountId> {
		pallet_kollectyve_id::EnrollmentKey::<Runtime>::get().map_or_else(Vec::new, |a| vec![a])
	}
}

pub struct ComplianceMembers;
impl SortedMembers<AccountId> for ComplianceMembers {
	fn sorted_members() -> Vec<AccountId> {
		pallet_kollectyve_id::ComplianceKey::<Runtime>::get().map_or_else(Vec::new, |a| vec![a])
	}
}

/// Enrollment: root or the genesis-wired hot enrollment key.
pub type EnrollmentOrigin =
	EitherOfDiverse<EnsureRoot<AccountId>, EnsureSignedBy<EnrollmentMembers, AccountId>>;
/// Compliance: root or the genesis-wired 2-of-3 multisig address.
pub type ComplianceOrigin =
	EitherOfDiverse<EnsureRoot<AccountId>, EnsureSignedBy<ComplianceMembers, AccountId>>;

/// Stable on-chain identity anchor.
impl pallet_kollectyve_id::Config for Runtime {
	type EnrollmentOrigin = EnrollmentOrigin;
	type ComplianceOrigin = ComplianceOrigin;
	type WeightInfo = pallet_kollectyve_id::weights::SubstrateWeight<Runtime>;
}

/// KYC/compliance gate for regulated assets.
impl pallet_kollectyve_policy::Config for Runtime {
	type AssetId = u32;
	type ComplianceOrigin = ComplianceOrigin;
	type Identity = KollectyveId;
	type WeightInfo = pallet_kollectyve_policy::weights::SubstrateWeight<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = KollectyveId;
}

/// Verifiable-credential registry keyed to identities. Trust anchors are governance's call, which
/// today means sudo; it moves with sudo when governance lands.
impl pallet_kollectyve_credentials::Config for Runtime {
	type Identity = KollectyveId;
	type AnchorOrigin = EnsureRoot<AccountId>;
	type WeightInfo = pallet_kollectyve_credentials::weights::SubstrateWeight<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = KollectyveId;
}

/// Provider registry — records keyed by identity, no policy gate.
impl pallet_kumulus::Config for Runtime {
	type Identity = KollectyveId;
	type WeightInfo = pallet_kumulus::weights::SubstrateWeight<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = KollectyveId;
}

// --- pallet-revive configuration ---

const ETH: u128 = 1_000_000_000_000_000_000;

pub const fn deposit(items: u32, bytes: u32) -> Balance {
	(items as Balance * UNIT + (bytes as Balance) * (5 * MILLI_UNIT / 100)) / 10
}

parameter_types! {
	pub const DepositPerItem: Balance = deposit(1, 0);
	pub const DepositPerByte: Balance = deposit(0, 1);
	pub const DepositPerChildTrieItem: Balance = deposit(1, 0) / 100;
	pub const CodeHashLockupDepositPercent: Perbill = Perbill::from_percent(30);
	pub const NativeToEthRatio: u32 = (ETH / UNIT) as u32;
}

/// Smart contracts.
///
/// `TestDefaultConfig` supplies the associated types we have no reason to diverge on; the
/// notable inherited default is `AllowEVMBytecode = true`, which enables the pallet's REVM
/// backend. That is what lets contracts compiled by plain `solc` — i.e. anything produced by
/// Remix, Hardhat or Foundry — be uploaded and executed unmodified, alongside PolkaVM blobs
/// produced by `resolc`.
#[derive_impl(pallet_revive::config_preludes::TestDefaultConfig)]
impl pallet_revive::Config for Runtime {
	type AddressMapper = pallet_revive::AccountId32Mapper<Self>;
	type Balance = Balance;
	type Currency = Balances;
	/// Ethereum chain id, as reported by `eth_chainId`. Configure MetaMask with this value.
	type ChainId = ConstU64<28000>;
	type Time = Timestamp;
	type NativeToEthRatio = NativeToEthRatio;
	type DepositPerItem = DepositPerItem;
	type DepositPerByte = DepositPerByte;
	type DepositPerChildTrieItem = DepositPerChildTrieItem;
	type CodeHashLockupDepositPercent = CodeHashLockupDepositPercent;
	type UploadOrigin = EnsureSigned<Self::AccountId>;
	type InstantiateOrigin = EnsureSigned<Self::AccountId>;
	/// Reconstructs the fee an Ethereum transaction would pay, so gas figures reported over
	/// the eth-rpc proxy match what the chain actually charges.
	type FeeInfo = pallet_revive::evm::fees::Info<Address, Signature, EthExtraImpl>;
	type RuntimeMemory = ConstU32<{ 128 * 1024 * 1024 }>;
	type PVFMemory = ConstU32<{ 512 * 1024 * 1024 }>;
	/// Maps an unmapped account on first use — required for MetaMask-signed transactions from
	/// accounts that have never touched the substrate side. Paired with the `AutoMapper` hooks
	/// on `frame_system::Config`.
	type AutoMap = ConstBool<true>;
	type DebugEnabled = ConstBool<true>;
	type GasScale = ConstU32<50_000>;
	type WeightInfo = pallet_revive::weights::SubstrateWeight<Self>;
}
