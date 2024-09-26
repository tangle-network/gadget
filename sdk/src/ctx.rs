//! A set of traits and utilities that provide a common interface for interacting with the Gadget SDK.
//!
//! Usually, when you need access to the SDK, you will need to pass the Context to your jobs/functions. In your code, you will create a struct that encapsulates all the things that you would need from outside world from your job.
//! for example, if you need to interact with the network, you will need to have a network client in your struct. If you need to interact with the database storage, you will need to have a db client in your struct. And so on.
//!
//! This module provides a set of traits that you can implement for your struct to make it a context-aware struct by adding new functionalities to it.
//!
//! # Example
//!
//! ```rust,no_run
//! use gadget_sdk::ctx::KeystoreContext;
//! use gadget_sdk::config::StdGadgetConfigurtion;
//!
//! // This your struct that encapsulates all the things you need from outside world.
//! #[derive(Clone, Debug, KeystoreContext)]
//! struct MyContext {
//!   foo: String,
//!   bar: u64,
//!   #[config]
//!   sdk_config: StdGadgetConfigurtion,
//! }
//! // By deriving KeystoreContext, you can now access the keystore client from your struct.
//!
//! #[job(id = 0, params(who), result(_))]
//! async fn my_job(ctx: &MyContext, who: String) -> Result<String, MyError> {
//!   // Access the keystore client from the context.
//!   let keystore = ctx.keystore();
//!  // Do something with the keystore client.
//!  // ...
//!  Ok(format!("Hello, {}!", who))
//! }
//! ```
//!

use crate::keystore::backend::GenericKeyStore;
// derives
pub use gadget_context_derive::*;

/// `KeystoreContext` trait provides access to the generic keystore from the context.
pub trait KeystoreContext<RwLock: lock_api::RawRwLock> {
    /// Get the keystore client from the context.
    fn keystore(&self) -> Result<GenericKeyStore<RwLock>, crate::config::Error>;
}

/// `GossipNetworkContext` trait provides access to the network client from the context.
pub trait GossipNetworkContext {
    /// Get the Goossip client from the context.
    fn gossip_network(&self) -> &crate::network::gossip::GossipHandle;
}
