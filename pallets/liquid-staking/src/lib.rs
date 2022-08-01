#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

use frame_support::traits::Currency; 
type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
// This type will be used for both balances pallets; they are tied together below in Config.
pub type BalanceTypeOf<T> = <<T as Config>::MainCurrency as Currency<AccountIdOf<T>>>::Balance; 

#[frame_support::pallet]
pub mod pallet {
	use frame_system::pallet_prelude::*;
	use frame_support::{
            PalletId,
            pallet_prelude::*,
            traits::{
                Currency,
                LockableCurrency,
                tokens::ExistenceRequirement,
            }
        };
    use frame_support::sp_runtime::traits::AccountIdConversion;
	use crate::BalanceTypeOf;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type MainCurrency: LockableCurrency<Self::AccountId>;
		type DerivativeCurrency: LockableCurrency<Self::AccountId, Balance = BalanceTypeOf<Self>>;
       
        #[pallet::constant]
		type PalletId: Get<PalletId>;
        #[pallet::constant]
		type PalletId2: Get<PalletId>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	// The pallet's runtime storage items.
	// https://docs.substrate.io/v3/runtime/storage
	#[pallet::storage]
	#[pallet::getter(fn something)]
	// Learn more about declaring storage items:
	// https://docs.substrate.io/v3/runtime/storage#declaring-storage-items
	pub type Something<T> = StorageValue<_, u32>;

	// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// An account has staked a certain amount of the staking token with the pallet [amount, account]
		StakeAdded(BalanceTypeOf<T>, T::AccountId),

		/// An account has redeemed a certain amount of the liquid token with the pallet [amount, account]
		DerivativeRedeemed(BalanceTypeOf<T>, T::AccountId),
		
		/// The staking tokens associated with the redeemed liquid tokens have been unbonded and 
		/// credited to the staker [amount, account]
		StakeReleased(BalanceTypeOf<T>, BalanceTypeOf<T>, T::AccountId),
		
		// ValidatorVoteSubmitted,
		// ReferendumVoteSubmitted,
	}

	// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::error]
	pub enum Error<T> {
		/// Amount staked is less than minimum account balance
		InsufficientStake,
		// VoteUnauthorized,
		// VoteQuantityInvalid,
		// NoSuchValidator,
		// NoSuchReferendum,
		// ReferendumUnavailable,
	}

	impl<T: Config> Pallet<T> {
		pub fn controller_account_id() -> T::AccountId {
			T::PalletId::get().into_account_truncating()
		}
		// Into_sub_account_truncating did not result in distinct account IDs for the 
		// two accounts.
		pub fn stash_account_id() -> T::AccountId {
			T::PalletId2::get().into_account_truncating()
		}
	}

	
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		
		/// TODO Add documentation!!!
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn add_stake(origin: OriginFor<T>, amount: BalanceTypeOf<T>) -> DispatchResult {
			// Ensure signature
			let who = ensure_signed(origin)?;

			// Ensure stake is more than minimum balance. If we want to require more we would do it here.
			ensure!(amount > T::MainCurrency::minimum_balance(), Error::<T>::InsufficientStake);

			// Transfer the incoming stake to the pallet account
			// TODO Review notes and update this based on what Kian said about genesis etc.
			let pot = Self::stash_account_id();
			T::MainCurrency::transfer(&who, &pot, amount, ExistenceRequirement::KeepAlive)?;			
	
			// Mint the equivalent in DerivativeCurrency
			// let derivative_amount = BalanceTypeOf<T> = ...;
			// T::DerivativeCurrency::deposit_creating(who, liquid_derivative);

			// We assume 'pot' is registered as a staker at this point
			// We immediately stake the incoming amount. Later we can worry about the voting
			// staking::bond_extra(pot). 

			// Emit an event.
			Self::deposit_event(Event::StakeAdded(amount, who));

			Ok(())
		}
/*
		/// An example dispatchable that may throw a custom error.
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn cause_error(origin: OriginFor<T>) -> DispatchResult {
			let _who = ensure_signed(origin)?;

			// Read a value from storage.
			match <Something<T>>::get() {
				// Return an error if the value has not been set.
				None => return Err(Error::<T>::NoneValue.into()),
				Some(old) => {
					// Increment the value read from storage; will error in the event of overflow.
					let new = old.checked_add(1).ok_or(Error::<T>::StorageOverflow)?;
					// Update the value in storage with the incremented result.
					<Something<T>>::put(new);
					Ok(())
				},
			}
		}
*/		
	}
}
