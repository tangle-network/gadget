use gadget_sdk::executor::process::manager::GadgetProcessManager;
use std::error::Error;

pub async fn run_tangle_validator() -> Result<(), Box<dyn Error>> {
    let mut manager = GadgetProcessManager::new();

    let install = manager.run("binary_install".to_string(), "wget https://github.com/webb-tools/tangle/releases/download/v1.0.0/tangle-default-linux-amd64").await?;
    manager.focus_service_to_completion(install).await?;

    let make_executable = manager
        .run(
            "make_executable".to_string(),
            "chmod +x tangle-default-linux-amd64",
        )
        .await?;
    manager.focus_service_to_completion(make_executable).await?;

    let start_validator = manager
        .run(
            "tangle_validator".to_string(),
            "./tangle-default-linux-amd64",
        )
        .await?;
    manager.focus_service_to_completion(start_validator).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_validator() {
        run_tangle_validator().await.unwrap();
    }
}
