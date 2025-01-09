use crate as blueprint;
use crate::MyContext;
use gadget_config::supported_chains::SupportedChains;
use gadget_config::ContextConfig;
use gadget_runner_tangle::error::TangleError;
use gadget_runner_tangle::tangle::TangleConfig;
use gadget_testing_utils::runner::TestEnv;
use gadget_testing_utils::tangle::keys::inject_tangle_key;
use gadget_testing_utils::tangle::node::NodeConfig;
use gadget_testing_utils::tangle::runner::TangleTestEnv;
use url::Url;

#[tokio::test]
async fn test_incredible_squaring() -> Result<(), TangleError> {
    // Start Local Tangle Node
    let node_config = NodeConfig::new(false);
    let tangle_node = gadget_testing_utils::tangle::node::run(node_config)
        .await
        .unwrap();

    // Setup testing directory
    let tmp_dir = tempfile::TempDir::new().map_err(TangleError::Io)?;
    let tmp_dir_path = tmp_dir.path().to_string_lossy().into_owned();
    inject_tangle_key(&tmp_dir_path, "alice").map_err(|e| TangleError::Keystore(e.to_string()))?;

    let context_config = ContextConfig::create_tangle_config(
        Url::parse(&format!("http://127.0.0.1:{}", tangle_node.ws_port())).unwrap(),
        Url::parse(&format!("ws://127.0.0.1:{}", tangle_node.ws_port())).unwrap(),
        tmp_dir_path,
        None,
        SupportedChains::LocalTestnet,
        0,
        Some(0),
    );
    let env = ::gadget_macros::ext::config::load(context_config.clone())
        .expect("Failed to load environment");

    let context = MyContext {
        env: env.clone(),
        call_id: None,
    };

    let x_square = blueprint::XsquareEventHandler::new(&env, context)
        .await
        .unwrap();

    let mut test_env = TangleTestEnv::new(TangleConfig::default(), env, vec![x_square]).unwrap();

    test_env.run_runner().await.unwrap();

    Ok(())
}
