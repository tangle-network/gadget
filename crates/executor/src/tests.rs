use crate::manager::GadgetProcessManager;
use crate::types::{GadgetProcess, ProcessOutput, Status};
use futures::stream;
use std::time::Duration;
use tokio::time::sleep;

// Helper function to create a runtime for async tests
fn get_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Runtime::new().expect("Failed to create runtime")
}

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
    process.kill().await.unwrap();
    assert_eq!(process.status, Status::Stopped);
}

#[tokio::test]
async fn test_manager_save_load() {
    let mut manager = GadgetProcessManager::new();
    manager.run("test".to_string(), "echo hello").await.unwrap();
    manager.save_state().await.unwrap();

    let loaded_manager = GadgetProcessManager::load_state("./savestate.json")
        .await
        .unwrap();
    assert_eq!(loaded_manager.children.len(), 1);
    assert!(loaded_manager.children.contains_key("test"));
}

#[tokio::test]
async fn test_process_manager_run_command() {
    let mut manager = GadgetProcessManager::new();
    let id = manager.run("test".to_string(), "echo hello").await.unwrap();
    assert!(manager.children.contains_key(&id));
}

#[tokio::test]
async fn test_process_status() {
    let mut process = GadgetProcess::new("sleep 0.1".to_string()).await.unwrap();
    assert_eq!(process.status, Status::Active);
    sleep(Duration::from_millis(200)).await;
    assert_eq!(process.status(), Status::Dead);
}

#[tokio::test]
async fn test_invalid_command() {
    let result = GadgetProcess::new("nonexistent_command".to_string()).await;
    let mut result = result.unwrap();
    result.start().await.unwrap();
    let mut status = result.get_output().unwrap();
    let output = status.recv().await.unwrap();
    println!("Output: {:#?}", output);
    // let mut stream = result.get_output().unwrap();
    // while let Ok(test) = stream.recv().await {
    //     println!("Output: {:#?}", test);
    // }
    // assert!(result.is_err());
}

#[tokio::test]
async fn test_focus_service_until_output_contains() {
    let mut manager = GadgetProcessManager::new();
    let _ = manager
        .run("test_focus".to_string(), "echo 'test output'")
        .await;

    let result = manager
        .focus_service_until_output_contains("test_focus".to_string(), "test output".to_string())
        .await
        .expect("Focus should succeed");

    match result {
        ProcessOutput::Output(output) => assert!(output.contains(&"test output".to_string())),
        _ => panic!("Expected ProcessOutput::Output"),
    }
}

#[tokio::test]
async fn test_process_manager_remove_dead() {
    let mut manager = GadgetProcessManager::new();

    // Start a quick process that will end immediately
    let _ = manager.run("quick_process".to_string(), "echo test").await;
    sleep(Duration::from_millis(100)).await;

    let removed = manager.remove_dead().await;
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
    manager.save_state().await.unwrap();

    let loaded_manager = GadgetProcessManager::load_state("./savestate.json")
        .await
        .unwrap();
    assert_eq!(loaded_manager.children.len(), 1);
    assert!(loaded_manager.children.contains_key("test1"));
}
