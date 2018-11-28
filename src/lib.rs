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
#[macro_use]
extern crate serde_derive;
#[cfg(test)]
#[macro_use]
extern crate hex_literal;
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
extern crate srml_grandpa as grandpa;
extern crate srml_democracy as democracy;

// use council::{voting, motions, seats};

// use rstd::prelude::*;
// use runtime_support::dispatch::Result;
// use primitives::ed25519;

pub mod bridge;
use bridge::{Module, Trait, RawEvent};

// Tests for Bridge Module
#[cfg(test)]
mod tests {
    use super::*;
    use grandpa::RawLog;
    use primitives::{H256, Blake2Hasher};

    use runtime_primitives::{BuildStorage};
    use runtime_primitives::traits::{BlakeTwo256};
    use runtime_primitives::testing::{Digest, DigestItem, Header};
    use runtime_primitives::generic::DigestItem as GenDigestItem;

    impl From<RawLog<u64, u64>> for DigestItem {
        fn from(log: RawLog<u64, u64>) -> DigestItem {
            GenDigestItem::Other(log.encode())
        }
    }

    impl_outer_origin! {
        pub enum Origin for Test {}
    }

    impl_outer_event! {
        pub enum Event for Test {
            bridge<T>,
            balances<T>,
            grandpa<T>,
        }
    }

    impl_outer_dispatch! {
        pub enum Call for Test where origin: Origin {
            balances::Balances,
            grandpa::Grandpa,
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
    impl grandpa::Trait for Test {
        type Log = DigestItem;
        type SessionKey = u64;
        type Event = Event;
    }
    impl Trait for Test {
        type Event = Event;
    }

    pub type System = system::Module<Test>;
    pub type Balances = balances::Module<Test>;
    pub type Grandpa = grandpa::Module<Test>;
    pub type Bridge = Module<Test>;

    // This function basically just builds a genesis storage key/value store according to
    // our desired mockup.
    fn new_test_ext() -> sr_io::TestExternalities<Blake2Hasher> {
        let mut t = system::GenesisConfig::<Test>::default().build_storage().unwrap().0;
        // // We use default for brevity, but you can configure as desired if needed.
        t.extend(bridge::GenesisConfig::<Test>{
            authorities: vec![1, 2, 3],
            _genesis_phantom_data: Default::default(),
        }.build_storage().unwrap().0);
        t.into()
    }
}
