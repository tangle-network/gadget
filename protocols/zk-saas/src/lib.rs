use crate::network::ZkNetworkService;
use crate::protocol::ZkJobAdditionalParams;
use async_trait::async_trait;
use gadget_common::client::{
    AccountId, ClientWithApi, JobsApiForGadget, JobsClient, PalletSubmitter,
};
use gadget_common::debug_logger::DebugLogger;
use gadget_common::full_protocol::SharedOptional;
use gadget_common::prelude::*;
use gadget_common::{generate_protocol, Error};
use mpc_net::prod::RustlsCertificate;
use network::ZkProtocolNetworkConfig;
use protocol_macros::protocol;
use sc_client_api::Backend;
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::Block;
use std::sync::Arc;
use tangle_primitives::roles::ZeroKnowledgeRoleType;
use tokio_rustls::rustls::{Certificate, PrivateKey, RootCertStore};

pub mod network;
pub mod protocol;

generate_protocol!(
    "ZK-Protocol",
    ZkProtocol,
    ZkJobAdditionalParams,
    protocol::generate_protocol_from,
    protocol::create_next_job,
    GadgetJobType::ZkSaaSPhaseTwo(_),
    RoleType::ZkSaaS(ZeroKnowledgeRoleType::ZkSaaSGroth16)
);

pub async fn create_zk_network(config: ZkProtocolNetworkConfig) -> Result<ZkNetworkService, Error> {
    let our_identity = RustlsCertificate {
        cert: Certificate(config.public_identity_der.clone()),
        private_key: PrivateKey(config.private_identity_der.clone()),
    };

    if let Some(addr) = &config.king_bind_addr {
        ZkNetworkService::new_king(config.account_id, *addr, our_identity).await
    } else {
        let king_addr = config
            .client_only_king_addr
            .expect("King address must be specified if king bind address is not specified");

        let mut king_certs = RootCertStore::empty();
        king_certs
            .add(&Certificate(
                config
                    .client_only_king_public_identity_der
                    .clone()
                    .expect("The client must specify the identity of the king"),
            ))
            .map_err(|err| Error::InitError {
                err: err.to_string(),
            })?;

        ZkNetworkService::new_client(king_addr, config.account_id, our_identity, king_certs).await
    }
}

generate_setup_and_run_command!(ZkProtocol);
