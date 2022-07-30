use crate as pallet_liquid_staking;
use frame_support::{
	traits::{ConstU16, ConstU64, },
	PalletId, parameter_types,
};
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage,
};

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
		MainBalances: pallet_balances::<Instance1>::{Pallet, Call, Storage, Config<T>, Event<T>},
		DerivativeBalances: pallet_balances::<Instance2>::{Pallet, Call, Storage, Config<T>, Event<T>},
	}
);

parameter_types! {
	pub const LiquidStakingPalletId: PalletId = PalletId(*b"py/lstkg");
	pub static ExistentialDeposit: BalanceImpl = 0;
}

impl crate::pallet::Config for Test {
	type Event = Event;
	type PalletId = LiquidStakingPalletId;
	type MainCurrency = MainBalances;
	type DerivativeCurrency = DerivativeBalances;
}

type MainToken = pallet_balances::Instance1;
impl pallet_balances::Config<MainToken> for Test {
	type MaxLocks = frame_support::traits::ConstU32<1024>;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type Balance = BalanceImpl;
	type Event = Event;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
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
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
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

// Build genesis storage according to the mock runtime.
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
