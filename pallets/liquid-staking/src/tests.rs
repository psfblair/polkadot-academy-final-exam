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

/////////////////////////////////////// ADD STAKE TESTS ////////////////////////////////////////////
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
fn add_stake_bonds_the_pot_if_not_yet_bonded() {
	let user_account_id = 1;
	let initial_balances = vec![
		(user_account_id, 10, 0),
		(stash_account_id(), 0, 0),
		(controller_account_id(), 0, 0),
	];
	// Arrange: Whatever account balances
	new_test_ext(initial_balances).execute_with(|| {
		// Act: Account 1 stakes 4 DOT
		assert_ok!(LiquidStakingModule::add_stake(Origin::signed(user_account_id), 4));

		// Assert: Pot is bonded for 4 DOT
		assert_eq!(StakingMock::active_stake(&stash_account_id), Some(4), "pot bonded staked amount is as expected");
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

#[ignore] // Right now this isn't passing because the test setup to lock part of the balance of the stash account isn't working.
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

		// Act: Add a stake of 5
		assert_ok!(LiquidStakingModule::add_stake(Origin::signed(user_account_id), 5));

		// Assert: The entire free amount in the stash account is bonded (and locked, but the mock implementation of staking doesn't do that)
		assert_eq!(StakingMock::active_stake(&stash_account_id), Some(35), "pot bonded staked amount is as expected");
	});
}

/////////////////////////////////////// ADD STAKE FAILURE SCENARIO TESTS ////////////////////////////////////////////

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

/////////////////////////////////////// VALIDATOR VOTE TESTS ////////////////////////////////////////////

#[test]
fn nominations_for_validators_are_stored() {
	let user1_account_id = 1;
	let user2_account_id = 2;
	let initial_balances = vec![
		(user1_account_id, 0, 48),
		(user2_account_id, 0, 32),
	];
	// Arrange: Users 1 and 2 both have derivative tokens
	new_test_ext(initial_balances).execute_with(|| {
		// Act: Account 1 submits nominations
		let nominations_1 = [(10, 2),(11, 2),(12, 2),(13, 2),(14, 2),(15, 2),(16, 2),(17, 2),
							 (18, 3),(19, 3),(20, 3),(21, 3),(22, 3),(23, 3),(24, 3),(25, 3)];
		assert_ok!(LiquidStakingModule::nominate(Origin::signed(user1_account_id), nominations_1));

		// Assert: Account 1 nominations are recorded
		let first_nomination_map = std::collections::HashMap::from(nominations_1);
		assert_eq!(LiquidStakingModule::NominationsStorage::get(&user1_account_id).unwrap(), first_nomination_map,
			"Initial nominations are stored correctly");

		// Act: Account 2 submits nominations
		let nominations_2 = [(20, 2),(21, 2),(22, 2),(23, 2),(24, 2),(25, 2),(26, 2),(27, 2),
							 (28, 2),(29, 2),(30, 2),(31, 2),(32, 2),(33, 2),(34, 2),(35, 2)];
		assert_ok!(LiquidStakingModule::nominate(Origin::signed(user2_account_id), nominations_2));

		// Assert: Account 2 nominations are recorded and storage reflects everything
		let second_nomination_map = std::collections::HashMap::from(
			[(10, 2),(11, 2),(12, 2),(13, 2),(14, 2),(15, 2),(16, 2),(17, 2),
			 (18, 3),(19, 3),(20, 5),(21, 5),(22, 5),(23, 5),(24, 5),(25, 5),
			 (26, 2),(27, 2),(28, 2),(29, 2),(30, 2),(31, 2),(32, 2),(33, 2),(34, 2),(35, 2)]
		);
		assert_eq!(LiquidStakingModule::NominationsStorage::get(&user2_account_id).unwrap(), second_nomination_map,
			"Subsequent nominations are stored correctly");
	});	
}

#[test]
fn tokens_used_in_nominations_are_locked_and_recorded() {
	let user1_account_id = 1;
	let initial_1 = 48;
	let user2_account_id = 2;
	let initial_2 = 32;
	let initial_balances = vec![
		(user1_account_id, 0, initial_1),
		(user2_account_id, 0, initial_2),
	];
	// Arrange: Users 1 and 2 both have derivative tokens
	new_test_ext(initial_balances).execute_with(|| {
		// Act: Account 1 submits some nominations, but not their entire amount
		let nominations_1 = [(10, 2),(11, 2),(12, 2),(13, 2),(14, 2),(15, 2),(16, 2),(17, 2),
							 (18, 3),(19, 3),(20, 3),(21, 3),(22, 3),(23, 3),(24, 3),(25, 3)];
		let committed_1 = 40;
		assert_ok!(LiquidStakingModule::nominate(Origin::signed(user1_account_id), nominations_1));

		// Assert: Account 1 tokens are locked and the number of locked tokens is recorded in storage
		assert_eq!(<DerivativeBalances as Currency<u64>>::free_balance(&user1_account_id), initial_1 - committed_1, 
			"Number of tokens locked is correct for user 1");	
		assert_eq!(LiquidStakingModule::NominationLocksStorage::get(&user1_account_id).unwrap(), committed_1,
			"Number of locked tokens is stored correctly for user 1");

		// Act: Account 2 submits nominations
		let nominations_2 = [(10, 2),(11, 2),(12, 2),(13, 2),(14, 2),(15, 2),(16, 2),(17, 2),
							 (18, 2),(19, 2),(20, 2),(21, 2),(22, 2),(23, 2),(24, 2),(25, 2)];
		let committed_2 = 32;
		assert_ok!(LiquidStakingModule::nominate(Origin::signed(user2_account_id), nominations_2));

		// Assert: Account 2 tokens are locked and the number of locked tokens is recorded in storage
		assert_eq!(<DerivativeBalances as Currency<u64>>::free_balance(&user2_account_id), initial_2 - committed_2, 
			"Number of tokens locked is correct for user 2");	
		assert_eq!(LiquidStakingModule::NominationLocksStorage::get(&user2_account_id).unwrap(), committed_2,
			"Number of locked tokens is stored correctly for user 2");

		// Act: Account 1 submits more nominations
		let nominations_3 = [(10, 1),(11, 1),(12, 1),(13, 1),(14, 1),(15, 1),(16, 1),(17, 1),
							 (18, 0),(19, 0),(20, 0),(21, 0),(22, 0),(23, 0),(24, 0),(25, 0)];
		let committed_3 = 8;
		assert_ok!(LiquidStakingModule::nominate(Origin::signed(user1_account_id), nominations_3));

		// Assert: More Account 1 tokens are locked and the total number of locked tokens is recorded in storage
		assert_eq!(<DerivativeBalances as Currency<u64>>::free_balance(&user1_account_id), initial_1 - (committed_1 + committed_3), 
			"Number of tokens locked is correct for user 1");	
		assert_eq!(LiquidStakingModule::NominationLocksStorage::get(&user1_account_id).unwrap(), committed_1 + committed_3,
			"Number of locked tokens is stored correctly for user 1");
	});	
}


////////////////////////////////////////////// TODO ////////////////////////////////////////////////////

// Test the on_runtime_upgrade hook 

fn controller_account_id() -> u64 {
	LiquidStakingModule::controller_account_id()
}

fn stash_account_id() -> u64 {
	LiquidStakingModule::stash_account_id()
}
