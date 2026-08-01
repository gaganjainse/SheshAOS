use std::process::Command;
use tempfile::TempDir;

/// Integration test for full CLI → Kernel → Tools flow
#[tokio::test]
async fn test_full_cli_kernel_tools_flow() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test_config.toml");
    
    // Create test config
    let config_content = r#"
[general]
data_dir = "data"
log_level = "debug"

[resource_limits]
max_ram_mb = 1024
max_vram_mb = 512
max_context_tokens = 4096
max_queue_depth = 10
min_disk_free_gb = 1

[policy]
confirm_destructive = false
confirm_writes = false
confirm_git_commits = false
confirm_terminal = false
dedup_window_secs = 1

[context]
simple_question = 1024
code_edit = 2048
feature_work = 4096
architecture = 8192
ram_headroom_mb = 128

[[model_providers]]
name = "mock-planner"
role = "planner"
base_url = "http://127.0.0.1:1234"
model_id = "mock"
max_context = 4096

[[model_providers]]
name = "mock-coder"
role = "coder"
base_url = "http://127.0.0.1:1234"
model_id = "mock"
max_context = 4096
"#;
    std::fs::write(&config_path, config_content).unwrap();

    // Test 1: nexusaos init
    let output = Command::new("cargo")
        .args(["run", "--bin", "nexusaos", "--", "--config", config_path.to_str().unwrap(), "init"])
        .current_dir("/home/gagan/Workspace/NexusAOS")
        .output()
        .expect("Failed to execute init");
    assert!(output.status.success(), "init failed: {}", String::from_utf8_lossy(&output.stderr));

    // Test 2: nexusaos doctor
    let output = Command::new("cargo")
        .args(["run", "--bin", "nexusaos", "--", "--config", config_path.to_str().unwrap(), "doctor"])
        .current_dir("/home/gagan/Workspace/NexusAOS")
        .output()
        .expect("Failed to execute doctor");
    assert!(output.status.success(), "doctor failed: {}", String::from_utf8_lossy(&output.stderr));

    // Test 3: nexusaos run with simple task
    let output = Command::new("cargo")
        .args(["run", "--bin", "nexusaos", "--", "--config", config_path.to_str().unwrap(), "run", "echo hello", "--yes"])
        .current_dir("/home/gagan/Workspace/NexusAOS")
        .output()
        .expect("Failed to execute run");
    // May fail due to no LLM server, but should not crash
    println!("Run output: {}", String::from_utf8_lossy(&output.stdout));
    println!("Run stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Test 4: nexusaos status
    let output = Command::new("cargo")
        .args(["run", "--bin", "nexusaos", "--", "--config", config_path.to_str().unwrap(), "status"])
        .current_dir("/home/gagan/Workspace/NexusAOS")
        .output()
        .expect("Failed to execute status");
    assert!(output.status.success(), "status failed: {}", String::from_utf8_lossy(&output.stderr));

    // Test 5: nexusaos config
    let output = Command::new("cargo")
        .args(["run", "--bin", "nexusaos", "--", "--config", config_path.to_str().unwrap(), "config"])
        .current_dir("/home/gagan/Workspace/NexusAOS")
        .output()
        .expect("Failed to execute config");
    assert!(output.status.success(), "config failed: {}", String::from_utf8_lossy(&output.stderr));
}

/// Test task submission and event store persistence
#[tokio::test]
async fn test_task_submission_and_event_persistence() {
    use nexusaos_kernel::{
        runtime::kernel::{Kernel, EventStore as EventStoreTrait},
        storage::event_store::EventStore,
        model::registry::ProviderRegistry,
        policy::{PolicyEngine, PolicyRule, TrustTier},
        tools::broker::ToolBroker,
        task::TaskInput,
    };
    use std::sync::Arc;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let events_dir = temp_dir.path().join("events");
    std::fs::create_dir_all(&events_dir).unwrap();

    let store = Arc::new(EventStore::open(events_dir).await.unwrap());
    
    let rules = vec![PolicyRule {
        name: "allow-all".to_string(),
        action_pattern: "*".to_string(),
        decision: "allow".to_string(),
        trust_tier: 0,
        description: None,
    }];
    let policy = PolicyEngine::new(rules, TrustTier::Autonomous);
    
    let registry = Arc::new(ProviderRegistry::new());
    let broker = Arc::new(ToolBroker::new(Arc::new(policy.clone())));
    
    let kernel = Kernel::new(store.clone(), policy, registry, broker).await.unwrap();
    
    // Submit a task
    let task_id = kernel.submit_task(TaskInput::Text("test task".to_string())).await.unwrap();
    
    // Verify event was persisted
    let events = store.get_task_events(&task_id).await.unwrap();
    assert!(!events.is_empty(), "TaskCreated event should be persisted");
    
    // Verify event types
    let has_task_created = events.iter().any(|e| matches!(e.kind, nexusaos_kernel::events::EventKind::TaskCreated));
    assert!(has_task_created, "TaskCreated event should exist");
}

/// Test kernel state transitions
#[tokio::test]
async fn test_kernel_state_transitions() {
    use nexusaos_kernel::{
        runtime::kernel::{Kernel, EventStore as EventStoreTrait},
        storage::event_store::EventStore,
        model::registry::ProviderRegistry,
        policy::{PolicyEngine, PolicyRule, TrustTier},
        tools::broker::ToolBroker,
        task::TaskInput,
    };
    use std::sync::Arc;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let events_dir = temp_dir.path().join("events");
    std::fs::create_dir_all(&events_dir).unwrap();

    let store = Arc::new(EventStore::open(events_dir).await.unwrap());
    
    let rules = vec![PolicyRule {
        name: "allow-all".to_string(),
        action_pattern: "*".to_string(),
        decision: "allow".to_string(),
        trust_tier: 0,
        description: None,
    }];
    let policy = PolicyEngine::new(rules, TrustTier::Autonomous);
    
    let registry = Arc::new(ProviderRegistry::new());
    let broker = Arc::new(ToolBroker::new(Arc::new(policy.clone())));
    
    let kernel = Kernel::new(store.clone(), policy, registry, broker).await.unwrap();
    
    // Submit task
    let task_id = kernel.submit_task(TaskInput::Text("test".to_string())).await.unwrap();
    
    // Execute task (will fail without LLM but should transition states)
    let _ = kernel.execute_task(&task_id).await;
    
    // Check events for state transitions
    let events = store.get_task_events(&task_id).await.unwrap();
    // submit_task emits TaskCreated and TaskClassified events
    let has_task_created = events.iter().any(|e| matches!(e.kind, nexusaos_kernel::events::EventKind::TaskCreated));
    let has_task_classified = events.iter().any(|e| matches!(e.kind, nexusaos_kernel::events::EventKind::TaskClassified));
    assert!(has_task_created, "Should have TaskCreated event");
    assert!(has_task_classified, "Should have TaskClassified event");
}