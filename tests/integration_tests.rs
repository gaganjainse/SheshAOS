//! Integration test suite for NexusAOS v2.
//!
//! Tests end-to-end task lifecycle, durability & event replay, policy enforcement,
//! and failure recovery.

use std::sync::Arc;

use nexusaos_kernel::{
    config::AppConfig,
    events::{Event, EventKind, EventPayload},
    model::registry::ProviderRegistry,
    policy::{PolicyEngine, PolicyRule, TrustTier},
    runtime::{kernel::Kernel, replay::ReplayEngine},
    state::TaskState,
    storage::event_store::EventStore,
    task::{Priority, TaskId, TaskInput},
    tools::broker::ToolBroker,
};
use tempfile::TempDir;

fn sample_config_toml(temp_dir: &std::path::Path) -> String {
    format!(
        r#"
[general]
data_dir = "{}"
log_level = "debug"

[resource_limits]
max_ram_mb = 12288
max_vram_mb = 5632
max_context_tokens = 65536
max_queue_depth = 32
min_disk_free_gb = 5

[policy]
confirm_destructive = true
confirm_writes = false
confirm_git_commits = false
confirm_terminal = true
dedup_window_secs = 1

[context]
simple_question = 8192
code_edit = 16384
feature_work = 32768
architecture = 65536
ram_headroom_mb = 2048

[[model_providers]]
name = "test-planner"
role = "planner"
base_url = "http://127.0.0.1:11111"
model_id = "test-planner-model"
max_context = 32768

[tools.filesystem]
allowed_paths = ["{}"]
denied_patterns = ["**/.git/objects/**"]

[tools.git]
enabled = true

[tools.terminal]
timeout_secs = 5
denied_prefixes = ["rm -rf /"]
"#,
        temp_dir.display(),
        temp_dir.display()
    )
}

fn create_allow_all_policy() -> PolicyEngine {
    let rules = vec![PolicyRule {
        name: "allow-all".to_string(),
        action_pattern: "*".to_string(),
        decision: "allow".to_string(),
        trust_tier: 0,
        description: Some("Allow all in tests".to_string()),
    }];
    PolicyEngine::new(rules, TrustTier::Autonomous)
}

fn create_deny_all_policy() -> PolicyEngine {
    PolicyEngine::deny_all()
}

#[tokio::test]
async fn test_end_to_end_task_submission_and_state() {
    let temp_dir = TempDir::new().unwrap();
    let event_store = Arc::new(JsonlEventStore::open(temp_dir.path().to_path_buf()).await.unwrap());
    let policy = create_allow_all_policy();
    let registry = Arc::new(ProviderRegistry::new());
    let broker = Arc::new(ToolBroker::new(Arc::new(policy.clone())));

    let kernel = Kernel::new(event_store.clone(), Arc::new(policy), registry, broker).await.unwrap();

    let input = TaskInput::Text("Explain the Rust ownership model".to_string());
    let task_id = kernel.submit_task(input).await.unwrap();

    assert_eq!(kernel.task_count().await, 1);
    let state = kernel.task_state(&task_id).await.unwrap();
    assert_eq!(state, TaskState::Classified);
}

#[tokio::test]
async fn test_durability_and_event_replay() {
    let temp_dir = TempDir::new().unwrap();
    let store_path = temp_dir.path().to_path_buf();

    let task_id = TaskId::new();

    // 1. Scope 1: Write events to event store
    {
        let event_store = JsonlEventStore::open(store_path.clone()).await.unwrap();

        let mut event1 = Event::new(
            task_id,
            EventKind::TaskCreated,
            EventPayload::TaskCreated {
                request: serde_json::json!({
                    "id": task_id,
                    "input": { "Text": "Test task" },
                    "priority": Priority::Normal,
                    "created_at": chrono::Utc::now(),
                    "parent_task_id": null,
                    "metadata": null
                }),
            },
            "kernel".to_string(),
        );

        let mut event2 = Event::new(
            task_id,
            EventKind::TaskStateChanged,
            EventPayload::StateChanged {
                from: "Received".to_string(),
                to: "Classified".to_string(),
            },
            "kernel".to_string(),
        );

        event_store.append(&mut event1).await.unwrap();
        event_store.append(&mut event2).await.unwrap();
        assert_eq!(event_store.count(), 2);
    }

    // 2. Scope 2: Re-open store and replay
    let reopened_store = JsonlEventStore::open(store_path).await.unwrap();
    let projection = ReplayEngine::replay(&reopened_store).await.unwrap();

    assert_eq!(projection.tasks.len(), 1);
    let task = projection.tasks.get(&task_id).unwrap();
    assert_eq!(task.current_state, TaskState::Classified);
}

#[tokio::test]
async fn test_policy_enforcement_denied_action() {
    let temp_dir = TempDir::new().unwrap();
    let event_store = Arc::new(JsonlEventStore::open(temp_dir.path().to_path_buf()).await.unwrap());
    let policy = create_deny_all_policy();
    let registry = Arc::new(ProviderRegistry::new());
    let broker = Arc::new(ToolBroker::new(Arc::new(policy.clone())));

    let kernel = Kernel::new(event_store, Arc::new(policy), registry, broker).await.unwrap();

    let input = TaskInput::Text("Dangerous command execution".to_string());
    let result = kernel.submit_task(input).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_config_parsing_integration() {
    let temp_dir = TempDir::new().unwrap();
    let config_toml = sample_config_toml(temp_dir.path());
    let config = AppConfig::parse_toml(&config_toml).unwrap();

    assert_eq!(config.model_providers.len(), 1);
    assert_eq!(config.model_providers[0].name, "test-planner");
    assert!(config.tools.git.enabled);
}
