// Copyright 2018 Commonwealth Labs, Inc.
// This file is part of Edgeware.

// Edgeware is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Edgeware is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Edgeware.  If not, see <http://www.gnu.org/licenses/>.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate serde;

// Needed for deriving `Serialize` and `Deserialize` for various types.
// We only implement the serde traits for std builds - they're unneeded
// in the wasm runtime.
#[cfg(feature = "std")]

extern crate parity_codec as codec;
extern crate substrate_primitives as primitives;
extern crate sr_std as rstd;
extern crate srml_support as runtime_support;
extern crate sr_primitives as runtime_primitives;
extern crate sr_io as runtime_io;

extern crate srml_balances as balances;
extern crate srml_system as system;
extern crate srml_session as session;

use democracy::{Approved, VoteThreshold};

use rstd::prelude::*;
use system::ensure_signed;
use runtime_support::{StorageValue, StorageMap};
use runtime_support::dispatch::Result;
use runtime_primitives::traits::{Zero, Hash};

/// Record indices.
pub type DepositIndex = u32;
pub type WithdrawIndex = u32;

pub trait Trait: balances::Trait + session::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

pub type LinkedProof = Vec<u8>;

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        /// The deposit function should always succeed (in order) a deposit transaction
        /// on the eligible blockchain that has an established two-way peg with Edgeware.
        /// This function can be triggered by the depositor or any bridge authority that
        /// sees the transaction first.
        pub fn deposit(origin, target: T::AccountId, transaction_hash: T::Hash, quantity: T::Balance) -> Result {
            let _sender = ensure_signed(origin)?;
            
            // Match on deposit records by the respective transaction hash on the eligible blockchain
            match <DepositOf<T>>::get(transaction_hash) {
                Some(_) => { return Err("Deposit should not exist")},
                None => {
                    // If sender is a bridge authority add them to the set of signers
                    let mut signers = vec![];
                    if <Authorities<T>>::get().iter().any(|a| a == &_sender) {
                        signers.push(_sender);
                    }

                    // Create new deposit record
                    let mut deposits = <Deposits<T>>::get();
                    deposits.push(transaction_hash);
                    <Deposits<T>>::put(deposits);

                    // Insert deposit record and send event
                    let index = Self::deposit_count();
                    <DepositCount<T>>::mutate(|i| *i += 1);
                    <DepositOf<T>>::insert(transaction_hash, (index, target.clone(), quantity, signers, false));
                    Self::deposit_event(RawEvent::Deposit(target, transaction_hash, quantity));
                },
            }

            Ok(())
        }

        /// The sign_deposit function should compile intentions (from sending tx) and
        /// check if a deposit proposal ever passes with each new valid signer.
        pub fn sign_deposit(origin, target: T::AccountId, transaction_hash: T::Hash, quantity: T::Balance) -> Result {
            let _sender = ensure_signed(origin)?;

            match <DepositOf<T>>::get(transaction_hash) {
                Some((inx, tgt, qty, signers, completed)) => {
                    // Ensure all parameters match for safety
                    ensure!(tgt == target.clone(), "Accounts do not match");
                    ensure!(qty == quantity, "Quantities don't match");
                    ensure!(!completed, "Transaction already completed");
                    // Ensure sender is a bridge authority
                    ensure!(Self::authorities().iter().any(|id| id == &_sender), "Invalid non-authority sender");
                    // Ensure senders can't sign twice
                    ensure!(!signers.iter().any(|id| id == &_sender), "Invalid duplicate signings");
                    // Add record update with new signer
                    let mut new_signers = signers.clone();
                    new_signers.push(_sender.clone());

                    // Check if we have reached enough signers for the deposit
                    // TODO: Ensure that checking balances is sufficient vs. finding explicit stake amounts
                    let stake_sum = new_signers.iter()
                        .map(|s| <balances::Module<T>>::total_balance(s))
                        .fold(Zero::zero(), |a,b| a + b);

                    // Check if we approve the proposal, if so, mark approved
                    let total_issuance = <balances::Module<T>>::total_issuance();
                    if VoteThreshold::SuperMajorityApprove.approved(stake_sum, total_issuance - stake_sum, total_issuance, total_issuance) {
                        <balances::Module<T>>::increase_free_balance_creating(&tgt, qty);
                        <DepositOf<T>>::insert(transaction_hash, (inx, tgt.clone(), qty, new_signers.clone(), true));
                        // TODO: fire event
                    } else {
                        <DepositOf<T>>::insert(transaction_hash, (inx, tgt.clone(), qty, new_signers.clone(), false));
                    }
                },
                None => { return Err("Invalid transaction hash") },
            }

            Ok(())
        }


        /// The withdraw function should precede (in order) a withdraw transaction on the
        /// eligible blockchain that has an established two-way peg with Edgeware. This
        /// function should only be called by a token holder interested in transferring
        /// native Edgeware tokens with Edgeware-compliant, non-native tokens like ERC20.
        pub fn withdraw(origin, quantity: T::Balance, signed_cross_chain_tx: Vec<u8>) -> Result {
            let _sender = ensure_signed(origin)?;

            let mut nonce = Self::withdraw_nonce_of(_sender.clone());
            let key = T::Hashing::hash_of(&(nonce, _sender.clone(), quantity));

            match <WithdrawOf<T>>::get(key) {
                Some(_) => { return Err("Withdraw already exists")},
                None => {
                    // If sender is a bridge authority add them to the set of signers
                    let mut signers = vec![];
                    if <Authorities<T>>::get().iter().any(|a| a == &_sender) {
                        signers.push((_sender.clone(), signed_cross_chain_tx));
                    }

                    // Ensure sender has enough balance to withdraw from
                    ensure!(<balances::Module<T>>::total_balance(&_sender) >= quantity, "Invalid balance for withdraw");

                    // Create new withdraw record
                    let mut withdraws = <Withdraws<T>>::get();
                    withdraws.push(key);
                    <Withdraws<T>>::put(withdraws);

                    // Insert withdraw record and send event
                    let index = Self::withdraw_count();
                    <WithdrawCount<T>>::mutate(|i| *i += 1);
                    <WithdrawOf<T>>::insert(key, (index, _sender.clone(), quantity, signers, false));
                    Self::deposit_event(RawEvent::Withdraw(_sender.clone(), quantity));
                },
            }

            nonce += 1;
            <WithdrawNonceOf<T>>::insert(_sender, nonce);
            Ok(())
        }

        /// The sign_withdraw function should compile signatures (from send tx) and
        /// check if a withdraw proposal ever passes with each new valid signer.
        pub fn sign_withdraw(origin, target: T::AccountId, record_hash: T::Hash, quantity: T::Balance, signed_cross_chain_tx: Vec<u8>) -> Result {
            let _sender = ensure_signed(origin)?;

            match <WithdrawOf<T>>::get(record_hash) {
                Some((inx, tgt, qty, signers, completed)) => {
                    // Ensure all parameters match for safety
                    ensure!(tgt == target.clone(), "Accounts do not match");
                    ensure!(qty == quantity, "Quantities don't match");
                    ensure!(!completed, "Transaction already completed");
                    // Ensure sender is a bridge authority if record exists
                    ensure!(Self::authorities().iter().any(|id| id == &_sender), "Invalid non-authority sender");
                    // Ensure senders can't sign twice
                    ensure!(!signers.iter().any(|s| s.0 == _sender), "Invalid duplicate signings");
                    // Add record update with new signer
                    let mut new_signers = signers;
                    new_signers.push((_sender, signed_cross_chain_tx));

                    // Check if we have reached enough signers for the withdrawal
                    // TODO: Ensure that checking balances is sufficient vs. finding explicit stake amounts
                    let stake_sum = new_signers.iter()
                        .map(|s| <balances::Module<T>>::total_balance(&s.0))
                        .fold(Zero::zero(), |a,b| a + b);

                    // Check if we approve the proposal
                    let total_issuance = <balances::Module<T>>::total_issuance();
                    if VoteThreshold::SuperMajorityApprove.approved(stake_sum, total_issuance - stake_sum, total_issuance, total_issuance) {
                        match <balances::Module<T>>::decrease_free_balance(&tgt, qty) {
                            Ok(_) => {
                                // TODO: do we still mark completed on error? or store a "failed" tx?
                                <WithdrawOf<T>>::insert(record_hash, (inx, tgt.clone(), qty, new_signers.clone(), true));
                                // TODO: fire event
                            },
                            Err(err) => { return Err(err); } // TODO test this?
                        };
                    } else {
                        <WithdrawOf<T>>::insert(record_hash, (inx, tgt.clone(), qty, new_signers.clone(), false));
                    }
                },
                None => { return Err("Invalid record hash") },
            }

            Ok(())
        }
    }
}

