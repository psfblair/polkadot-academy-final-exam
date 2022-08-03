# Liquid Staking Pallet

## Overview
This pallet is designed to let users stake funds in a token 
called the `main currency` and receive back a token called the
`derivative currency`. The derivative is liquid and can be 
exchanged like a normal token. The staked funds are 
controlled by the pallet to nominate validators; 
however, the choice of nominators is made by a vote of
the users via their token. This behavior combines
behavior built out separately in the `nomination-pools`
and `elections-phragmen` pallets, but it is part of the 
assignment to implement them in this pallet.

Holders of the derivative currency can redeem their tokens
for the main currency in two phases. First, they signal
that they wish to redeem. Their derivative token is then
burned. In the second phase (i.e., after the unbonding
period required by the staking pallet) users may issue
a second request to withdraw their share of the main 
currency. Note that this share is calculated at the time
funds are withdrawn and not at redemption time, because
staked funds are subject to slashing during the unbonding
period, so the exchange rate of the derivative token to
the main token may change during the bonding period. An
event issued at redemption time indicates the staking
era when funds will be withdrawable.

The exchange rate between the main currency and derivative
currency is determined by the ratio of total issuance of
the derivative currency compared with the total main
currency staked. The exchange rate starts at 1:1. This 
formula equitably distributes rewards and slashing across
users.

Holders of the derivative token may vote on nominators.
During the voting period the derivative tokens they 
commit to the vote are locked, to prevent voting by
multiple parties using the same token. Voting takes
place for a configurable period of blocks from the
beginning of the staking era. Token holders may vote
up to a configurable maximum number of candidates, 
placing a certain amount of token on each; token
holders need not use all their tokens for voting
and may vote for fewer than the maximum number of
candidates.

After the vote, the pallet tallies votes and nominates
the top N validator candidates in proportion to their
share of the votes for the top N candidates. (This is
not yet implemented.)

Also not yet implemented is the ability for holders of
the derivative token to vote on referenda issued for
holders of the main token. To do so, a staker would
submit a request to vote on a particular referendum.
The pool would then delegate to that staker a certain
number of votes in proportion to the amount of
derivative token committed to the election (based on
the exchange rate). Again, the derivative token would
have to be locked during the voting period in order 
to prevent double-voting.

## Implementation

The pallet currently implements the following dispatchable
calls:
```
add_stake(origin: OriginFor<T>, amount: BalanceTypeOf<T>) -> DispatchResult
```
User submits `amount` to be staked in a signed transaction
from `origin`.

```
pub fn redeem_stake(origin: OriginFor<T>, amount: BalanceTypeOf<T>) -> DispatchResult
```
This endpoint allows the user at `origin` to designate
`amount` of the derivative currency for redemption. A 
`DerivativeRedeemed` event indicates the era at the end of 
the bonding period.

```
pub fn nominate(origin: OriginFor<T>, 
						nominations: BoundedVec<(AccountIdOf<T>, BalanceTypeOf<T>) -> DispatchResult
```
User at `origin` submits a signed transaction of a bounded
quantity of tuples of `(accountId, amount)` indicating how much 
derivative token `amount` to commit to backing each validator 
specified by `accountId`. Currently there is no implementation
for limiting the account IDs receiving votes to the 
candidate validator pool.

### Tests

Testing is fairly extensive; unimplemented tests are also
shown in order to give an idea of what would be tested in
the ideal case. Make sure to check out the mock implementation
of `StakingInterface` in `mocks.rs`, which was lifted from
the `nomination-pools` pallet (and tweaked a little bit).

## Unimplemented endpoints

The following additional dispatchables are envisioned, but
not yet implemented:

```
pub fn withdraw_stake(origin: OriginFor<T>) -> DispatchResult
```
This endpoint would permit users to withdraw main currency
from the stake after the bonding period was over. Any
funds available for withdrawal at the given era are
transferred to the user.

```
pub fn referendum_vote(origin: OriginFor<T>, referendum_vote: ReferendumIndex, commitment: BalanceTypeOf<T>) -> DispatchResult) 
```
This endpoint would allow users to request delegated votes
from the pool for a referendum `ReferendumIndex` in proportion
to the amount of derivative currency assigned by `commitment`.

## How to see it work

Code at the level of the runtime configuration was not
fully implemented and may not compile. Please compile 
and run `cargo test` inside the `pallet/liquid-staking`
directory in order to validate what is working.

Note that there are certain tests marked as `#[ignore]`
which I was unable to get to pass. Currently there is
only one test which attempts to verify that the
derivative token is locked when a user submits 
nominations for validators. It is unclear why that
test is failing; it appears to be calling the right
API in the correct way and with the correct parameters.

