use crate::config::BlueprintManagerConfig;
use crate::error::Result;
use crate::gadget::native::FilteredBlueprint;
use async_trait::async_trait;
use blueprint_sdk::runner::config::BlueprintEnvironment;
use std::path::PathBuf;

pub mod github;
pub mod testing;

#[async_trait]
#[auto_impl::auto_impl(Box)]
pub trait BinarySourceFetcher: Send + Sync {
    async fn get_binary(&self) -> Result<PathBuf>;
    fn blueprint_id(&self) -> u64;
    fn name(&self) -> String;
}

#[must_use]
pub fn process_arguments_and_env(
    gadget_config: &BlueprintEnvironment,
    manager_opts: &BlueprintManagerConfig,
    blueprint_id: u64,
    service_id: u64,
    blueprint: &FilteredBlueprint,
    sub_service_str: &str,
) -> (Vec<String>, Vec<(String, String)>) {
    let mut arguments = vec![];
    arguments.push("run".to_string());

    if manager_opts.test_mode {
        arguments.push("--test-mode".to_string());
    }

    if manager_opts.pretty {
        arguments.push("--pretty".to_string());
    }

    for bootnode in &gadget_config.bootnodes {
        arguments.push(format!("--bootnodes={}", bootnode));
    }

    if manager_opts.test_mode {
        gadget_logging::warn!("Test mode is enabled");
    }

    let chain = match gadget_config.http_rpc_endpoint.as_str() {
        url if url.contains("127.0.0.1") || url.contains("localhost") => {
            SupportedChains::LocalTestnet
        }
        _ => SupportedChains::Testnet,
    };

    arguments.extend([
        format!("--http-rpc-url={}", gadget_config.http_rpc_endpoint),
        format!("--ws-rpc-url={}", gadget_config.ws_rpc_endpoint),
        format!("--keystore-uri={}", gadget_config.keystore_uri),
        format!("--protocol={}", blueprint.protocol),
        format!("--chain={}", chain),
        format!("--blueprint-id={}", blueprint_id),
        format!("--service-id={}", service_id),
    ]);

    // TODO: Add support for keystore password
    // if let Some(keystore_password) = &gadget_config.keystore_password {
    //     arguments.push(format!("--keystore-password={}", keystore_password));
    // }

    // Uses occurrences of clap short -v
    if manager_opts.verbose > 0 {
        arguments.push(format!("-{}", "v".repeat(manager_opts.verbose as usize)));
    }

    // Add required env vars for all child processes/gadgets
    let mut env_vars = vec![
        (
            "HTTP_RPC_URL".to_string(),
            gadget_config.http_rpc_endpoint.to_string(),
        ),
        (
            "WS_RPC_URL".to_string(),
            gadget_config.ws_rpc_endpoint.to_string(),
        ),
        (
            "KEYSTORE_URI".to_string(),
            manager_opts.keystore_uri.clone(),
        ),
        ("BLUEPRINT_ID".to_string(), format!("{}", blueprint_id)),
        ("SERVICE_ID".to_string(), format!("{}", service_id)),
    ];

    let base_data_dir = &manager_opts.data_dir;
    let data_dir = base_data_dir.join(format!("blueprint-{blueprint_id}-{sub_service_str}"));
    env_vars.push((
        "DATA_DIR".to_string(),
        data_dir.to_string_lossy().into_owned(),
    ));

    // Ensure our child process inherits the current processes' environment vars
    env_vars.extend(std::env::vars());

    if blueprint.registration_mode {
        env_vars.push(("REGISTRATION_MODE_ON".to_string(), "true".to_string()));
    }

    (arguments, env_vars)
}
