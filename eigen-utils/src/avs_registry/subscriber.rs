use alloy_network::{Network};
use alloy_provider::Provider;
use alloy_provider::ProviderBuilder;
use alloy_pubsub::Subscription;
use alloy_rpc_types::Filter;
use alloy_rpc_types::Log;
use alloy_transport::Transport;
use eigen_contracts::{BlsApkRegistry, RegistryCoordinator};

use crate::types::*;

type AvsRegistrySubscriberResult<T> = Result<T, AvsError>;

pub struct AvsRegistryChainSubscriber<T, P, N>
where
    T: Transport + Clone,
    P: Provider<T, N> + Copy + 'static,
    N: Network,
{
    // logger: Logger,
    registry_coordinator: RegistryCoordinator::RegistryCoordinatorInstance<T, P, N>,
    bls_apk_registry: BlsApkRegistry::BlsApkRegistryInstance<T, P, N>,
    eth_client: P,
}

impl<T, P, N> AvsRegistryChainSubscriber<T, P, N>
where
    T: Transport + Clone,
    P: Provider<T, N> + Copy + 'static,
    N: Network,
{
    pub fn new(
        // logger: Logger,
        registry_coordinator: RegistryCoordinator::RegistryCoordinatorInstance<T, P, N>,
        bls_apk_registry: BlsApkRegistry::BlsApkRegistryInstance<T, P, N>,
        eth_client: P,
    ) -> Self {
        Self {
            // logger,
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
