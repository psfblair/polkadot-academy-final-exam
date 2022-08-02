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
	use frame_system::{pallet_prelude::*};
	use frame_support::{
            PalletId,
            pallet_prelude::*,
            traits::{
                Currency, LockableCurrency, WithdrawReasons, LockIdentifier,
                tokens::ExistenceRequirement,
            }
        };
    use frame_support::sp_runtime::{
            traits::{AccountIdConversion, CheckedAdd, CheckedMul, CheckedDiv, Zero}
        };
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
		#[pallet::constant]
		type MinimumStake: Get<BalanceTypeOf<Self>>;
		#[pallet::constant]
		type MaxValidatorNominees: Get<u8>; // Maybe this is like the 640k thing but I think voting for more than 255 validators would be ridiculous

		type StakingInterface: sp_staking::StakingInterface<
			Balance = BalanceTypeOf<Self>,
			AccountId = AccountIdOf<Self>,
		>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// The quantity of derivative tokens an account has locked during the voting period.
	///
	/// TWOX-QUESTION: `AccountId`s are crypto hashes anyway, so is this safe? What if an attacker knew a user's account ID and
	/// wanted to change their votes?  
	#[pallet::storage]
	#[pallet::getter(fn nomination_locks_for)]
	pub type NominationLocksStorage<T> = StorageMap<_, Twox64Concat, AccountIdOf<T>, BalanceTypeOf<T>>;

	/// The votes 
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
		DerivativeRedeemed(AccountIdOf<T>, BalanceTypeOf<T>, EraIndex, T::TransactionId),
		
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
		// VoteUnauthorized,
		// VoteQuantityInvalid,
		// NoSuchValidator,
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
		// 1. If the current era is different from the era of the previous block, set this 
		//    block as era start and reset the previous era tracker
		// 2. Otherwise: Determine if we are voting_window blocks from the beginning of the era. If not, do nothing.
		// 3. If so, then:
			// Unlock all derivative tokens locked
			// Tally all votes
			// Adjust nominations
			// Reinitialize storage for the next round of voting - both storage of locked tokens and storage of votes
		fn on_initialize(_n: T::BlockNumber) -> Weight {
			0
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

			// Emit an event.
			Self::deposit_event(Event::StakeAdded(who, amount));

			Ok(())
		}

		/// Submit an amount of the derivative token to redeem for the main token. The derivative
		/// token is immediately burned; the amount is recorded in storage along with the era
		/// the underlying funds will be available, and keyed by a transaction ID that can be
		/// used by the staker to retrieve the funds as of that era. The DerivativeRedeemed
		/// event indicates the era when the active bonded balance can be withdrawn and the 
		/// transaction ID that can be used to withdraw it.
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))] // TODO Benchmark eventually
		pub fn redeem_stake(origin: OriginFor<T>, amount: BalanceTypeOf<T>) -> DispatchResult {
            Ok(())
		}

		/// Withdraw the amount of the main token that corresponds to the amount of derivative
		/// token redeemed in the given transaction_id (which is obtained from the event deposited
		/// by `redeem_stake`). The unbonding period has to have elapsed before withdrawal may
		/// proceed. The amount redeemed corresponds to the current value of the quantity of 
		/// derivative token redeemed in the prior call to `redeem_stake`, not the value at the 
		/// time of redemption; it is important to note that the value may have been affected by 
		/// slashing or rewards in the meantime. 
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))] // TODO Benchmark eventually
		pub fn withdraw_stake(origin: OriginFor<T>, transaction_id: T::TransactionId) -> DispatchResult {
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

			// TODO Determine if we are within n blocks of the beginning of an era, else reject 
			// TODO Determine if the submission is voting for accounts that are not actually nominatable.

			// Determine whether the submission has enough tokens in their free balance to match the tokens voted. If not, reject.
            let total_voted = nominations.iter().fold(Zero::zero(), |accum: BalanceTypeOf<T>, (_, votes)| accum.checked_add(votes))?; 

			// Lock that quantity of derivative token in the origin's account
			T::DerivativeCurrency::set_lock(NOMINATION_LOCK_ID, &who, total_voted, WithdrawReasons::RESERVE);

			// Store the account and the number of tokens locked for that account (add to the total)
			NominationLocksStorage::<T>::try_mutate(who, |maybe_value| {
				match maybe_value {
					Some(prior_amount) => *maybe_value = Some(prior_amount.checked_add(total_voted)?),
					None => *maybe_value = Some(total_voted),
				}
			})?;
			// Store the nominations and the amounts (adding to the totals)
			for (validator, votes) in nominations.iter() {
				NominationsStorage::<T>::try_mutate(validator, |maybe_value| {
					match maybe_value {
						Some(prior_amount) => *maybe_value = Some(prior_amount.checked_add(votes)?),
						None => *maybe_value = Some(votes),
					}
				})?;		
			}
            Ok(())
		}
	}
}
