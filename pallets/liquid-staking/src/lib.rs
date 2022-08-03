#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

use frame_support::traits::Currency; 
pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
// This type will be used for both balances pallets; they are tied together below in Config.
pub type BalanceTypeOf<T> = <<T as Config>::MainCurrency as Currency<AccountIdOf<T>>>::Balance; 

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{
			pallet_prelude::*,
			sp_runtime::{
				traits::{AccountIdConversion, CheckedAdd, CheckedMul, CheckedDiv, Zero}
			},
			traits::{
				Currency, LockableCurrency, WithdrawReasons, LockIdentifier,
				tokens::ExistenceRequirement,
			},
			PalletId, BoundedBTreeMap,
	};
	use frame_system::{pallet_prelude::*};
	use sp_staking::{StakingInterface, EraIndex};
	use crate::{AccountIdOf, BalanceTypeOf};

	const NOMINATION_LOCK_ID: LockIdentifier = *b"nomlocks";

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type MainCurrency: LockableCurrency<AccountIdOf<Self>>;
		type DerivativeCurrency: LockableCurrency<AccountIdOf<Self>, Balance = BalanceTypeOf<Self>>;
		type TransactionId: IsType<<Self as frame_system::Config>::Hash> + Encode + Decode + Clone + PartialEq + TypeInfo + core::fmt::Debug;
       
        #[pallet::constant]
		type PalletId: Get<PalletId>;
        #[pallet::constant]
		type PalletId2: Get<PalletId>;
		/// The minimum quantity of the main currency that is allowed to be staked.
		#[pallet::constant]
		type MinimumStake: Get<BalanceTypeOf<Self>>;
		/// The largest number of validators that can possibly be nominated, which sets a bound for the vector passed in
		/// when the staker submits a nomination.
		#[pallet::constant]
		type MaxValidatorNominees: Get<u32>; // Maybe this is like the 640k thing but I think voting for more than 255 validators would be ridiculous
		/// Number of blocks after the start of the era during which stakers can vote on which validators the pool will nominate.
		#[pallet::constant]
		type NominatorVotingPeriodBlocks: Get<BlockNumberFor<Self>>;
		/// The bound for the max number of eras that can be held in the BoundedBTreeMap holding token redemption information. 
		/// See the documentation of `RedemptionsAwaitingWithdrawal` below. 
		#[pallet::constant]
		type WithdrawalBound: Get<u32>; 

		type StakingInterface: sp_staking::StakingInterface<
			Balance = BalanceTypeOf<Self>,
			AccountId = AccountIdOf<Self>,
		>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// The era of the previous block, for detecting the block when the era changes.
	#[pallet::storage]
	#[pallet::getter(fn era_of_previous_block)]
	pub type EraOfPreviousBlock<T> = StorageValue<_, EraIndex>;

	/// The block number of the start of the current era.
	#[pallet::storage]
	#[pallet::getter(fn era_start_block)]
	pub type EraStartBlockNumber<T> = StorageValue<_, BlockNumberFor<T>>;

	/// The number of derivative tokens that a stakeholder has redeemed, paired with the era when the stakeholder
	/// can withdraw the corresponding main token from the stash account. Since one account can redeem tokens multiple
	/// times, the value is a vector of such pairs. There is potentially a way for this vector to grow large if a 
	/// stakeholder redeems tokens and then does not withdraw the funds, over a long period of time. We will collapse
	/// the data by making it instead a map of maps: account_id -> era -> amount  . Besides the economic disincentive
	/// that failing to withdraw one's tokens makes spamming the system expensive, we can also institute a BoundedBTreeMap
	/// such that if a stakeholder is waiting to withdraw tokens for more than a configurable number of eras, those entries
	/// fall out of the map and redemption can no longer occur.
	///
	/// TWOX-QUESTION: `AccountId`s are crypto hashes anyway, so is this safe? What if an attacker knew a user's account ID and
	/// wanted to change their redemptions? Isn't this account ID a public thing?
	#[pallet::storage]
	#[pallet::getter(fn redemptions_awaiting_withdrawal)]
	pub type RedemptionsAwaitingWithdrawal<T> = StorageMap<_, Twox64Concat, AccountIdOf<T>, BoundedBTreeMap<EraIndex, BalanceTypeOf<T>, <T as Config>::WithdrawalBound>>;

	/// The quantity of derivative tokens an account has locked during the voting period.
	///
	/// TWOX-QUESTION: Same question as above 
	#[pallet::storage]
	#[pallet::getter(fn nomination_locks_for)]
	pub type NominationLocksStorage<T> = StorageMap<_, Twox64Concat, AccountIdOf<T>, BalanceTypeOf<T>>;

	/// The votes that each validator candidate has received in this round of voting.
	///
	/// TWOX-QUESTION: `AccountId`s are crypto hashes anyway, so is this safe? Aren't the IDs of validators generally known, and
	/// if so would this be liable to being attacked by those who know those IDs?
	#[pallet::storage]
	#[pallet::getter(fn nomination_votes_for)]
	pub type NominationsStorage<T> = StorageMap<_, Twox64Concat, AccountIdOf<T>, BalanceTypeOf<T>>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// An account has staked a certain amount of the staking token with the pallet [account, amount]
		StakeAdded(AccountIdOf<T>, BalanceTypeOf<T>),

		/// An account has redeemed a certain amount of the derivative token with the pallet. The underlying 
		/// main token may be claimed on the era specified in the event. [account, amount, era, transaction_id]
		DerivativeRedeemed(AccountIdOf<T>, BalanceTypeOf<T>, EraIndex),
		
		/// The staking tokens associated with the redeemed liquid tokens have been unbonded and 
		/// credited to the staker [account, amount]
		StakeReleased(AccountIdOf<T>, BalanceTypeOf<T>, BalanceTypeOf<T>),
		
		// ValidatorVoteSubmitted,
		// ReferendumVoteSubmitted,
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Amount staked is less than minimum account balance
		InsufficientStake,
		/// Amount staked exceeded maximum amount allowed
		ExceededMaxStake,

		/// A request was made to redeem a auantity of derivative token that was greater
		/// than the free balance in the account.
		InsufficientFundsForRedemption,

		/// A request was made to redeem a auantity of derivative token but so many previous
		/// redemptions are awaiting withdrawal that no additional redemptions may be performed.
		TooManyRedemptionsAwaitingWithdrawal,

		// A request to vote for validators has originated from an account that does not
		// hold the derivative token, or at a time when voting is not taking place.
		VoteUnauthorized,

		/// The quantity of votes submitted exceeded the submitter's free balance of
		/// the derivative token or exceeded the maximum allowable value of those tokens.
		VoteQuantityInvalid,

		/// The number of votes received by a validator exceeds the maximum permissible amount.
		ValidatorVoteQuantityInvalid,

		/// A vote was received to nominate an address that is not a candidate validator.
		NoSuchValidator,
		// NoSuchReferendum,
		// ReferendumUnavailable,
	}

	impl<T: Config> Pallet<T> {
		pub fn controller_account_id() -> AccountIdOf<T> {
			T::PalletId::get().into_account_truncating()
		}
		// Into_sub_account_truncating does not result in distinct account IDs for the two
		// accounts, so to save much more involved coding we are just using 2 pallet IDs.
		pub fn stash_account_id() -> AccountIdOf<T> {
			T::PalletId2::get().into_account_truncating()
		}

		fn quantity_to_mint(
				amount_staked: BalanceTypeOf<T>, 
				total_stake_stash: BalanceTypeOf<T>, 
				derivative_total_issuance: BalanceTypeOf<T>) -> Result<BalanceTypeOf<T>, Error<T>> {

            let product = derivative_total_issuance.checked_mul(&amount_staked).ok_or_else(|| crate::Error::ExceededMaxStake)?;
            match product.checked_div(&total_stake_stash) {
                None => Ok(amount_staked), // If we start with nothing staked, we exchange at 1:1
                Some(quotient) => Ok(quotient)
            }
		}
	}
	
	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(block_number: T::BlockNumber) -> Weight {
			match Self::era_of_previous_block() {
				Some(era) => 
					// If the current era is different from the era of the previous block, set this 
					// block as era start and reset the previous era tracker
					if T::StakingInterface::current_era() > era { 
							EraOfPreviousBlock::<T>::put(T::StakingInterface::current_era());
							EraStartBlockNumber::<T>::put(block_number);
							10 // TODO Figure out what weights should be 
					} else {
						match Self::era_start_block() {
							// Otherwise, if we are the configured number of blocks from the beginning of the era
							// (not using safe math here because we are in control of the configuration and the block numbers)
							Some(era_start_block_number) => {
								if block_number == era_start_block_number + T::NominatorVotingPeriodBlocks::get() {

									// TODO Unlock all derivative tokens locked
									// TODO Tally all votes
									// TODO Adjust nominations. This is done via the nominate() endpoint on the StakingInterface
									// TODO Reinitialize storage for the next round of voting - both storage of locked tokens and storage of votes						

									100 // TODO Figure out what weights should be 
								} else {
									0 // Do nothing; TODO Figure out what weights should be 
								}
							},
							None => 0 // Do nothing; TODO Figure out what weights should be 
						}
					},
				// If we have no era for the previous block, initialize it to the current era so we can update it when it rolls over
				None => {
					EraOfPreviousBlock::<T>::put(T::StakingInterface::current_era());
					10 // TODO Figure out what weights should be 
				},
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		
		/// Submit an amount of the main token to be staked, resulting in a credit of a 
		/// certain amount of the derivative token to the staker's account. This amount
		/// is minted and is calculated using the ratio of the total issuance of the derivative
		/// token to the total amount currently in the pallet's stash account (the pooled stake).
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))] // TODO Benchmark eventually
		pub fn add_stake(origin: OriginFor<T>, amount: BalanceTypeOf<T>) -> DispatchResult {
			// Ensure signature
			let who = ensure_signed(origin)?;

			// Ensure stake is more than minimum.
			ensure!(amount > T::MinimumStake::get(), Error::<T>::InsufficientStake);

			let pot = Self::stash_account_id();
	        
			// Mint the equivalent in DerivativeCurrency
			let total_staked = T::MainCurrency::total_balance(&pot);
			let derivative_total_issuance = T::DerivativeCurrency::total_issuance();
			let derivative_quantity_to_mint = Self::quantity_to_mint(amount, total_staked, derivative_total_issuance)?;
			
            // Transfer the incoming stake to the pallet account
			// TODO Review notes and update this based on what Kian said about genesis etc.
			T::MainCurrency::transfer(&who, &pot, amount, ExistenceRequirement::KeepAlive)?;			
			T::DerivativeCurrency::deposit_creating(&who, derivative_quantity_to_mint);

			// Note: If we entirely fail to bond, the transaction will fail and the staker will not be staked.
			match T::StakingInterface::bond_extra(pot.clone(), amount) { // Confused: Shouldn't this be signed?

				// See if we got an error because the pot was not yet bonded, by trying to bond it. If that fails, we bail.
				Err(err) => T::StakingInterface::bond(pot.clone(), Self::controller_account_id(), amount, pot.clone())?,
				_ => ()
			}

			// Emit a StakeAdded event.
			Self::deposit_event(Event::StakeAdded(who, amount));

			Ok(())
		}

		/// Submit an amount of the derivative token to redeem for the main token. The derivative
		/// token is immediately burned; the amount is recorded in storage along with the era
		/// the underlying funds will be available. The DerivativeRedeemed event indicates 
		/// the amount and the era when the active bonded balance can be withdrawn.
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))] // TODO Benchmark eventually
		pub fn redeem_stake(origin: OriginFor<T>, amount: BalanceTypeOf<T>) -> DispatchResult {
			// Ensure signature
			let who = ensure_signed(origin)?;

			// Ensure that the staker has at least the specified quantity free in their derivative token balance
			ensure!(amount < T::DerivativeCurrency::free_balance(&who), Error::<T>::InsufficientFundsForRedemption);

			// The era when funds will be available. This addition is safe, since the era and duration come from the pallet.
			let era_available = T::StakingInterface::current_era() + T::StakingInterface::bonding_duration();

			// Add the amount and the era available to storage.
			// TODO Refactor this code!
			RedemptionsAwaitingWithdrawal::<T>::try_mutate(who, |maybe_existing_value| {
				match maybe_existing_value {
					Some(existing_map) => {
						match existing_map.get_mut(&era_available) {
							Some(existing_value) => 
								match existing_value.checked_add(&amount) {
									Some(total) => { *existing_value = total;  Ok(()) },
									None => Err(Error::<T>::VoteQuantityInvalid),
								},
							None => existing_map.try_insert(era_available, amount).map(
								|maybe_key_existed| () // We know the key isn't actually already there because we checked it above. qed 
							).map_err(
								|_| Error::<T>::TooManyRedemptionsAwaitingWithdrawal
							),
						}
					}
					None => {
						let new_map = BoundedBTreeMap::<EraIndex, BalanceTypeOf<T>, T::WithdrawalBound>::new();
						new_map.try_insert(era_available, amount).map(
							// We know the key isn't actually already there because we just created the map. qed 
							|_| RedemptionsAwaitingWithdrawal::<T>::insert(who, map),
						).map_err(
							// This Err case should never happen because this is a new map. But if it does it means the bounds have 
							// been exceeded. Seems like unreasonable work to create a new error for this misconfiguration case.
							|_| Error::<T>::TooManyRedemptionsAwaitingWithdrawal
						)
					},
				}
			})?;

			// Emit a DerivativeRedeemed event.
			Self::deposit_event(Event::DerivativeRedeemed(who, amount, era_available));

            Ok(())
		}

		/// Withdraw the amount of the main token that corresponds to the amount of derivative
		/// token available for withdrawal at the given era (which may be seen in the events 
		/// deposited by `redeem_stake`). The unbonding period has to have elapsed before withdrawal 
		/// may proceed. The amount redeemed corresponds to the current value of the quantity of 
		/// derivative token redeemed in the prior call to `redeem_stake`, not the value at the 
		/// time of redemption; it is important to note that the value may have been affected by 
		/// slashing or rewards in the meantime. 
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))] // TODO Benchmark eventually
		pub fn withdraw_stake(origin: OriginFor<T>, for_era: T::TransactionId) -> DispatchResult {
            Ok(())
		}

		// At the beginning of an era we allow holders of the derivative token to choose who the pallet
		// will nominate. A certain number of blocks of time is designated within which token holders may
		// vote for nominators by calling this dispatchable with a slate of 16 candidates and an amount
		// of derivative token backing each one, indicating the degree of support. These tokens are locked
		// during the voting period to prevent duplicate voting. At the end of the voting period the votes
		// are tallied. If the total number of votes is less than a majority of the total issuance of the
		// derivative token, the nominations of the previous era remain unchanged. Otherwise, the pallet
		// selects the sixteen top vote-getters and nominates them.
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))] // TODO Benchmark eventually
		pub fn nominate(origin: OriginFor<T>, 
						nominations: BoundedVec<(AccountIdOf<T>, BalanceTypeOf<T>), T::MaxValidatorNominees>) -> DispatchResult {
			// Ensure signature     
			let who = ensure_signed(origin)?;

			// TODO Determine if we are within n blocks of the beginning of an era, else reject with VoteUnauthorized
			// TODO Determine if the staker is voting for accounts that are not actually nominatable; if so reject with NoSuchValidator. 
			// 		This is important since without this check the endpoint could be spammed. Since we store the number of votes per 
			// 		validator this would eat up storage.

			// Determine whether the submission has enough tokens in their free balance to match the tokens voted. If not, reject.
            let maybe_total_voted = nominations.iter().fold(
					Some(Zero::zero()), 
					|accum: Option<BalanceTypeOf<T>>, (_, votes)| {
						match accum {
							Some(sum) => sum.checked_add(votes),
							None => None,
						}
					}
			); 
			let total_voted = maybe_total_voted.ok_or(Error::<T>::VoteQuantityInvalid)?;

			// TODO This doesn't seem to be working, or at least the test doesn't show that the free balance is affected.
			// Lock that quantity of derivative token in the origin's account
			T::DerivativeCurrency::set_lock(NOMINATION_LOCK_ID, &who, total_voted, WithdrawReasons::RESERVE);

			// Store the account and the number of tokens locked for that account (add to the total).
			NominationLocksStorage::<T>::try_mutate(who, |maybe_existing_value| {
				match maybe_existing_value {
					Some(prior_amount) => {
						match prior_amount.checked_add(&total_voted) {
							maybe_total @ Some(total) => { 
								*maybe_existing_value = maybe_total; 
								Ok(())
							 },
							 None => Err(Error::<T>::VoteQuantityInvalid),
						}
					}
					None => {
						*maybe_existing_value = Some(total_voted);
						Ok(())
					},
				}
			})?;
			// Store the nominations and the amounts (adding to the totals). This is bounded by the number of 
			// nominators, which we control through configuration.
			for (validator, votes) in nominations.iter() {
				NominationsStorage::<T>::try_mutate(validator, |maybe_existing_value| {
					match maybe_existing_value {
						Some(prior_amount) => {
							match prior_amount.checked_add(&votes) {
								maybe_total @ Some(total) => { 
									*maybe_existing_value = maybe_total; 
									Ok(())
								},
								None => Err(Error::<T>::ValidatorVoteQuantityInvalid),
							}
						},
						None => {
							*maybe_existing_value = Some(*votes);
							Ok(())
						}
					}
				})?;		
			}
            Ok(())
		}
	}
}
