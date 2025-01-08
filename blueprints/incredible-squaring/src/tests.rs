use crate as blueprint;
use crate::MyContext;
use gadget_config::supported_chains::SupportedChains;
use gadget_config::ContextConfig;
use gadget_runner_tangle::error::TangleError;
use gadget_runner_tangle::tangle::TangleConfig;
use gadget_testing_utils::runner::TestEnv;
use gadget_testing_utils::tangle::runner::TangleTestEnv;
use url::Url;

#[tokio::test]
async fn test_incredible_squaring() -> Result<(), TangleError> {
    let context_config = ContextConfig::create_tangle_config(
        Url::parse("http://127.0.0.1:0").unwrap(),
        Url::parse("ws://127.0.0.1:0").unwrap(),
        Default::default(),
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
