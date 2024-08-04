use gadget_common::gadget_io;
use gadget_common::prelude::GadgetEnvironment;
use gadget_common::sp_core::Pair;
use shell_sdk::Client;
use structopt::StructOpt;
pub mod config;
pub mod error;
pub mod executor;
pub mod gadget;
pub mod protocols;
pub mod utils;

pub use executor::run_blueprint_manager;
