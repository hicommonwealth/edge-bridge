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
// #[cfg(feature = "std")]
// #[macro_use]
// extern crate serde_derive;
// #[cfg(test)]
// #[macro_use]
// extern crate hex_literal;
#[macro_use] extern crate parity_codec_derive;
#[macro_use] extern crate srml_support;


extern crate parity_codec as codec;
extern crate substrate_primitives as primitives;
#[cfg_attr(not(feature = "std"), macro_use)]
extern crate sr_std as rstd;
extern crate srml_support as runtime_support;
extern crate sr_primitives as runtime_primitives;
extern crate sr_io as runtime_io;

extern crate srml_system as system;
extern crate srml_balances as balances;
extern crate srml_session as session;
extern crate srml_timestamp as timestamp;
extern crate srml_democracy as democracy;
extern crate srml_consensus as consensus;

// use council::{voting, motions, seats};

use runtime_support::dispatch::Result;
// use primitives::ed25519;

pub mod bridge;
pub use bridge::{Module, Trait, RawEvent, Event};

// Tests for Bridge Module
#[cfg(test)]
mod tests {
    use super::*;
    use runtime_io::with_externalities;
    use system::{EventRecord, Phase};
    use primitives::{H256, Blake2Hasher, Hasher};
    use runtime_primitives::{BuildStorage};
    use runtime_primitives::traits::{BlakeTwo256, Identity};
    use runtime_primitives::testing::{Digest, DigestItem, Header};

    impl_outer_origin! {
        pub enum Origin for Test {}
    }

    impl_outer_event! {
        pub enum Event for Test {
            bridge<T>,
            balances<T>,
            session<T>,
        }
    }

    impl_outer_dispatch! {
        pub enum Call for Test where origin: Origin {
            balances::Balances,
            session::Session,
        }
    }

    // For testing the module, we construct most of a mock runtime. This means
    // first constructing a configuration type (`Test`) which `impl`s each of the
    // configuration traits of modules we want to use.
    #[derive(Clone, PartialEq, Eq, Debug, Decode, Encode)]
    pub struct Test;
    impl system::Trait for Test {
        type Origin = Origin;
        type Index = u64;
        type BlockNumber = u64;
        type Hash = H256;
        type Hashing = BlakeTwo256;
        type Digest = Digest;
        type AccountId = u64;
        type Header = Header;
        type Event = Event;
        type Log = DigestItem;
    }
    impl balances::Trait for Test {
        type Balance = u64;
        type AccountIndex = u64;
        type OnFreeBalanceZero = ();
        type EnsureAccountLiquid = ();
        type Event = Event;
    }
    impl consensus::Trait for Test {
        const NOTE_OFFLINE_POSITION: u32 = 1;
        type Log = DigestItem;
        type SessionKey = u64;
        type OnOfflineValidator = ();
    }
    impl timestamp::Trait for Test {
        const TIMESTAMP_SET_POSITION: u32 = 0;
        type Moment = u64;
    }
    impl session::Trait for Test {
        type ConvertAccountIdToSessionKey = Identity;
        type OnSessionChange = Bridge;
        type Event = Event;
    }
    impl Trait for Test {
        type Event = Event;
    }

    pub type System = system::Module<Test>;
    pub type Balances = balances::Module<Test>;
    pub type Session = session::Module<Test>;
    pub type Bridge = Module<Test>;

    // This function basically just builds a genesis storage key/value store according to
    // our desired mockup.
    fn new_test_ext() -> sr_io::TestExternalities<Blake2Hasher> {
        let mut t = system::GenesisConfig::<Test>::default().build_storage().unwrap().0;
        // // We use default for brevity, but you can configure as desired if needed.
        t.extend(balances::GenesisConfig::<Test>{
            balances: [(1, 10000), (2, 10000), (3, 10000), (4, 100), (5, 100), (6, 100)].to_vec(),
            transaction_base_fee: 0,
            transaction_byte_fee: 0,
            existential_deposit: 0,
            transfer_fee: 0,
            creation_fee: 0,
            reclaim_rebate: 0,
            _genesis_phantom_data: Default::default(),
        }.build_storage().unwrap().0);
        t.extend(bridge::GenesisConfig::<Test>{
            authorities: vec![1, 2, 3],
            _genesis_phantom_data: Default::default(),
        }.build_storage().unwrap().0);
        t.into()
    }

