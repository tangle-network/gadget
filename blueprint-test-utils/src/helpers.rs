use alloy_contract::{CallBuilder, CallDecoder};
use alloy_primitives::hex;
use alloy_rpc_types::TransactionReceipt;
use gadget_sdk::config::protocol::{EigenlayerContractAddresses, SymbioticContractAddresses};
use gadget_sdk::error;
use std::collections::HashMap;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::process::Child;
use tokio::sync::Mutex;
use url::Url;

use crate::test_ext::{find_open_tcp_bind_port, NAME_IDS};
use crate::{inject_test_keys, KeyGenType};
use alloy_provider::{network::Ethereum, Provider};
use alloy_transport::{Transport, TransportError};
use gadget_io::SupportedChains;

use gadget_sdk::config::Protocol;

#[derive(Error, Debug)]
pub enum BlueprintError {
    #[error("Transport error occurred: {0}")]
    TransportError(#[from] TransportError),

    #[error("Contract error occurred: {0}")]
    ContractError(#[from] alloy_contract::Error),

    #[error("{0}")]
    Transaction(#[from] alloy_provider::PendingTransactionError),
}

pub struct BlueprintProcess {
    pub handle: Child,
    pub arguments: Vec<String>,
    pub env_vars: HashMap<String, String>,
}

impl BlueprintProcess {
    pub async fn new(
        program_path: PathBuf,
        arguments: Vec<String>,
        env_vars: HashMap<String, String>,
    ) -> Result<Self, std::io::Error> {
        let mut command = tokio::process::Command::new(program_path);
        command
            .args(&arguments)
            .envs(&env_vars)
            .kill_on_drop(true)
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .stdin(std::process::Stdio::null());

        let handle = command.spawn()?;

        Ok(BlueprintProcess {
            handle,
            arguments,
            env_vars,
        })
    }

    pub async fn kill(&mut self) -> Result<(), std::io::Error> {
        self.handle.kill().await
    }
}

pub struct BlueprintProcessManager {
    processes: Arc<Mutex<Vec<BlueprintProcess>>>,
}

impl Default for BlueprintProcessManager {
    fn default() -> Self {
        Self::new()
    }
}

impl BlueprintProcessManager {
    pub fn new() -> Self {
        BlueprintProcessManager {
            processes: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Helper function to start a blueprint process with given parameters.
    async fn start_blueprint_process(
        program_path: PathBuf,
        instance_id: usize,
        http_endpoint: &str,
        ws_endpoint: &str,
        protocol: Protocol,
        keystore_path: &str,
    ) -> Result<BlueprintProcess, std::io::Error> {
        let keystore_path = Path::new(&keystore_path);
        let keystore_uri = keystore_path.join(format!(
            "keystores/{}/{}",
            NAME_IDS[instance_id].to_lowercase(),
            uuid::Uuid::new_v4()
        ));
        assert!(
            !keystore_uri.exists(),
            "Keystore URI cannot exist: {:?}",
            keystore_uri
        );

        let keystore_uri_normalized =
            std::path::absolute(&keystore_uri).expect("Failed to resolve keystore URI");
        let keystore_uri_str = format!("file:{}", keystore_uri_normalized.display());

        match protocol {
            Protocol::Tangle => {
                inject_test_keys(&keystore_uri.clone(), KeyGenType::Tangle(0))
                    .await
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            }
            Protocol::Eigenlayer => {
                inject_test_keys(&keystore_uri.clone(), KeyGenType::Anvil(0))
                    .await
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            }
            Protocol::Symbiotic => {
                inject_test_keys(&keystore_uri.clone(), KeyGenType::Anvil(0))
                    .await
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            }
        }

        let mut arguments = vec![
            "run".to_string(),
            format!("--target-addr={}", IpAddr::from_str("127.0.0.1").unwrap()),
            format!("--target-port={}", find_open_tcp_bind_port()),
            format!("--http-rpc-url={}", Url::parse(http_endpoint).unwrap()),
            format!("--ws-rpc-url={}", Url::parse(ws_endpoint).unwrap()),
            format!("--keystore-uri={}", keystore_uri_str.clone()),
            format!("--chain={}", SupportedChains::LocalTestnet),
            "-vvv".to_string(),
            "--pretty".to_string(),
            format!("--blueprint-id={}", instance_id),
            format!("--service-id={}", instance_id),
            format!("--protocol={}", protocol),
        ];

        match protocol {
            Protocol::Tangle => {
                arguments.push(format!("--blueprint-id={}", instance_id));
                arguments.push(format!("--service-id={}", instance_id));
            }
            Protocol::Eigenlayer => {
                arguments.push(format!(
                    "--registry-coordinator={}",
                    EigenlayerContractAddresses::default().registry_coordinator_address
                ));
                arguments.push(format!(
                    "--operator-state-retriever={}",
                    EigenlayerContractAddresses::default().operator_state_retriever_address
                ));
                arguments.push(format!(
                    "--delegation-manager={}",
                    EigenlayerContractAddresses::default().delegation_manager_address
                ));
                arguments.push(format!(
                    "--strategy-manager={}",
                    EigenlayerContractAddresses::default().strategy_manager_address
                ));
                arguments.push(format!(
                    "--avs-directory={}",
                    EigenlayerContractAddresses::default().avs_directory_address
                ));
            }
            Protocol::Symbiotic => {
                arguments.push(format!(
                    "--operator-registry-address={}",
                    SymbioticContractAddresses::default().operator_registry_address
                ));
                arguments.push(format!(
                    "--network-registry-address={}",
                    SymbioticContractAddresses::default().network_registry_address
                ));
                arguments.push(format!(
                    "--base-delegator-address={}",
                    SymbioticContractAddresses::default().base_delegator_address
                ));
                arguments.push(format!(
                    "--network-opt-in-service-address={}",
                    SymbioticContractAddresses::default().network_opt_in_service_address
                ));
                arguments.push(format!(
                    "--vault-opt-in-service-address={}",
                    SymbioticContractAddresses::default().vault_opt_in_service_address
                ));
                arguments.push(format!(
                    "--slasher-address={}",
                    SymbioticContractAddresses::default().slasher_address
                ));
                arguments.push(format!(
                    "--veto-slasher-address={}",
                    SymbioticContractAddresses::default().veto_slasher_address
                ));
            }
        }

        let mut env_vars = HashMap::new();
        env_vars.insert("HTTP_RPC_URL".to_string(), http_endpoint.to_string());
        env_vars.insert("WS_RPC_URL".to_string(), ws_endpoint.to_string());
        env_vars.insert("KEYSTORE_URI".to_string(), keystore_uri_str.clone());
        env_vars.insert("DATA_DIR".to_string(), keystore_uri_str);
        env_vars.insert("REGISTRATION_MODE_ON".to_string(), "true".to_string());

        BlueprintProcess::new(program_path, arguments, env_vars).await
    }

    /// Starts multiple blueprint processes and adds them to the process manager.
    pub async fn start_blueprints(
        &self,
        blueprint_paths: Vec<PathBuf>,
        http_endpoint: &str,
        ws_endpoint: &str,
        protocol: Protocol,
        keystore_path: &str,
    ) -> Result<(), std::io::Error> {
        for (index, program_path) in blueprint_paths.into_iter().enumerate() {
            let process = Self::start_blueprint_process(
                program_path,
                index,
                http_endpoint,
                ws_endpoint,
                protocol,
                keystore_path,
            )
            .await?;
            self.processes.lock().await.push(process);
        }
        Ok(())
    }

    pub async fn kill_all(&self) -> Result<(), std::io::Error> {
        let mut processes = self.processes.lock().await;
        for process in processes.iter_mut() {
            process.kill().await?;
        }
        processes.clear();
        Ok(())
    }
}

pub async fn get_receipt<T, P, D>(
    call: CallBuilder<T, P, D, Ethereum>,
) -> Result<TransactionReceipt, BlueprintError>
where
    T: Transport + Clone,
    P: Provider<T, Ethereum>,
    D: CallDecoder,
{
    let pending_tx = match call.send().await {
        Ok(tx) => tx,
        Err(e) => {
            error!("Failed to send transaction: {:?}", e);
            return Err(e.into());
        }
    };

    let receipt = match pending_tx.get_receipt().await {
        Ok(receipt) => receipt,
        Err(e) => {
            error!("Failed to get transaction receipt: {:?}", e);
            return Err(e.into());
        }
    };

    Ok(receipt)
}

/// Waits for the given `successful_responses` Mutex to be greater than or equal to `task_response_count`.
pub async fn wait_for_responses(
    successful_responses: Arc<Mutex<usize>>,
    task_response_count: usize,
    timeout_duration: Duration,
) -> Result<Result<(), std::io::Error>, tokio::time::error::Elapsed> {
    tokio::time::timeout(timeout_duration, async move {
        loop {
            let count = *successful_responses.lock().await;
            if count >= task_response_count {
                crate::info!("Successfully received {} task responses", count);
                return Ok(());
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    })
    .await
}
