# Tangle Testing Framework

A lightweight testing framework for Tangle blueprints that provides a complete test environment with minimal setup.

## Architecture

The framework consists of three main components:

### TangleTestHarness

The main test harness that manages the complete test infrastructure. It handles:

- Local node setup and management
- Key/signer initialization
- MBSM deployment
- Service deployment and configuration
- Job execution and verification

### TangleTestEnv

The test environment implementation that manages blueprint runners and event handlers. It's used internally by the harness to:

- Initialize event handlers
- Manage job execution
- Handle test-specific configurations

### Blueprint Context

Your blueprint-specific context (e.g. `MyContext`) that holds the state needed by your event handlers.

## Usage Example

```rust
#[tokio::test]
async fn test_my_blueprint() -> Result<()> {
    // Initialize test harness (node, keys, deployment)
    let harness = TangleTestHarness::setup().await?;

    // Create your blueprint context
    let blueprint_ctx = MyContext {
        env: harness.env.clone(),
        call_id: None,
    };

    // Initialize your event handler
    let handler = MyEventHandler::new(&harness.env, blueprint_ctx).await?;

    // Setup service and run test
    let (_blueprint_id, service_id) = harness.setup_service(vec![handler]).await?;

    // Execute jobs and verify results
    let results = harness
        .execute_job(
            service_id,
            JOB_ID,
            inputs,
            expected_outputs,
        )
        .await?;
}
```
