use crate::manager::GadgetProcessManager;
use crate::types::{ProcessOutput, Status};
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::time::sleep;

// Helper function to create a runtime for async tests
fn get_runtime() -> Runtime {
    Runtime::new().expect("Failed to create runtime")
}

#[test]
fn test_process_manager_creation() {
    let manager = GadgetProcessManager::new();
    assert!(manager.children.is_empty());
}

#[test]
fn test_process_manager_run_command() {
    let rt = get_runtime();
    rt.block_on(async {
        let mut manager = GadgetProcessManager::new();
        let result = manager.run("test_echo".to_string(), "echo hello");
        let result = result.unwrap();
        println!("Result: {}", result);
        // assert!(result.is_ok());

        // Allow a bit more time for the output to be processed
        sleep(Duration::from_millis(100)).await;

        assert!(manager.children.contains_key("test_echo"));
        let process = manager.children.get("test_echo").unwrap();
        assert!(!process.output.is_empty());
        assert!(process.output.iter().any(|line| line.contains("hello")));
    });
}

#[test]
fn test_process_manager_remove_dead() {
    let rt = get_runtime();
    rt.block_on(async {
        let mut manager = GadgetProcessManager::new();

        // Start a quick process that will end immediately
        let _ = manager.run("quick_process".to_string(), "echo test");
        sleep(Duration::from_millis(100)).await;

        let removed = manager.remove_dead();
        assert!(!removed.is_empty());
        assert!(removed.contains(&"quick_process".to_string()));
    });
}

#[test]
fn test_focus_service_until_output_contains() {
    let rt = get_runtime();
    rt.block_on(async {
        let mut manager = GadgetProcessManager::new();
        let _ = manager.run("test_focus".to_string(), "echo 'test output'");

        let result = manager
            .focus_service_until_output_contains(
                "test_focus".to_string(),
                "test output".to_string(),
            )
            .await
            .expect("Focus should succeed");

        match result {
            ProcessOutput::Output(output) => assert!(output.contains(&"test output".to_string())),
            _ => panic!("Expected ProcessOutput::Output"),
        }
    });
}

#[test]
fn test_process_status() {
    let rt = get_runtime();
    rt.block_on(async {
        let mut manager = GadgetProcessManager::new();
        let _ = manager.run("status_test".to_string(), "sleep 1");

        let process = manager.children.get("status_test").unwrap();
        let status = process.status();

        // Process should be active or sleeping
        assert!(matches!(status, Status::Active | Status::Sleeping));

        // Wait for process to complete
        sleep(Duration::from_secs(2)).await;
        let status = process.status();
        assert!(matches!(status, Status::Dead));
    });
}

#[test]
fn test_process_kill() {
    let rt = get_runtime();
    rt.block_on(async {
        let mut manager = GadgetProcessManager::new();
        let _ = manager.run("kill_test".to_string(), "sleep 10");

        let process = manager.children.get("kill_test").unwrap();
        assert!(process.kill().is_ok());

        sleep(Duration::from_millis(100)).await;
        let status = process.status();
        assert!(matches!(status, Status::Dead));
    });
}

#[test]
fn test_invalid_command() {
    let rt = get_runtime();
    rt.block_on(async {
        let mut manager = GadgetProcessManager::new();
        let result = manager.run("invalid".to_string(), "nonexistent_command");
        assert!(result.is_err());
    });
}

#[test]
fn test_save_and_load_state() {
    let rt = get_runtime();
    rt.block_on(async {
        let mut manager = GadgetProcessManager::new();
        let _ = manager.run("save_test".to_string(), "echo 'test save'");

        // Save state
        let save_result = manager.save_state().await;
        assert!(save_result.is_ok());

        // Load state
        let loaded_manager = GadgetProcessManager::new_from_saved("./savestate.json");
        assert!(loaded_manager.is_ok());

        let loaded_manager = loaded_manager.unwrap();
        assert!(loaded_manager.children.contains_key("save_test"));
    });
}
