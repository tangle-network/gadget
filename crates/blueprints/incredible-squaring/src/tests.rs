use crate::MyContext;
use gadget_client_tangle::TangleClient;
use gadget_runner_tangle::tangle::TangleRunner;
use gadget_std::error::Error;
use gadget_testing_utils::test_client::TestClient;

#[tokio::test]
async fn test_incredible_squaring() -> Result<(), Error> {
    // Create a test client
    let client = TestClient::new();

    // Create the Tangle client context
    let context = MyContext;

    // Create a Tangle runner with the test client
    let runner = TangleRunner::new(client.clone());

    // Run the job with input 5, expecting output 25
    let input = 5u64;
    let expected_output = 25u64;

    // Execute the job
    let result = runner.run_job(0, vec![input], context).await?;

    // Verify the result
    assert_eq!(result, expected_output);

    // Test with a larger number
    let input = 1000u64;
    let expected_output = 1_000_000u64;

    let result = runner.run_job(0, vec![input], context).await?;
    assert_eq!(result, expected_output);

    // Test overflow handling
    let input = u64::MAX;
    let result = runner.run_job(0, vec![input], context).await?;
    assert_eq!(result, u64::MAX); // Should saturate at u64::MAX

    Ok(())
}
