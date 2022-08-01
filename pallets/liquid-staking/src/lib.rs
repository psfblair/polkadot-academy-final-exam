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
	use frame_system::{Origin, pallet_prelude::*};
	use frame_support::{
            PalletId,
            pallet_prelude::*,
            traits::{
                Currency,
                LockableCurrency,
                tokens::ExistenceRequirement,
            }
        };
    use frame_support::sp_runtime::{
            traits::{AccountIdConversion, CheckedMul, CheckedDiv,}
        };
	use sp_staking::StakingInterface;
	
	use crate::BalanceTypeOf;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type MainCurrency: LockableCurrency<Self::AccountId>;
		type DerivativeCurrency: LockableCurrency<Self::AccountId, Balance = BalanceTypeOf<Self>>;
		type StakingInterface: sp_staking::StakingInterface;
       
        #[pallet::constant]
		type PalletId: Get<PalletId>;
        #[pallet::constant]
		type PalletId2: Get<PalletId>;
		#[pallet::constant]
		type MinimumStake: Get<BalanceTypeOf<Self>>;
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
		/// An account has staked a certain amount of the staking token with the pallet [account, amount]
		StakeAdded(T::AccountId, BalanceTypeOf<T>),

		/// The pot account was unable to bond some free funds [account, error]
		BondingFailed(T::AccountId, DispatchError),

		/// An account has redeemed a certain amount of the liquid token with the pallet [account, amount]
		DerivativeRedeemed(T::AccountId, BalanceTypeOf<T>),
		
		/// The staking tokens associated with the redeemed liquid tokens have been unbonded and 
		/// credited to the staker [account, amount]
		StakeReleased(T::AccountId, BalanceTypeOf<T>, BalanceTypeOf<T>),
		
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
		pub fn controller_account_id() -> T::AccountId {
			T::PalletId::get().into_account_truncating()
		}
		// Into_sub_account_truncating does not result in distinct account IDs for the two
		// accounts, so to save much more involved coding we are just using 2 pallet IDs.
		pub fn stash_account_id() -> T::AccountId {
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
		fn on_initialize(n: T::BlockNumber) -> Weight {
			0
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		
		/// TODO Add documentation!!!
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
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

			// Emit an event.
			Self::deposit_event(Event::StakeAdded(who, amount));

			// We assume 'pot' is registered as a staker at this point. We will bond all free funds 
			// in the stash account instead of bonding only the funds that just came in, in case we 
			// received a deposit in a prior transaction that was not able to be bonded. Also, we
			// won't fail this transaction if the funds came in but bonding fails; we will bond those
			// funds the next time new funds come in. Or if we need to be more vigorous, then we could
			// implement bonding of free funds in on_initialize, but doing that once every six seconds
			// seems expensive unless our liquid staking offering is extremely active.
			let not_yet_bonded = T::MainCurrency::free_balance(&pot);
			match T::StakingInterface::bond_extra(Origin::signed(&pot), not_yet_bonded) {
				// An unorthodox use of an event to signal an error condition. We don't want to fail the transaction
				// if we fail to bond at this point, but we do want some indication out in the world that bonding failed.
				// Success will result in a Bonded event from the staking pallet so we don't need an event for that.
				err @ Err(_) => Self::deposit_event(Event::BondingFailed(pot, err)),
				_ => ()
			}

			Ok(())
		}
	}
}
