import re

with open("/home/gagan/Workspace/NexusAOS/src/runtime/kernel.rs", "r") as f:
    content = f.read()

test_imports = """    use std::sync::Mutex;
    use crate::model::provider::ModelProvider;
    use crate::model::types::CompletionResponse;
    use async_trait::async_trait;

    struct MockProvider { role: crate::state::ModelRole, content: String }
    #[async_trait]
    impl ModelProvider for MockProvider {
        fn name(&self) -> &str { "mock" }
        fn role(&self) -> crate::state::ModelRole { self.role }
        fn max_context(&self) -> usize { 100 }
        fn supports_vision(&self) -> bool { false }
        async fn health_check(&self) -> Result<bool, crate::error::ProviderError> { Ok(true) }
        async fn complete(&self, _r: CompletionRequest) -> Result<CompletionResponse, crate::error::ProviderError> {
            Ok(CompletionResponse { content: self.content.clone(), finish_reason: None, prompt_tokens: None, completion_tokens: None, model: "mock".into() })
        }
        async fn cancel(&self) -> Result<(), crate::error::ProviderError> { Ok(()) }
    }

"""

content = content.replace("    use std::sync::Mutex;", test_imports)

# Replace Kernel::new(store, policy).await.unwrap() with mock registry and broker
setup_str = """        let mut registry = ProviderRegistry::new();
        registry.register(Box::new(MockProvider { role: crate::state::ModelRole::Planner, content: "plan without code".into() }));
        let broker = Arc::new(ToolBroker::new(Arc::new(PolicyEngine::new(vec![], TrustTier::Autonomous))));
        let kernel = Kernel::new(store, policy, Arc::new(registry), broker).await.unwrap();"""

content = re.sub(r'let kernel = Kernel::new\(store, policy\).await.unwrap\(\);', setup_str, content)

with open("/home/gagan/Workspace/NexusAOS/src/runtime/kernel.rs", "w") as f:
    f.write(content)
