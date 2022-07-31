use crate::{mock::*, Error};
use frame_support::{
	traits::{Currency},
	assert_noop, assert_ok
};

#[test]
fn test_genesis_balances() {
	let (user_account_id, controller_account_id, stash_account_id) = account_ids();
	let initial_balances = initial_balances(user_account_id, controller_account_id, stash_account_id);

	new_test_ext(initial_balances).execute_with(|| {
		assert_eq!(<MainBalances as Currency<u64>>::total_balance(&user_account_id), 10, 
			"origin balance was not as expected");	
	});
}

#[test]
fn add_stake_transfers_dot() {
	let (user_account_id, controller_account_id, stash_account_id) = account_ids();
	let initial_balances = initial_balances(user_account_id, controller_account_id, stash_account_id);

	new_test_ext(initial_balances).execute_with(|| {
		// Account 1 starts with 10 DOT
		assert_ok!(LiquidStakingModule::add_stake(Origin::signed(user_account_id), 3));

        println!("User main: {} User derivative: {} Pot main: {} Pot derivative: {}",
                 <MainBalances as Currency<u64>>::total_balance(&user_account_id),
                 <DerivativeBalances as Currency<u64>>::total_balance(&user_account_id),
                 <MainBalances as Currency<u64>>::total_balance(&LiquidStakingModule::stash_account_id()),
                 <DerivativeBalances as Currency<u64>>::total_balance(&LiquidStakingModule::stash_account_id()));

		assert_eq!(<MainBalances as Currency<u64>>::total_balance(&LiquidStakingModule::stash_account_id()), 7, 
			"origin balance diminished by main currency transfer amount");	
		assert_eq!(<MainBalances as Currency<u64>>::total_balance(&LiquidStakingModule::stash_account_id()), 23, 
			"slash account augmented by main currency transfer amount");
		assert_eq!(<MainBalances as Currency<u64>>::total_balance(&LiquidStakingModule::controller_account_id()), 0, 
			"staking does not affect controller account balance");
	});
}


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

fn initial_balances(user_account_id: u64, controller_account_id: u64, stash_account_id: u64) -> Vec<(u64, u128, u128)> {
	vec![
		(user_account_id, 10, 70),
		(controller_account_id, 0, 0),
		(stash_account_id, 20, 160),
	]
}
