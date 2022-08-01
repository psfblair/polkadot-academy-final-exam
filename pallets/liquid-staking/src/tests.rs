use crate::{
    mock::{new_test_ext, LiquidStakingModule, MainBalances, DerivativeBalances, Origin, Test}, 
    Event, Error,
};
use frame_support::{
	traits::{Currency, LockableCurrency, WithdrawReasons},
	assert_noop, assert_ok
};
use frame_system::pallet::Pallet;

#[test]
fn test_genesis_balances() {
	let user_account_id = 1;
	let initial_balances = vec![
		(user_account_id, 10, 70),
	];

	new_test_ext(initial_balances).execute_with(|| {
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

	new_test_ext(initial_balances).execute_with(|| {
		// Account 1 starts with 10 DOT and stakes 3 
		assert_ok!(LiquidStakingModule::add_stake(Origin::signed(user_account_id), 3));

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

	new_test_ext(initial_balances).execute_with(|| {
		// Account 1 starts with 10 DOT and stakes 3, gets back 3 sDOT
		assert_ok!(LiquidStakingModule::add_stake(Origin::signed(user_account_id), 3));

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
	// Account 1 starts with 7 DOT and 3 sDOT; stash account now has 6 DOT
	let initial_balances = vec![
		(user_account_id, 7, 3),
		(stash_account_id(), 6, 0),
		(controller_account_id(), 0, 0),
	];
	
	new_test_ext(initial_balances).execute_with(|| {
		// Account 1 stakes 4 DOT, gets back 2 sDOT
		assert_ok!(LiquidStakingModule::add_stake(Origin::signed(user_account_id), 4));

		assert_eq!(<DerivativeBalances as Currency<u64>>::total_balance(&user_account_id), 5, 
				"origin sDOT balance increased correctly");	
		assert_eq!(<DerivativeBalances as Currency<u64>>::total_issuance(), 5, 
				"total sDOT issuance increased correctly");
	});	
}

#[test]
fn add_stake_mints_sdot_for_later_staker_after_slash() {
	let user_account_id = 1;
	// Account 1 starts with 7 DOT and 3 sDOT; stash account now has 2 DOT
	let initial_balances = vec![
		(user_account_id, 7, 3),
		(stash_account_id(), 2, 0),
		(controller_account_id(), 0, 0),
	];
	
	new_test_ext(initial_balances).execute_with(|| {
		// Account 1 stakes 4 DOT, gets back 6 sDOT
		assert_ok!(LiquidStakingModule::add_stake(Origin::signed(user_account_id), 4));

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
	
	new_test_ext(initial_balances).execute_with(|| {
		assert_ok!(LiquidStakingModule::add_stake(Origin::signed(user_account_id), 4));

		Pallet::<Test>::assert_has_event(Event::StakeAdded(user_account_id, 4).into());
	});	
}

#[test]
fn add_stake_bonds_with_all_free_funds_available() {
	let stash_account_id = stash_account_id();
	let initial_balances = vec![
		(user_account_id, 10, 0),
		(stash_account_id, 30, 0),
	];
	
	new_test_ext(initial_balances).execute_with(|| {
		// Set up a state where 20 of the balance of 30 are already locked and bonded
		Staking::bond_extra(Origin::signed(stash_account_id), amount);
		<MainBalances as LockableCurrency<u64>>::set_lock(*b"lockid", &stash_account_id, 20, WithdrawReasons::RESERVE);

		assert_ok!(LiquidStakingModule::add_stake(Origin::signed(user_account_id), 5));

		assert_eq!(Staking::active_stake(&stash_account_id()), 35, "pot bonded staked amount is as expected");
		assert_eq!(<MainBalances as Currency<u64>>::free_balance(&stash_account_id), 0, "stash is entirely locked");
	}
}

// add_stake failure scenarios:

#[test]
fn add_stake_fails_with_insufficient_balance() {
	let user_account_id = 1;
	let initial_balances = vec![
		(user_account_id, 0, 0),
	];
	new_test_ext(initial_balances).execute_with(|| {
		// Account 1 starts with 10 DOT; we try to stake 30
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
	new_test_ext(initial_balances).execute_with(|| {
		// Try to stake 1 DOT
		assert_noop!(LiquidStakingModule::add_stake(Origin::signed(user_account_id), 1),
			Error::<Test>::InsufficientStake
		);
	});
}

#[test]
fn add_stake_fails_when_max_stake_exceeded() {
	let user_account_id = 1;
	let initial_balances = vec![
		(user_account_id, u128::MAX - 2u128, 0),
		(stash_account_id(), 2, 2),
	];
	new_test_ext(initial_balances).execute_with(|| {
		// Try to stake u128::MAX DOT
		assert_noop!(LiquidStakingModule::add_stake(Origin::signed(user_account_id), u128::MAX),
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