    fn deposit(who: u64, target: u64, transaction_hash: H256, quantity: u64) -> super::Result {
        Bridge::deposit(Origin::signed(who), target, transaction_hash, quantity)
    }

    fn sign_deposit(who: u64, target: u64, transaction_hash: H256, quantity: u64) -> super::Result {
        Bridge::sign_deposit(Origin::signed(who), target, transaction_hash, quantity)
    }

    fn withdraw(who: u64, quantity: u64, signed_cross_chain_tx: &[u8]) -> super::Result {
        Bridge::withdraw(Origin::signed(who), quantity, signed_cross_chain_tx.to_vec())
    }

    fn sign_withdraw(who: u64, target: u64, record_hash: H256, quantity: u64, signed_cross_chain_tx: &[u8]) -> super::Result {
        Bridge::sign_withdraw(Origin::signed(who), target, record_hash, quantity, signed_cross_chain_tx.to_vec())
    }

    #[test]
    fn params_should_be_set_correctly() {
        with_externalities(&mut new_test_ext(), || {
            System::set_block_number(1);
            assert_eq!(Balances::total_balance(&1), 10000);
            assert_eq!(Balances::total_balance(&2), 10000);
            assert_eq!(Balances::total_balance(&3), 10000);
            assert_eq!(Balances::total_balance(&4), 100);
            assert_eq!(Balances::total_balance(&5), 100);
            assert_eq!(Balances::total_balance(&6), 100);
            assert_eq!(Bridge::authorities(), vec![1, 2, 3]);
        });
    }

    #[test]
    fn deposit_as_a_function_should_work() {
        with_externalities(&mut new_test_ext(), || {
            System::set_block_number(1);
            let hash = Blake2Hasher::hash(b"a sends money to b");
            assert_ok!(deposit(5, 5, hash, 10));
            assert_eq!(System::events(), vec![
                EventRecord {
                    phase: Phase::ApplyExtrinsic(0),
                    event: Event::bridge(RawEvent::Deposit(5, hash, 10)),
                }]
            );
        });
    }

    #[test]
    fn deposit_with_same_tx_twice_should_not_work() {
        with_externalities(&mut new_test_ext(), || {
            System::set_block_number(1);
            let hash = Blake2Hasher::hash(b"a sends money to b");
            let quantity = 10;
            assert_ok!(deposit(5, 5, hash, quantity));
            assert_eq!(deposit(5, 5, hash, quantity), Err("Deposit should not exist"));
        });
    }

    #[test]
    fn sign_deposit_as_bridge_authority_should_work() {
        with_externalities(&mut new_test_ext(), || {
            System::set_block_number(1);
            let hash = Blake2Hasher::hash(b"a sends money to b");
            let quantity = 10;
            assert_ok!(deposit(5, 5, hash, quantity));
            assert_ok!(sign_deposit(1, 5, hash, quantity));
        });
    }

    #[test]
    fn sign_deposit_supermajority_should_work() {
        with_externalities(&mut new_test_ext(), || {
            System::set_block_number(1);
            let hash = Blake2Hasher::hash(b"a sends money to b");
            let quantity = 10;
            assert_ok!(deposit(5, 5, hash, quantity));
            assert_eq!(Balances::total_balance(&5), 100);
            assert_ok!(sign_deposit(1, 5, hash, quantity));
            assert_eq!(Balances::total_balance(&5), 100);
            assert_ok!(sign_deposit(2, 5, hash, quantity));
            assert_eq!(Balances::total_balance(&5), 110);
        });
    }

