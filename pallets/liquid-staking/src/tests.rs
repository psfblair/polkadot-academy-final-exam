use crate::{mock::*, Error};
use frame_support::{
	traits::{Currency},
	assert_noop, assert_ok
};

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
		(controller_account_id(), 0, 0),
		(stash_account_id(), 20, 160),
	];

	new_test_ext(initial_balances).execute_with(|| {
		// Account 1 starts with 10 DOT and 
		assert_ok!(LiquidStakingModule::add_stake(Origin::signed(user_account_id), 3));

		assert_eq!(<MainBalances as Currency<u64>>::total_balance(&user_account_id), 7, 
				"origin balance diminished by transfer amount");	
		assert_eq!(<MainBalances as Currency<u64>>::total_balance(&stash_account_id()), 23, 
				"stash account augmented by transfer amount");
		assert_eq!(<MainBalances as Currency<u64>>::total_balance(&controller_account_id()), 0, 
				"staking does not affect controller account balance");
	});
}

#[test]
fn add_stake_fails_with_insufficient_balance() {
	let user_account_id = 1;
	let initial_balances = vec![
		(user_account_id, 0, 0),
	];
	new_test_ext(initial_balances).execute_with(|| {
		// Account 1 starts with 10 DOT
		assert_noop!(LiquidStakingModule::add_stake(Origin::signed(user_account_id), 30),
			pallet_balances::pallet::Error::<Test>::InsufficientBalance
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
		// Account 1 starts with 10 DOT
		assert_noop!(LiquidStakingModule::add_stake(Origin::signed(user_account_id), 1),
			Error::<Test>::InsufficientStake
		);
	});
}

fn controller_account_id() -> u64 {
	LiquidStakingModule::controller_account_id()
}

fn stash_account_id() -> u64 {
	LiquidStakingModule::stash_account_id()
}