impl<T: Trait> Module<T> {
    pub fn withdraw_record_hash(index: usize) -> T::Hash {
        return <Withdraws<T>>::get()[index];
    }
}

impl<X, T> session::OnSessionChange<X> for Module<T> where T: Trait, T: session::Trait {
    fn on_session_change(_: X, _: bool) {
        let next_authorities = <session::Module<T>>::validators()
            .into_iter()
            .collect::<Vec<T::AccountId>>();

        // instant changes
        let last_authorities = <Authorities<T>>::get();
        if next_authorities != last_authorities {
            <Authorities<T>>::put(next_authorities.clone());
            Self::deposit_event(RawEvent::NewAuthorities(next_authorities));
        }
    }
}

/// An event in this module.
decl_event!(
    pub enum Event<T> where <T as system::Trait>::Hash,
                            <T as system::Trait>::AccountId,
                            <T as balances::Trait>::Balance {
        // Deposit event for an account, an eligible blockchain transaction hash, and quantity
        Deposit(AccountId, Hash, Balance),
        // Withdraw event for an account, and an amount
        Withdraw(AccountId, Balance),
        // New authority set has been applied.
        NewAuthorities(Vec<AccountId>),
    }
);

decl_storage! {
    trait Store for Module<T: Trait> as BridgeStorage {
        /// Mapping from an eligible blockchain by Hash(name) to the list of block headers
        /// TODO: V2 feature when we have stronger proofs of transfers
        pub BlockHeaders get(block_headers): map T::Hash => Vec<T::Hash>;

        /// The active set of bridge authorities who can sign off on requests
        pub Authorities get(authorities) config(): Vec<T::AccountId>;

        /// Number of deposits
        pub DepositCount get(deposit_count): u32;
        /// List of all deposit requests on Edgeware taken to be the transaction hash
        /// from the eligible blockchain
        pub Deposits get(deposits): Vec<T::Hash>;
        /// Mapping of deposit transaction hashes from the eligible blockchain to the
        /// deposit request record
        pub DepositOf get(deposit_of): map T::Hash => Option<(DepositIndex, T::AccountId, T::Balance, Vec<T::AccountId>, bool)>;
        
        /// Number of withdraws
        pub WithdrawCount get(withdraw_count): u32;
        /// List of all withdraw requests on Edgeware taken to be the unique hash created
        /// on Edgeware with the user's account, quantity, and nonce
        pub Withdraws get(withdraws): Vec<T::Hash>;
        /// Mapping of withdraw record hashes to the record
        pub WithdrawOf get(withdraw_of): map T::Hash => Option<(WithdrawIndex, T::AccountId, T::Balance, Vec<(T::AccountId, Vec<u8>)>, bool)>;
        /// Nonce for creating unique hashes per user per withdraw request
        pub WithdrawNonceOf get(withdraw_nonce_of): map T::AccountId => u32;
    }
}
