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
                Currency,
                LockableCurrency,
                tokens::ExistenceRequirement,
            }
        };
    use frame_support::sp_runtime::{
            traits::{AccountIdConversion, CheckedMul, CheckedDiv,}
        };
	use sp_staking::StakingInterface;
	
	use crate::{AccountIdOf, BalanceTypeOf};

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type MainCurrency: LockableCurrency<AccountIdOf<Self>>;
		type DerivativeCurrency: LockableCurrency<AccountIdOf<Self>, Balance = BalanceTypeOf<Self>>;
		type TransactionId: Hash;
       
        #[pallet::constant]
		type PalletId: Get<PalletId>;
        #[pallet::constant]
		type PalletId2: Get<PalletId>;
		#[pallet::constant]
		type MinimumStake: Get<BalanceTypeOf<Self>>;

		type StakingInterface: sp_staking::StakingInterface<
			Balance = BalanceTypeOf<Self>,
			AccountId = AccountIdOf<Self>,
		>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn something)]
	pub type Something<T> = StorageValue<_, u32>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// An account has staked a certain amount of the staking token with the pallet [account, amount]
		StakeAdded(AccountIdOf<T>, BalanceTypeOf<T>),

		/// The pot account was unable to bond some free funds [account, error]
		BondingFailed(AccountIdOf<T>, DispatchError),

		/// An account has redeemed a certain amount of the derivative token with the pallet. The underlying 
		/// main token may be claimed on the era specified in the event. [account, amount, era, transaction_id]
		DerivativeRedeemed(AccountIdOf<T>, BalanceTypeOf<T>, EraIndex, TransactionId),
		
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
		/// Establish the pallet as a nominator if it is not already one. We are taking into account that this
		/// pallet might not have been in use at genesis, so whenever there is a runtime upgrade we check to see
		/// if we are a nominator and if not, make it so.
		fn on_runtime_upgrade() -> Weight {
			let stash_account_id = Self::stash_account_id();
			// nominations returns an option with the nominations of a stash, if they are a nominator, None otherwise.
			let _ = T::StakingInterface::nominations(stash_account_id).unwrap_or_else(|| 
				// At least the minimum bond amount must be present in the stash account for this pallet to work.
				if T::MainCurrency::free_balance(&stash_account_id); T::StakingInterface::minimum_bond() {
					T::StakingInterface::bond(
						stash_account_id,
						Self::controller_account_id(),
						T::StakingInterface::minimum_bond(), 
						stash_account_id
					)
				}
			);
			200 // TODO Not sure what this should be...
		}

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
			match T::StakingInterface::bond_extra(pot.clone(), not_yet_bonded) { // Confused: Shouldn't this be signed?
				// An unorthodox use of an event to signal an error condition. We don't want to fail the transaction
				// if we fail to bond at this point, but we do want some indication out in the world that bonding failed.
				// Success will result in a Bonded event from the staking pallet so we don't need an event for that.
				Err(err) => Self::deposit_event(Event::BondingFailed(pot, err)),
				_ => ()
			}

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

		}

	}
}
