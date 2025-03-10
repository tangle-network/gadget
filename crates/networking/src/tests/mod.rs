#![allow(dead_code)]

use crate::{
    NetworkConfig, NetworkService, service::AllowedKeys, service_handle::NetworkServiceHandle,
};
use gadget_crypto::KeyType;
use libp2p::{
    Multiaddr, PeerId,
    identity::{self, Keypair},
};
use std::{collections::HashSet, time::Duration};
use tokio::time::timeout;
use tracing::info;

mod blueprint_protocol;
mod discovery;
mod gossip;
mod handshake;
