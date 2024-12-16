#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

macro_rules! cfg_remote {
    ($($item:item)*) => {
        $(
        #[cfg(any(
            feature = "aws-signer",
            feature = "gcp-signer",
            feature = "ledger-browser",
            feature = "ledger-node"
        ))]
        $item
        )*
    };
}

pub(crate) use cfg_remote;

pub mod error;
pub use error::*;
mod keystore;
pub use keystore::*;

cfg_remote! {
    pub mod remote;
}

pub mod storage;
