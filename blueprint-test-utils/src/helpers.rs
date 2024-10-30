use alloy_contract::{CallBuilder, CallDecoder};
use alloy_primitives::hex;
use alloy_rpc_types::TransactionReceipt;
use gadget_sdk::config::protocol::{EigenlayerContractAddresses, SymbioticContractAddresses};
use gadget_sdk::error;
use gadget_sdk::keystore::backend::fs::FilesystemKeystore;
use gadget_sdk::keystore::Backend;
use std::collections::HashMap;
use std::net::IpAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use tokio::process::Child;
use tokio::sync::Mutex;
use url::Url;

use crate::test_ext::{find_open_tcp_bind_port, ANVIL_PRIVATE_KEYS, NAME_IDS};
use alloy_provider::{network::Ethereum, Provider};
use alloy_transport::{Transport, TransportError};
use gadget_io::SupportedChains;
use gadget_sdk::config::Protocol;

use thiserror::Error;
use crate::{inject_test_keys, KeyGenType};

#[derive(Error, Debug)]
pub enum BlueprintError {
    #[error("Transport error occurred: {0}")]
    TransportError(#[from] TransportError),

    #[error("Contract error occurred: {0}")]
    ContractError(#[from] alloy_contract::Error),
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
    ) -> Result<BlueprintProcess, std::io::Error> {
        let tmp_store = uuid::Uuid::new_v4().to_string();
        let keystore_uri = format!(
            "./target/keystores/{}/{tmp_store}/",
            NAME_IDS[instance_id].to_lowercase()
        );
        assert!(
            !std::path::Path::new(&keystore_uri).exists(),
            "Keystore URI cannot exist: {}",
            keystore_uri
        );

        let keystore_uri_normalized =
            std::path::absolute(&keystore_uri).expect("Failed to resolve keystore URI");
        let keystore_uri_str = format!("file:{}", keystore_uri_normalized.display());

        let filesystem_keystore = FilesystemKeystore::open(keystore_uri_str.clone())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        // let _ = ANVIL_PRIVATE_KEYS.iter().enumerate().map(|(key, _)| inject_test_keys(&keystore_uri_normalized.clone(), KeyGenType::Anvil(key)));

        for seed in [
            "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
            "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d",
            "0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a",
            "0x7c852118294e51e653712a81e05800f419141751be58f605c371e15141b007a6",
            "0x47e179ec197488593b187f80a00eb0da91f1b9d0b13f8733639f19c30a34926a",
            "0x8b3a350cf5c34c9194ca85829a2df0ec3153be0318b5e2d3348e872092edffba",
            "0x92db14e403b83dfe3df233f83dfa3a0d7096f21ca9b0d6d6b8d88b2b4ec1564e",
            "0x4bbbf85ce3377467afe5d46f804f221813b2bb87f24d81f60f1fcdbf7cbf4356",
            "0xdbda1821b80551c9d65939329250298aa3472ba22feea921c0cf5d620ea67b97",
            "0x2a871d0798f97d79848a013d4936a73bf4cc922c825d33c1cf7073dff6d409c6",
        ] {
            let seed_bytes = hex::decode(&seed[2..]).expect("Invalid hex seed");
            filesystem_keystore
                .ecdsa_generate_new(Some(&seed_bytes))
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        }

        if let Protocol::Eigenlayer = protocol {
            filesystem_keystore.bls_bn254_generate_from_string("1371012690269088913462269866874713266643928125698382731338806296762673180359922".to_string())
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        };



        let mut arguments = vec![
            "run".to_string(),
            format!("--bind-addr={}", IpAddr::from_str("127.0.0.1").unwrap()),
            format!("--bind-port={}", find_open_tcp_bind_port()),
            format!("--http-rpc-url={}", Url::parse(http_endpoint).unwrap()),
            format!("--ws-rpc-url={}", Url::parse(ws_endpoint).unwrap()),
            format!("--keystore-uri={}", keystore_uri_str.clone()),
            format!("--chain={}", SupportedChains::LocalTestnet),
            // format!("--vvv"),
            format!("--pretty"),
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
    ) -> Result<(), std::io::Error> {
        for (index, program_path) in blueprint_paths.into_iter().enumerate() {
            let process = Self::start_blueprint_process(
                program_path,
                index,
                http_endpoint,
                ws_endpoint,
                protocol,
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
