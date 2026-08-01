use std::collections::HashMap;

use crate::{error::ProviderError, model::provider::ModelProvider, state::ModelRole};

/// Registry of available model providers, indexed by role.
pub struct ProviderRegistry {
    providers: HashMap<ModelRole, Box<dyn ModelProvider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self { providers: HashMap::new() }
    }

    /// Register a provider for a role.
    pub fn register(&mut self, provider: Box<dyn ModelProvider>) {
        self.providers.insert(provider.role(), provider);
    }

    /// Get the provider for a given role.
    pub fn get(&self, role: &ModelRole) -> Option<&dyn ModelProvider> {
        self.providers.get(role).map(|p| p.as_ref())
    }

    /// Check health of all registered providers.
    pub async fn health_check_all(&self) -> HashMap<ModelRole, Result<bool, ProviderError>> {
        let mut results = HashMap::new();
        for (role, provider) in &self.providers {
            let result = provider.health_check().await;
            results.insert(*role, result);
        }
        results
    }

    /// List all registered roles.
    pub fn available_roles(&self) -> Vec<ModelRole> {
        self.providers.keys().copied().collect()
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;

    use super::*;
    use crate::model::types::{CompletionRequest, CompletionResponse};

    struct MockProvider;

    #[async_trait]
    impl ModelProvider for MockProvider {
        fn name(&self) -> &str {
            "mock"
        }
        fn role(&self) -> ModelRole {
            ModelRole::Planner
        }
        fn max_context(&self) -> usize {
            100
        }
        fn supports_vision(&self) -> bool {
            false
        }
        async fn health_check(&self) -> Result<bool, ProviderError> {
            Ok(true)
        }
        async fn complete(
            &self,
            _r: CompletionRequest,
        ) -> Result<CompletionResponse, ProviderError> {
            unimplemented!()
        }
        async fn cancel(&self) -> Result<(), ProviderError> {
            Ok(())
        }
    }

    #[test]
    fn test_registry_register_get() {
        let mut registry = ProviderRegistry::new();
        registry.register(Box::new(MockProvider));
        assert!(registry.get(&ModelRole::Planner).is_some());
        assert!(registry.get(&ModelRole::Coder).is_none());
        assert_eq!(registry.available_roles(), vec![ModelRole::Planner]);
    }

    #[test]
    fn test_registry_multiple_providers() {
        struct MockCoder;
        #[async_trait]
        impl ModelProvider for MockCoder {
            fn name(&self) -> &str { "mock-coder" }
            fn role(&self) -> ModelRole { ModelRole::Coder }
            fn max_context(&self) -> usize { 100 }
            fn supports_vision(&self) -> bool { false }
            async fn health_check(&self) -> Result<bool, ProviderError> { Ok(true) }
            async fn complete(&self, _r: CompletionRequest) -> Result<CompletionResponse, ProviderError> { unimplemented!() }
            async fn cancel(&self) -> Result<(), ProviderError> { Ok(()) }
        }

        let mut registry = ProviderRegistry::new();
        registry.register(Box::new(MockProvider));
        registry.register(Box::new(MockCoder));

        let roles = registry.available_roles();
        assert_eq!(roles.len(), 2);
        assert!(roles.contains(&ModelRole::Planner));
        assert!(roles.contains(&ModelRole::Coder));
    }

    #[test]
    fn test_registry_overwrite_provider() {
        let mut registry = ProviderRegistry::new();
        registry.register(Box::new(MockProvider));

        struct AnotherPlanner;
        #[async_trait]
        impl ModelProvider for AnotherPlanner {
            fn name(&self) -> &str { "another-planner" }
            fn role(&self) -> ModelRole { ModelRole::Planner }
            fn max_context(&self) -> usize { 200 }
            fn supports_vision(&self) -> bool { true }
            async fn health_check(&self) -> Result<bool, ProviderError> { Ok(true) }
            async fn complete(&self, _r: CompletionRequest) -> Result<CompletionResponse, ProviderError> { unimplemented!() }
            async fn cancel(&self) -> Result<(), ProviderError> { Ok(()) }
        }

        registry.register(Box::new(AnotherPlanner));
        let planner = registry.get(&ModelRole::Planner).unwrap();
        assert_eq!(planner.name(), "another-planner");
        assert_eq!(planner.max_context(), 200);
        assert!(planner.supports_vision());
    }

    #[test]
    fn test_registry_default() {
        let registry = ProviderRegistry::default();
        assert!(registry.available_roles().is_empty());
    }

    #[test]
    fn test_registry_empty_get() {
        let registry = ProviderRegistry::new();
        assert!(registry.get(&ModelRole::Vision).is_none());
        assert!(registry.get(&ModelRole::Reviewer).is_none());
    }

    #[tokio::test]
    async fn test_registry_health_check_all() {
        let mut registry = ProviderRegistry::new();
        registry.register(Box::new(MockProvider));

        struct HealthyProvider;
        #[async_trait]
        impl ModelProvider for HealthyProvider {
            fn name(&self) -> &str { "healthy" }
            fn role(&self) -> ModelRole { ModelRole::Coder }
            fn max_context(&self) -> usize { 100 }
            fn supports_vision(&self) -> bool { false }
            async fn health_check(&self) -> Result<bool, ProviderError> { Ok(true) }
            async fn complete(&self, _r: CompletionRequest) -> Result<CompletionResponse, ProviderError> { unimplemented!() }
            async fn cancel(&self) -> Result<(), ProviderError> { Ok(()) }
        }

        registry.register(Box::new(HealthyProvider));

        let results = registry.health_check_all().await;
        assert_eq!(results.len(), 2);
        match results.get(&ModelRole::Planner) {
            Some(Ok(true)) => {}
            _ => panic!("Expected planner to be healthy"),
        }
        match results.get(&ModelRole::Coder) {
            Some(Ok(true)) => {}
            _ => panic!("Expected coder to be healthy"),
        }
    }

    #[tokio::test]
    async fn test_registry_health_check_all_with_failure() {
        struct FailingProvider;
        #[async_trait]
        impl ModelProvider for FailingProvider {
            fn name(&self) -> &str { "failing" }
            fn role(&self) -> ModelRole { ModelRole::Reviewer }
            fn max_context(&self) -> usize { 100 }
            fn supports_vision(&self) -> bool { false }
            async fn health_check(&self) -> Result<bool, ProviderError> {
                Err(ProviderError::HealthCheckFailed { name: "failing".into(), reason: "timeout".into() })
            }
            async fn complete(&self, _r: CompletionRequest) -> Result<CompletionResponse, ProviderError> { unimplemented!() }
            async fn cancel(&self) -> Result<(), ProviderError> { Ok(()) }
        }

        let mut registry = ProviderRegistry::new();
        registry.register(Box::new(MockProvider));
        registry.register(Box::new(FailingProvider));

        let results = registry.health_check_all().await;
        assert_eq!(results.len(), 2);
        match results.get(&ModelRole::Planner) {
            Some(Ok(_)) => {}
            _ => panic!("Expected planner to be ok"),
        }
        match results.get(&ModelRole::Reviewer) {
            Some(Err(_)) => {}
            _ => panic!("Expected reviewer to be err"),
        }
    }

    #[test]
    fn test_registry_new_is_empty() {
        let registry = ProviderRegistry::new();
        assert!(registry.available_roles().is_empty());
    }
}
