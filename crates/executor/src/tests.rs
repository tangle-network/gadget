use crate::manager::GadgetProcessManager;
use crate::types::{GadgetProcess, ProcessOutput, Status};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_process_manager_creation() {
    let manager = GadgetProcessManager::new();
    assert!(manager.children.is_empty());
}

#[tokio::test]
async fn test_process_output() {
    let process = GadgetProcess::new("echo hello".to_string()).await.unwrap();
    assert!(process.output.is_some());
    let mut rx = process.get_output().unwrap();
    let msg = rx.recv().await.unwrap();
    assert!(msg.contains("hello"));
}

#[tokio::test]
async fn test_process_kill() {
    let mut manager = GadgetProcessManager::new();
    let id = manager
        .run("kill_test".to_string(), "sleep 10")
        .await
        .unwrap();

    let process = manager.children.get_mut(&id).unwrap();

    gadget_logging::info!("Process status: {:?}", process);

    process.kill().unwrap();
    assert_eq!(process.status, Status::Dead);
}

#[tokio::test]
async fn test_process_manager_run_command() {
    let mut manager = GadgetProcessManager::new();
    let id = manager.run("test".to_string(), "echo hello").await.unwrap();
    assert!(manager.children.contains_key(&id));
}

#[tokio::test]
async fn test_process_status() {
    let process = GadgetProcess::new("sleep 0.1".to_string()).await.unwrap();
    assert_eq!(process.status, Status::Active);
    sleep(Duration::from_millis(200)).await;
    assert_eq!(process.status(), Status::Dead);
}

#[tokio::test]
async fn test_invalid_command() {
    let mut manager = GadgetProcessManager::new();
    let result = manager
        .run("nonexistent_command".to_string(), "nonexistent command")
        .await;
    println!("result: {:#?}", result);
    assert!(result.is_ok());
    let id = result.unwrap();
    let process = manager.children.get_mut(&id).unwrap();
    println!("process: {:#?}", process);
    let output = process.read_until_timeout(3).await;
    assert!(output.is_err());
}

#[tokio::test]
async fn test_focus_service_until_output_contains() {
    let mut manager = GadgetProcessManager::new();
    let command_name = manager
        .run("test_focus".to_string(), "echo 'test output'")
        .await
        .unwrap();

    let result = manager
        .focus_service_until_output_contains(command_name, "test output".to_string())
        .await
        .expect("Focus should succeed");

    match result {
        ProcessOutput::Output(output) => {
            assert!(output.iter().any(|line| line.contains("test output")));
        }
        output => panic!("Expected ProcessOutput::Output, got: {:#?}", output),
    }
}

#[tokio::test]
async fn test_process_manager_remove_dead() {
    let mut manager = GadgetProcessManager::new();

    // Start a quick process that will end immediately
    let _ = manager.run("quick_process".to_string(), "echo test").await;
    sleep(Duration::from_millis(100)).await;

    let removed = manager.remove_dead();
    assert!(!removed.is_empty());
    assert!(removed.contains(&"quick_process".to_string()));
}

#[tokio::test]
async fn test_save_and_load_state() {
    let mut manager = GadgetProcessManager::new();
    manager
        .run("test1".to_string(), "echo test1")
        .await
        .unwrap();
    manager.save_state().unwrap();

    let loaded_manager = GadgetProcessManager::load_state("./savestate.json").unwrap();
    assert_eq!(loaded_manager.children.len(), 1);
    assert!(loaded_manager.children.contains_key("test1"));
}
