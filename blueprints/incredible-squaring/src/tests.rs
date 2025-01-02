use crate as blueprint;
use crate::MyContext;
use async_trait::async_trait;
use gadget_config::GadgetConfiguration;
use gadget_event_listeners::core::EventListener;
use gadget_event_listeners::tangle::events::TangleEventListener;
use gadget_event_listeners::tangle::services::{services_post_processor, services_pre_processor};
use gadget_logging::info;
use gadget_macros::ext::tangle::tangle_subxt::tangle_testnet_runtime::api::services::events::JobCalled;
use gadget_runners::core::config::BlueprintConfig;
use gadget_runners::core::error::RunnerError;
use gadget_runners::core::runner::BlueprintRunner;
use gadget_std::{future::Future, pin::Pin};
use gadget_testing_utils::tangle_utils::test_runner::{RunnerSetup, RunnerTestEnv};
use gadget_testing_utils::tangle_utils::Error as TangleError;

#[derive(Clone)]
struct SquaringConfig {
    env: GadgetConfiguration,
}

#[async_trait]
impl BlueprintConfig for SquaringConfig {
    async fn register(&self, _env: &GadgetConfiguration) -> Result<(), RunnerError> {
        Ok(())
    }

    async fn requires_registration(&self, _env: &GadgetConfiguration) -> Result<bool, RunnerError> {
        Ok(false)
    }
}

#[tokio::test]
async fn test_incredible_squaring_multi_node() -> Result<(), TangleError> {
    // Create test environment
    let test_env = RunnerTestEnv::new()?;

    // Test cases
    let test_cases = vec![
        (5u64, 25u64),
        (1000u64, 1_000_000u64),
        (u64::MAX, u64::MAX), // Testing saturation
    ];

    let mut runner_setups = Vec::new();

    for (input, expected) in test_cases {
        let config = SquaringConfig {
            env: test_env.config.clone(),
        };

        let setup = RunnerSetup::new(config.clone(), move |runner: &mut BlueprintRunner| {
            let config = config.clone();
            let input = input;
            Box::pin(async move {
                info!("Running test with input: {}", input);
                let context = MyContext {
                    env: config.env.clone(),
                    call_id: None,
                };

                let job = blueprint::XsquareEventHandler::new(&config.env, context)
                    .await
                    .unwrap();
                runner.job(job);

                // Run the runner which will process the job
                runner.run().await.map_err(|e| TangleError::Runner(e))?;

                Ok(())
            })
                as Pin<Box<dyn Future<Output = Result<(), TangleError>> + Send + 'static>>
        });

        runner_setups.push(setup);
    }

    // Run all tests concurrently
    test_env.run_multiple_runners(runner_setups).await?;

    Ok(())
}

#[tokio::test]
async fn test_incredible_squaring_single() -> Result<(), TangleError> {
    let test_env = RunnerTestEnv::new()?;

    let config = SquaringConfig {
        env: test_env.config.clone(),
    };

    let setup = RunnerSetup::new(config, |runner| {
        Box::pin(async move {
            let test_cases = vec![(5u64, 25u64), (1000u64, 1_000_000u64), (u64::MAX, u64::MAX)];

            for (input, expected) in test_cases {
                let context = MyContext {
                    env: config.env.clone(),
                    call_id: None,
                };

                let job = blueprint::XsquareEventHandler::new(&config.env.clone(), context)
                    .await
                    .unwrap();
                runner.job(job);

                // Run the runner which will process the job
                runner.run().await?;
            }

            Ok(())
        })
    });

    test_env.run_runner(setup.config, setup.setup).await?;

    Ok(())
}
