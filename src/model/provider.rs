use async_trait::async_trait;

use crate::{
    error::ProviderError,
    model::types::{CompletionRequest, CompletionResponse},
    state::ModelRole,
};

/// Trait that all model providers must implement.
/// This is the contract the kernel uses to talk to models.
#[async_trait]
pub trait ModelProvider: Send + Sync {
    /// Human-readable provider name.
    fn name(&self) -> &str;

    /// What role this provider serves.
    fn role(&self) -> ModelRole;

    /// Maximum context length in tokens.
    fn max_context(&self) -> usize;

    /// Whether this provider supports vision/image input.
    fn supports_vision(&self) -> bool;

    /// Check if the provider's backend is healthy and reachable.
    async fn health_check(&self) -> Result<bool, ProviderError>;

    /// Generate a completion (non-streaming).
    async fn complete(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse, ProviderError>;

    /// Cancel any in-flight request.
    async fn cancel(&self) -> Result<(), ProviderError>;
}
