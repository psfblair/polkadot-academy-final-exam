use crate::{
    mock::{new_test_ext, LiquidStakingModule, MainBalances, DerivativeBalances, Origin, Test, StakingMock}, 
    Event, Error,
};
use frame_support::{
	traits::{Currency, LockableCurrency, WithdrawReasons},
	assert_noop, assert_ok
};
use frame_system::pallet::Pallet;
use sp_staking::StakingInterface;

#[test]
fn test_genesis_balances() {
	let user_account_id = 1;
	let initial_balances = vec![
		(user_account_id, 10, 70),
	];

	// Arrange and Act -- set up genesis
	new_test_ext(initial_balances).execute_with(|| {
		// Assert: Balances don't overwrite each other
		assert_eq!(<MainBalances as Currency<u64>>::total_balance(&user_account_id), 10, 
			"genesis main balance was not as expected");	
		assert_eq!(<MainBalances as Currency<u64>>::total_balance(&user_account_id), 10, 
			"genesis derivative balance was not as expected");	
	});
}

#[test]
fn add_stake_transfers_dot() {
	let user_account_id = 1;
	let initial_balances = vec![
		(user_account_id, 10, 70),
		(stash_account_id(), 20, 160),
		(controller_account_id(), 0, 0),
	];

	// Arrange: Account 1 starts with 10 DOT
	new_test_ext(initial_balances).execute_with(|| {
		// Act: Account 1 stakes 3 
		assert_ok!(LiquidStakingModule::add_stake(Origin::signed(user_account_id), 3));

		// Assert: Money gets transferred to stash account
		assert_eq!(<MainBalances as Currency<u64>>::total_balance(&user_account_id), 7, 
				"origin balance diminished by staked amount");	
		assert_eq!(<MainBalances as Currency<u64>>::total_balance(&stash_account_id()), 23, 
				"stash account augmented by staked amount");
		assert_eq!(<MainBalances as Currency<u64>>::total_balance(&controller_account_id()), 0, 
				"staking does not affect controller account balance");
	});
}

#[test]
fn add_stake_mints_sdot_for_first_staker() {
	let user_account_id = 1;
	let initial_balances = vec![
		(user_account_id, 10, 0),
		(stash_account_id(), 0, 0),
		(controller_account_id(), 0, 0),
	];

	// Arrange: Account 1 starts with 10 DOT
	new_test_ext(initial_balances).execute_with(|| {
		// Act: Account 1 stakes 3
		assert_ok!(LiquidStakingModule::add_stake(Origin::signed(user_account_id), 3));

		// Assert: Account 1 gets back 3 sDOT
		assert_eq!(<DerivativeBalances as Currency<u64>>::total_balance(&user_account_id), 3, 
				"origin sDOT balance increased by staked amount at inception");	
		assert_eq!(<DerivativeBalances as Currency<u64>>::total_issuance(), 3, 
				"total sDOT issuance increased by staked amount at inception");

		assert_eq!(<DerivativeBalances as Currency<u64>>::total_balance(&stash_account_id()), 0, 
				"stash sDOT account balance unaffected by staking");
		assert_eq!(<DerivativeBalances as Currency<u64>>::total_balance(&controller_account_id()), 0, 
				"controller sDOT account balance unaffected by staking");
	});
}

#[test]
fn add_stake_mints_sdot_for_later_staker_after_rewards() {
	let user_account_id = 1;	
	let initial_balances = vec![
		(user_account_id, 7, 3),
		(stash_account_id(), 6, 0),
		(controller_account_id(), 0, 0),
	];
	
	// Arrange: Account 1 starts with 7 DOT and 3 sDOT; stash account has 6 DOT
	new_test_ext(initial_balances).execute_with(|| {
		// Act: Account 1 stakes 4 DOT
		assert_ok!(LiquidStakingModule::add_stake(Origin::signed(user_account_id), 4));

		// Assert: Account 1 gets back 2 sDOT
		assert_eq!(<DerivativeBalances as Currency<u64>>::total_balance(&user_account_id), 5, 
				"origin sDOT balance increased correctly");	
		assert_eq!(<DerivativeBalances as Currency<u64>>::total_issuance(), 5, 
				"total sDOT issuance increased correctly");
	});	
}

