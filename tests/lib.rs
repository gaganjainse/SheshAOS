//! NexusAOS Test Suite Library
//! 
//! This library provides common test utilities and re-exports
//! for the NexusAOS test suite.

pub mod test_utils {
    use nexusaos_kernel::{
        runtime::kernel::Kernel,
        storage::JsonlEventStore,
        model::registry::ProviderRegistry,
        policy::{PolicyEngine, PolicyRule, TrustTier},
        tools::broker::ToolBroker,
    };
    use std::sync::Arc;
    use tempfile::TempDir;

    /// Create a test kernel with default configuration
    pub async fn create_test_kernel() -> (Kernel, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let events_dir = temp_dir.path().join("events");
        std::fs::create_dir_all(&events_dir).unwrap();

        let store = Arc::new(JsonlEventStore::open(events_dir).await.unwrap());
        
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
        
        let kernel = Kernel::new(store, Arc::new(policy), registry, broker).await.unwrap();
        
        (kernel, temp_dir)
    }
}