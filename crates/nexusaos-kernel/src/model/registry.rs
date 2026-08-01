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
}
