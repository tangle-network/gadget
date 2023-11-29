// Copyright 2022 Webb Technologies Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![warn(missing_docs)]

use sp_application_crypto::sp_core::{ecdsa, keccak_256, Pair};

use dkg_runtime_primitives::crypto;

/// Set of test accounts using [`crypto`] types.
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::IntoEnumIterator)]
pub enum Keyring {
    Alice,
    Bob,
    Charlie,
    Dave,
    Eve,
    Ferdie,
    One,
    Two,
    Custom(u128),
}

impl Keyring {
    /// Sign `msg`.
    pub fn sign(self, msg: &[u8]) -> crypto::Signature {
        let msg = keccak_256(msg);
        ecdsa::Pair::from(self).sign_prehashed(&msg).into()
    }

    /// Return key pair.
    pub fn pair(self) -> crypto::Pair {
        Pair::from_string(self.to_seed().as_str(), None)
            .unwrap_or_else(|_| panic!("Could not find pair"))
            .into()
    }

    /// Return public key.
    pub fn public(self) -> crypto::Public {
        self.pair().public()
    }

    /// Return seed string.
    pub fn to_seed(self) -> String {
        format!("//{self:?}")
    }

    /// Iterator over all test accounts
    pub fn iter() -> impl (Iterator<Item = Keyring>) {
        <Self as strum::IntoEnumIterator>::iter()
    }
}

impl From<Keyring> for crypto::Pair {
    fn from(k: Keyring) -> Self {
        k.pair()
    }
}

impl From<Keyring> for ecdsa::Pair {
    fn from(k: Keyring) -> Self {
        k.pair().into()
    }
}

#[cfg(test)]
mod tests {
    use super::Keyring;
    use dkg_runtime_primitives::crypto;
    use sp_application_crypto::sp_core::{ecdsa, keccak_256, Pair};

    #[test]
    fn verify_should_work() {
        let msg = keccak_256(b"I am Alice!");
        let sig = Keyring::Alice.sign(b"I am Alice!");

        assert!(ecdsa::Pair::verify_prehashed(
            &sig.clone().into(),
            &msg,
            &Keyring::Alice.public().into(),
        ));

        // different public key -> fail
        assert!(!ecdsa::Pair::verify_prehashed(
            &sig.clone().into(),
            &msg,
            &Keyring::Bob.public().into(),
        ));

        let msg = keccak_256(b"I am not Alice!");

        // different msg -> fail
        assert!(
            !ecdsa::Pair::verify_prehashed(&sig.into(), &msg, &Keyring::Alice.public().into(),)
        );
    }

    #[test]
    fn pair_works() {
        let want = crypto::Pair::from_string("//Alice", None).expect("Pair failed").to_raw_vec();
        let got = Keyring::Alice.pair().to_raw_vec();
        assert_eq!(want, got);

        let want = crypto::Pair::from_string("//Bob", None).expect("Pair failed").to_raw_vec();
        let got = Keyring::Bob.pair().to_raw_vec();
        assert_eq!(want, got);

        let want = crypto::Pair::from_string("//Charlie", None).expect("Pair failed").to_raw_vec();
        let got = Keyring::Charlie.pair().to_raw_vec();
        assert_eq!(want, got);

        let want = crypto::Pair::from_string("//Dave", None).expect("Pair failed").to_raw_vec();
        let got = Keyring::Dave.pair().to_raw_vec();
        assert_eq!(want, got);

        let want = crypto::Pair::from_string("//Eve", None).expect("Pair failed").to_raw_vec();
        let got = Keyring::Eve.pair().to_raw_vec();
        assert_eq!(want, got);

        let want = crypto::Pair::from_string("//Ferdie", None).expect("Pair failed").to_raw_vec();
        let got = Keyring::Ferdie.pair().to_raw_vec();
        assert_eq!(want, got);

        let want = crypto::Pair::from_string("//One", None).expect("Pair failed").to_raw_vec();
        let got = Keyring::One.pair().to_raw_vec();
        assert_eq!(want, got);

        let want = crypto::Pair::from_string("//Two", None).expect("Pair failed").to_raw_vec();
        let got = Keyring::Two.pair().to_raw_vec();
        assert_eq!(want, got);
    }
}