    #[test]
    fn sign_deposit_once_complete_should_not_work() {
        with_externalities(&mut new_test_ext(), || {
            System::set_block_number(1);
            let hash = Blake2Hasher::hash(b"a sends money to b");
            let quantity = 10;
            assert_ok!(deposit(5, 5, hash, quantity));
            assert_eq!(Balances::total_balance(&5), 100);
            assert_ok!(sign_deposit(1, 5, hash, quantity));
            assert_eq!(Balances::total_balance(&5), 100);
            assert_ok!(sign_deposit(2, 5, hash, quantity));
            assert_eq!(Balances::total_balance(&5), 110);
            assert_eq!(sign_deposit(3, 5, hash, quantity),
                       Err("Transaction already completed"));
        });
    }
    
    #[test]
    fn sign_non_existent_deposit_as_bridge_authority_should_not_work() {
        with_externalities(&mut new_test_ext(), || {
            System::set_block_number(1);
            let hash = Blake2Hasher::hash(b"a sends money to b");
            let quantity = 10;
            assert_eq!(sign_deposit(1, 5, hash, quantity), Err("Invalid transaction hash"));
        });
    }

    #[test]
    fn sign_deposit_with_wrong_quantity_should_not_work() {
        with_externalities(&mut new_test_ext(), || {
            System::set_block_number(1);
            let hash = Blake2Hasher::hash(b"a sends money to b");
            let quantity = 10;
            assert_ok!(deposit(5, 5, hash, quantity));
            assert_eq!(sign_deposit(1, 5, hash, quantity - 1), Err("Quantities don't match"));
        });
    }

    #[test]
    fn sign_deposit_with_wrong_target_should_not_work() {
        with_externalities(&mut new_test_ext(), || {
            System::set_block_number(1);
            let hash = Blake2Hasher::hash(b"a sends money to b");
            let quantity = 10;
            assert_ok!(deposit(5, 5, hash, quantity));
            assert_eq!(sign_deposit(1, 4, hash, quantity), Err("Accounts do not match"));
        });
    }

    #[test]
    fn sign_deposit_as_non_authority_should_not_work() {
        with_externalities(&mut new_test_ext(), || {
            System::set_block_number(1);
            let hash = Blake2Hasher::hash(b"a sends money to b");
            let quantity = 10;
            assert_ok!(deposit(5, 5, hash, quantity));
            assert_eq!(sign_deposit(5, 5, hash, quantity), Err("Invalid non-authority sender"));
        });
    }

    #[test]
    fn sign_deposit_twice_should_not_work() {
        with_externalities(&mut new_test_ext(), || {
            System::set_block_number(1);
            let hash = Blake2Hasher::hash(b"a sends money to b");
            let quantity = 10;
            assert_ok!(deposit(5, 5, hash, quantity));
            assert_ok!(sign_deposit(1, 5, hash, quantity));
            assert_eq!(sign_deposit(1, 5, hash, quantity), Err("Invalid duplicate signings"))
        });
    }

    #[test]
    fn withdraw_as_a_function_should_work() {
        with_externalities(&mut new_test_ext(), || {
            System::set_block_number(1);
            let signed_tx = b"a sends money to b on Ethereum";
            assert_ok!(withdraw(5, 10, signed_tx));
            assert_eq!(System::events(), vec![
                EventRecord {
                    phase: Phase::ApplyExtrinsic(0),
                    event: Event::bridge(RawEvent::Withdraw(5, 10)),
                }]
            );
        });
    }

    #[test]
    fn withdraw_with_not_enough_balance_not_work() {
        with_externalities(&mut new_test_ext(), || {
            System::set_block_number(1);
            let signed_tx = b"a sends money to b on Ethereum";
            assert_eq!(Balances::total_balance(&4), 100);
            assert_eq!(withdraw(4, 101, signed_tx), Err("Invalid balance for withdraw"));
        });
    }

