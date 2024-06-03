use alloy_network::Ethereum;
use alloy_provider::Provider;
use alloy_pubsub::Subscription;
use alloy_rpc_types::Filter;
use alloy_rpc_types::Log;
use alloy_transport::Transport;
use eigen_contracts::{BlsApkRegistry, RegistryCoordinator};

use crate::types::*;

type AvsRegistrySubscriberResult<T> = Result<T, AvsError>;

#[derive(Debug, Clone)]
pub struct AvsRegistryChainSubscriber<T, P>
where
    T: Transport + Clone,
    P: Provider<T, Ethereum> + Clone,
{
    registry_coordinator: RegistryCoordinator::RegistryCoordinatorInstance<T, P>,
    bls_apk_registry: BlsApkRegistry::BlsApkRegistryInstance<T, P>,
    eth_client: P,
}

impl<T, P> AvsRegistryChainSubscriber<T, P>
where
    T: Transport + Clone,
    P: Provider<T, Ethereum> + Clone,
{
    pub fn new(
        registry_coordinator: RegistryCoordinator::RegistryCoordinatorInstance<T, P>,
        bls_apk_registry: BlsApkRegistry::BlsApkRegistryInstance<T, P>,
        eth_client: P,
    ) -> Self {
        Self {
            registry_coordinator,
            bls_apk_registry,
            eth_client,
        }
    }

    pub async fn subscribe_to_new_pubkey_registrations(
        &self,
    ) -> AvsRegistrySubscriberResult<Subscription<Log>> {
        let filter = Filter::new()
            .address(*self.bls_apk_registry.address())
            .event("NewPubkeyRegistration");
        let subscription = self.eth_client.subscribe_logs(&filter).await?;
        Ok(subscription)
    }

    pub async fn subscribe_to_operator_socket_updates(
        &self,
    ) -> AvsRegistrySubscriberResult<Subscription<Log>> {
        let filter = Filter::new()
            .address(*self.registry_coordinator.address())
            .event("OperatorSocketUpdate");
        let subscription = self
            .registry_coordinator
            .provider()
            .subscribe_logs(&filter)
            .await?;
        Ok(subscription)
    }
}