#[test]
fn add_stake_mints_sdot_for_later_staker_after_slash() {
	let user_account_id = 1;
	let initial_balances = vec![
		(user_account_id, 7, 3),
		(stash_account_id(), 2, 0),
		(controller_account_id(), 0, 0),
	];
	
	// Arrange: Account 1 starts with 7 DOT and 3 sDOT; stash account has 2 DOT
	new_test_ext(initial_balances).execute_with(|| {
		// Act: Account 1 stakes 4 DOT
		assert_ok!(LiquidStakingModule::add_stake(Origin::signed(user_account_id), 4));

		// Assert: Account 1 gets back 6 sDOT
		assert_eq!(<DerivativeBalances as Currency<u64>>::total_balance(&user_account_id), 9, 
				"origin sDOT balance increased correctly");	
		assert_eq!(<DerivativeBalances as Currency<u64>>::total_issuance(), 9, 
				"total sDOT issuance increased correctly");
	});	
}

#[test]
fn add_stake_deposits_stake_added_event() {
	let user_account_id = 1;
	let initial_balances = vec![
		(user_account_id, 10, 0),
		(stash_account_id(), 0, 0),
	];
	// Arrange: Whatever account balances
	new_test_ext(initial_balances).execute_with(|| {
		// Act: Account 1 stakes 4 DOT
		assert_ok!(LiquidStakingModule::add_stake(Origin::signed(user_account_id), 4));

		// Assert: Event is deposited
		Pallet::<Test>::assert_has_event(Event::StakeAdded(user_account_id, 4).into());
	});	
}

#[test]
fn add_stake_bonds_with_all_free_funds_available() {
	let user_account_id = 1;
	let stash_account_id = stash_account_id();
	let initial_balances = vec![
		(user_account_id, 10, 0),
		(stash_account_id, 30, 0),
	];
	
	// Arrange: Stash account has 30 DOT
	new_test_ext(initial_balances).execute_with(|| {
		// Arrange: Set up a state where 20 of the balance of 30 are already locked and bonded
		let bonded_amount = 20;
		assert_ok!(StakingMock::bond(stash_account_id, controller_account_id(), bonded_amount, stash_account_id));
		<MainBalances as LockableCurrency<u64>>::set_lock(*b"stlockid", &stash_account_id, bonded_amount, WithdrawReasons::RESERVE);

		// Act: Add stake
		assert_ok!(LiquidStakingModule::add_stake(Origin::signed(user_account_id), 5));

		// Assert: Bonded amount is increased (and locked, but the mock doesn't do the locking)
		assert_eq!(StakingMock::active_stake(&stash_account_id), Some(35), "pot bonded staked amount is as expected");
		assert_eq!(<MainBalances as Currency<u64>>::free_balance(&stash_account_id), 0, "stash is entirely locked");
	});
}

// add_stake failure scenarios:

#[test]
fn add_stake_fails_with_insufficient_balance() {
	let user_account_id = 1;
	let initial_balances = vec![
		(user_account_id, 0, 0),
	];

	// Arrange: Account 1 starts with 10 DOT
	new_test_ext(initial_balances).execute_with(|| {
		// Act and Assert: Account 1 tries to stake 30 and gets InsufficientBalance
		assert_noop!(LiquidStakingModule::add_stake(Origin::signed(user_account_id), 30),
			pallet_balances::pallet::Error::<Test, pallet_balances::Instance1>::InsufficientBalance
		);
	});
}

#[test]
fn add_stake_fails_with_insufficient_stake() {
	let user_account_id = 1;
	let initial_balances = vec![
		(user_account_id, 10, 70),
	];

	// Arrange: Account 1 starts with 10 DOT
	new_test_ext(initial_balances).execute_with(|| {
		// Act and Assert: Account 1 tries to stake 1 and gets InsufficientStake
		assert_noop!(LiquidStakingModule::add_stake(Origin::signed(user_account_id), 1),
			Error::<Test>::InsufficientStake
		);
	});
}

#[test]
fn add_stake_fails_when_max_stake_exceeded() {
	let big_amount = u128::MAX - 2u128;
	let user_account_id = 1;
	let initial_balances = vec![
		(user_account_id, big_amount, 0),
		(stash_account_id(), 2, 2),
	];

	// Arrange: Account 1 starts with u128::MAX - 2 DOT (have to leave room for total issuance not to overflow)
	new_test_ext(initial_balances).execute_with(|| {
		// Act and Assert: Account 1 tries to stake the entire amount and gets ExceededMaxStake
		assert_noop!(LiquidStakingModule::add_stake(Origin::signed(user_account_id), big_amount),
			Error::<Test>::ExceededMaxStake
		);
	});
}

fn controller_account_id() -> u64 {
	LiquidStakingModule::controller_account_id()
}

fn stash_account_id() -> u64 {
	LiquidStakingModule::stash_account_id()
}