    #[test]
    fn sign_withdraw_supermajority_should_work() {
        with_externalities(&mut new_test_ext(), || {
            System::set_block_number(1);
            let cross_chain_proof = b"a sent b 1 ETH";
            let quantity = 10;
            assert_ok!(withdraw(5, quantity, cross_chain_proof));
            assert_eq!(Balances::total_balance(&5), 100);
            let hash = Bridge::withdraw_record_hash(0);
            assert_ok!(sign_withdraw(1, 5, hash, quantity, cross_chain_proof));
            assert_eq!(Balances::total_balance(&5), 100);
            assert_ok!(sign_withdraw(2, 5, hash, quantity, cross_chain_proof));
            assert_eq!(Balances::total_balance(&5), 100 - quantity);
        });
    }

    #[test]
    fn sign_withdraw_once_complete_should_not_work() {
        with_externalities(&mut new_test_ext(), || {
            System::set_block_number(1);
            let cross_chain_proof = b"a sent b 1 ETH";
            let quantity = 10;
            assert_ok!(withdraw(5, quantity, cross_chain_proof));
            assert_eq!(Balances::total_balance(&5), 100);
            let hash = Bridge::withdraw_record_hash(0);
            assert_ok!(sign_withdraw(1, 5, hash, quantity, cross_chain_proof));
            assert_eq!(Balances::total_balance(&5), 100);
            assert_ok!(sign_withdraw(2, 5, hash, quantity, cross_chain_proof));
            assert_eq!(Balances::total_balance(&5), 100 - quantity);
            assert_eq!(sign_withdraw(3, 5, hash, quantity, cross_chain_proof),
                       Err("Transaction already completed"))
        });
    }
    
    #[test]
    fn sign_withdraw_with_wrong_quantity_should_not_work() {
        with_externalities(&mut new_test_ext(), || {
            System::set_block_number(1);
            let cross_chain_proof = b"a sent b 1 ETH";
            let quantity = 10;
            assert_ok!(withdraw(5, quantity, cross_chain_proof));
            let hash = Bridge::withdraw_record_hash(0);
            assert_eq!(sign_withdraw(1, 5, hash, quantity - 1, cross_chain_proof), Err("Quantities don't match"));
        });
    }

    #[test]
    fn sign_withdraw_with_wrong_target_should_not_work() {
        with_externalities(&mut new_test_ext(), || {
            System::set_block_number(1);
            let cross_chain_proof = b"a sent b 1 ETH";
            let quantity = 10;
            assert_ok!(withdraw(5, quantity, cross_chain_proof));
            let hash = Bridge::withdraw_record_hash(0);
            assert_eq!(sign_withdraw(1, 4, hash, quantity, cross_chain_proof), Err("Accounts do not match"));
        });
    }

    #[test]
    fn sign_withdraw_as_non_authority_should_not_work() {
        with_externalities(&mut new_test_ext(), || {
            System::set_block_number(1);
            let cross_chain_proof = b"a sent b 1 ETH";
            let quantity = 10;
            assert_ok!(withdraw(5, quantity, cross_chain_proof));
            let hash = Bridge::withdraw_record_hash(0);
            assert_eq!(sign_withdraw(5, 5, hash, quantity, cross_chain_proof), Err("Invalid non-authority sender"));
        });
    }

    #[test]
    fn sign_withdraw_twice_should_not_work() {
        with_externalities(&mut new_test_ext(), || {
            System::set_block_number(1);
            let cross_chain_proof = b"a sent b 1 ETH";
            let quantity = 10;
            assert_ok!(withdraw(5, quantity, cross_chain_proof));
            let hash = Bridge::withdraw_record_hash(0);
            assert_ok!(sign_withdraw(1, 5, hash, quantity, cross_chain_proof));
            assert_eq!(sign_withdraw(1, 5, hash, quantity, cross_chain_proof), Err("Invalid duplicate signings"))
        });
    }

    #[test]
    fn sign_withdraw_with_non_existent_record_hash_should_not_work() {
        with_externalities(&mut new_test_ext(), || {
            System::set_block_number(1);
            let cross_chain_proof = b"a sent b 1 ETH";
            let quantity = 10;
            let hash = Blake2Hasher::hash(b"drew stone was here");
            assert_eq!(sign_withdraw(1, 4, hash, quantity, cross_chain_proof), Err("Invalid record hash"));
        });
    }
}
