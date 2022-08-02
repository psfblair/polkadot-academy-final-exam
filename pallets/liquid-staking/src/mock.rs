use crate as pallet_liquid_staking;
use frame_support::{
	pallet_prelude::*,
	traits::{ConstU16, ConstU64, StorageMapShim,},
	storage::bounded_btree_map::BoundedBTreeMap,
	PalletId, parameter_types,
};
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage,
};
use sp_staking::{EraIndex, StakingInterface};
use sp_std::collections::btree_map::BTreeMap;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
type BalanceImpl = u128;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		LiquidStakingModule: pallet_liquid_staking::{Pallet, Call, Storage, Event<T>},
		MainBalances: pallet_balances::<Instance1>::{Pallet, Call, Storage, Config<T, Instance1>, Event<T, Instance1>},
		DerivativeBalances: pallet_balances::<Instance2>::{Pallet, Call, Storage, Config<T, Instance2>, Event<T, Instance2>},
	}
);

parameter_types! {
	// With some more work this would not be needed but into_sub_account_truncating does not result
	// in distinct account IDs for the two accounts, so something more involved would be needed.
	pub const PalletIdImpl: PalletId = PalletId(*b"px/lstkg");
	pub const PalletIdImpl2: PalletId = PalletId(*b"py/lstkg");
	pub const MinimumStakeImpl: BalanceImpl = 2;
	pub static ExistentialDepositImpl: BalanceImpl = 0;
}

impl crate::pallet::Config for Test {
	type Event = Event;
	type PalletId = PalletIdImpl;
	type PalletId2 = PalletIdImpl2;
	type MinimumStake = MinimumStakeImpl;
	type MainCurrency = MainBalances;
	type DerivativeCurrency = DerivativeBalances;
	type StakingInterface = StakingMock;
}

type MainToken = pallet_balances::Instance1;
impl pallet_balances::Config<MainToken> for Test {
	type MaxLocks = frame_support::traits::ConstU32<1024>;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type Balance = BalanceImpl;
	type Event = Event;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDepositImpl;
	type AccountStore = System;
	type WeightInfo = ();
}

type DerivativeToken = pallet_balances::Instance2;
impl pallet_balances::Config<DerivativeToken> for Test {
	type MaxLocks = frame_support::traits::ConstU32<1024>;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type Balance = BalanceImpl;
	type Event = Event;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDepositImpl;
	type AccountStore = StorageMapShim<pallet_balances::pallet::Account<Test, DerivativeToken>, frame_system::Provider<Test>, Self::AccountId, pallet_balances::AccountData<BalanceImpl>>;
	type WeightInfo = ();
}

// StakingMock below is taken from the nomination-pools pallet
parameter_types! {
	pub static CurrentEra: EraIndex = 0;
	pub static BondingDuration: EraIndex = 3;
	pub storage BondedBalanceMap: BTreeMap<AccountId, Balance> = Default::default();
	pub storage UnbondingBalanceMap: BTreeMap<AccountId, Balance> = Default::default();
	#[derive(Clone, PartialEq)]
	pub static MaxUnbonding: u32 = 8;
	pub storage Nominations: Option<Vec<AccountId>> = None;
}

pub struct StakingMock;
impl StakingMock {
	pub(crate) fn set_bonded_balance(who: AccountId, bonded: Balance) {
		let mut x = BondedBalanceMap::get();
		x.insert(who, bonded);
		BondedBalanceMap::set(&x)
	}
}

impl sp_staking::StakingInterface for StakingMock {
	type Balance = BalanceImpl;
	type AccountId = Self::AccountId;

	fn minimum_bond() -> Self::Balance {
		10
	}

	fn current_era() -> EraIndex {
		CurrentEra::get()
	}

	fn bonding_duration() -> EraIndex {
		BondingDuration::get()
	}

	fn active_stake(who: &Self::AccountId) -> Option<Self::Balance> {
		BondedBalanceMap::get().get(who).map(|v| *v)
	}

	fn total_stake(who: &Self::AccountId) -> Option<Self::Balance> {
		match (
			UnbondingBalanceMap::get().get(who).map(|v| *v),
			BondedBalanceMap::get().get(who).map(|v| *v),
		) {
			(None, None) => None,
			(Some(v), None) | (None, Some(v)) => Some(v),
			(Some(a), Some(b)) => Some(a + b),
		}
	}

	fn bond_extra(who: Self::AccountId, extra: Self::Balance) -> DispatchResult {
		let mut x = BondedBalanceMap::get();
		x.get_mut(&who).map(|v| *v += extra);
		BondedBalanceMap::set(&x);
		Ok(())
	}

	fn unbond(who: Self::AccountId, amount: Self::Balance) -> DispatchResult {
		let mut x = BondedBalanceMap::get();
		*x.get_mut(&who).unwrap() = x.get_mut(&who).unwrap().saturating_sub(amount);
		BondedBalanceMap::set(&x);
		let mut y = UnbondingBalanceMap::get();
		*y.entry(who).or_insert(Self::Balance::zero()) += amount;
		UnbondingBalanceMap::set(&y);
		Ok(())
	}

	fn chill(_: Self::AccountId) -> sp_runtime::DispatchResult {
		Ok(())
	}

	fn withdraw_unbonded(who: Self::AccountId, _: u32) -> Result<bool, DispatchError> {
		// Simulates removing unlocking chunks and only having the bonded balance locked
		let mut x = UnbondingBalanceMap::get();
		x.remove(&who);
		UnbondingBalanceMap::set(&x);

		Ok(UnbondingBalanceMap::get().is_empty() && BondedBalanceMap::get().is_empty())
	}

	fn bond(
		stash: Self::AccountId,
		_: Self::AccountId,
		value: Self::Balance,
		_: Self::AccountId,
	) -> DispatchResult {
		StakingMock::set_bonded_balance(stash, value);
		Ok(())
	}

	fn nominate(_: Self::AccountId, nominations: Vec<Self::AccountId>) -> DispatchResult {
		Nominations::set(&Some(nominations));
		Ok(())
	}
}

impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();	
	type Origin = Origin;
	type Call = Call;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = ConstU64<250>;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<BalanceImpl>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ConstU16<42>;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

pub fn new_test_ext(users: Vec<(u64, u128, u128)>) -> sp_io::TestExternalities {
	let mut storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
	GenesisConfig {
		main_balances: MainBalancesConfig {
			balances: users.iter().map(|(account_id, main_balance, _)| (*account_id, *main_balance) ).collect(),
		},
		derivative_balances: DerivativeBalancesConfig {
			balances: users.iter().map(|(account_id, _, derivative_balances)| (*account_id, *derivative_balances) ).collect(),
		},
		..Default::default()
	}
	.assimilate_storage(&mut storage)
	.unwrap();
	
	let mut externalities = sp_io::TestExternalities::new(storage);
	externalities.execute_with(|| System::set_block_number(1));
	externalities
}
