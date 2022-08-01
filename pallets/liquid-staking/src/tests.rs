use crate::{mock::*, Error};
use frame_support::{
	traits::{Currency},
	assert_noop, assert_ok
};

#[test]
fn test_genesis_balances() {
	let (user_account_id, controller_account_id, stash_account_id) = account_ids();
	let initial_balances = vec![
		(user_account_id, 10, 70),
		(controller_account_id, 0, 0),
		(stash_account_id, 20, 160),
	];

	new_test_ext(initial_balances).execute_with(|| {
		assert_eq!(<MainBalances as Currency<u64>>::total_balance(&user_account_id), 10, 
			"origin balance was not as expected");	
	});
}

#[test]
fn add_stake_transfers_dot() {
	let (user_account_id, controller_account_id, stash_account_id) = account_ids();
	let initial_balances = vec![
		(user_account_id, 10, 70),
		(controller_account_id, 0, 0),
		(stash_account_id, 20, 160),
	];

	new_test_ext(initial_balances).execute_with(|| {
		// Account 1 starts with 10 DOT and 
		assert_ok!(LiquidStakingModule::add_stake(Origin::signed(user_account_id), 3));

		assert_eq!(<MainBalances as Currency<u64>>::total_balance(&user_account_id), 7, 
				"origin balance diminished by transfer amount");	
		assert_eq!(<MainBalances as Currency<u64>>::total_balance(&LiquidStakingModule::stash_account_id()), 23, 
				"stash account augmented by transfer amount");
			assert_eq!(<MainBalances as Currency<u64>>::total_balance(&LiquidStakingModule::controller_account_id()), 0, 
		"staking does not affect controller account balance");
	});
}

// #[test]
// fn add_stake_fails_with_insufficient_balance() {
// 	let (user_account_id, controller_account_id, stash_account_id) = account_ids();
// 	let initial_balances = initial_balances(user_account_id, controller_account_id, stash_account_id);

// 	new_test_ext(initial_balances).execute_with(|| {
// 		// Account 1 starts with 10 DOT
// 		assert_noop!(LiquidStakingModule::add_stake(Origin::signed(user_account_id), 30),
// 			Error::<Test>::InsufficientBalance
// 		);
// 	});
// }


/* Test errors
#[test]
fn correct_error_for_none_value() {
	new_test_ext().execute_with(|| {
		// Ensure the expected error is thrown when no value is present.
		assert_noop!(LiquidStakingModule::cause_error(Origin::signed(1)), Error::<Test>::NoneValue);
	});
}

*/

fn account_ids() -> (u64, u64, u64){
	let user_account_id = 1;
	let controller_account_id = LiquidStakingModule::controller_account_id();
	let stash_account_id = LiquidStakingModule::stash_account_id();
	(user_account_id, controller_account_id, stash_account_id)
}

