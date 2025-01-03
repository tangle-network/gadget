use crate::MyContext;
use gadget_logging::info;
use gadget_runner_tangle::error::TangleError;
use gadget_runner_tangle::tangle::TangleConfig;
use gadget_testing_utils::runner::RunnerSetup;
use gadget_testing_utils::tangle::runner::TangleTestEnv;

#[tokio::test]
async fn test_incredible_squaring_multi_node() -> Result<(), TangleError> {
    let test_env = TangleTestEnv::new()?;

    let test_cases = vec![(5u64, 25u64), (1000u64, 1_000_000u64), (u64::MAX, u64::MAX)];

    let mut runner_setups = Vec::new();

    for (input, expected) in test_cases {
        let config = TangleConfig::default();

        let setup = RunnerSetup::new(config, move |runner| {
            async move {
                info!("Running test with input: {}", input);
                let context = MyContext {
                    env: runner.env().clone(),
                    call_id: None,
                };

                let x_square = blueprint::XsquareEventHandler::new(runner.env(), context).await?;

                runner.job(x_square);

                runner.run().await?;

                // Add assertion here to check if the result matches the expected value

                Ok(())
            }
        });

        runner_setups.push(setup);
    }

    test_env.run_multiple_runners(runner_setups).await?;

    Ok(())
}
