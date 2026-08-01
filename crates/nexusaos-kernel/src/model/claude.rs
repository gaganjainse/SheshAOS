//! Anthropic Claude Provider Implementation for NexusAOS Kernel.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::{
    error::ProviderError,
    model::{
        provider::ModelProvider,
        types::{CompletionRequest, CompletionResponse},
    },
    state::ModelRole,
};

pub struct ClaudeProvider {
    name: String,
    role: ModelRole,
    api_key: String,
    model_id: String,
    max_context: usize,
    client: Client,
}

impl ClaudeProvider {
    pub fn new(api_key: String, model_id: String, role: ModelRole) -> Self {
        Self {
            name: format!("anthropic-claude-{}", model_id),
            role,
            api_key,
            model_id,
            max_context: 200_000,
            client: Client::builder().build().unwrap_or_default(),
        }
    }
}

#[derive(Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: usize,
    messages: Vec<ClaudeMessage>,
}

#[derive(Serialize, Deserialize)]
struct ClaudeMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ClaudeResponse {
    content: Vec<ClaudeContentBlock>,
}

#[derive(Deserialize)]
struct ClaudeContentBlock {
    text: Option<String>,
}

#[async_trait]
impl ModelProvider for ClaudeProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn role(&self) -> ModelRole {
        self.role
    }

    fn max_context(&self) -> usize {
        self.max_context
    }

    fn supports_vision(&self) -> bool {
        true
    }

    async fn health_check(&self) -> Result<bool, ProviderError> {
        Ok(!self.api_key.is_empty())
    }

    async fn complete(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse, ProviderError> {
        let messages = request
            .messages
            .into_iter()
            .map(|m| ClaudeMessage {
                role: match m.role {
                    crate::model::types::ChatRole::User => "user".to_string(),
                    crate::model::types::ChatRole::Assistant => "assistant".to_string(),
                    crate::model::types::ChatRole::System => "user".to_string(),
                },
                content: m.content,
            })
            .collect();

        let req_body = ClaudeRequest {
            model: self.model_id.clone(),
            max_tokens: request.max_tokens,
            messages,
        };

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&req_body)
            .send()
            .await
            .map_err(|e| ProviderError::Http(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ProviderError::InferenceFailed(format!(
                "Anthropic HTTP {}",
                response.status()
            )));
        }

        let claude_resp: ClaudeResponse =
            response.json().await.map_err(|e| ProviderError::MalformedResponse(e.to_string()))?;

        let text = claude_resp.content.first().and_then(|c| c.text.clone()).unwrap_or_default();

        Ok(CompletionResponse {
            content: text,
            finish_reason: Some("end_turn".to_string()),
            prompt_tokens: None,
            completion_tokens: None,
            model: self.model_id.clone(),
        })
    }

    async fn cancel(&self) -> Result<(), ProviderError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_provider_construction() {
        let provider = ClaudeProvider::new(
            "dummy_key".to_string(),
            "claude-3-7-sonnet".to_string(),
            ModelRole::Coder,
        );
        assert_eq!(provider.name(), "anthropic-claude-claude-3-7-sonnet");
        assert_eq!(provider.role(), ModelRole::Coder);
        assert_eq!(provider.max_context(), 200_000);
        assert!(provider.supports_vision());
    }

    #[test]
    fn test_claude_provider_different_roles() {
        let planner = ClaudeProvider::new("key".into(), "claude-opus".into(), ModelRole::Planner);
        assert_eq!(planner.role(), ModelRole::Planner);
        assert!(planner.supports_vision());

        let vision = ClaudeProvider::new("key".into(), "claude-sonnet".into(), ModelRole::Vision);
        assert_eq!(vision.role(), ModelRole::Vision);
        assert!(vision.supports_vision());

        let reviewer = ClaudeProvider::new("key".into(), "claude-haiku".into(), ModelRole::Reviewer);
        assert_eq!(reviewer.role(), ModelRole::Reviewer);
        assert!(reviewer.supports_vision());
    }

    #[test]
    fn test_claude_provider_max_context_constant() {
        let provider = ClaudeProvider::new("key".into(), "any-model".into(), ModelRole::Coder);
        assert_eq!(provider.max_context(), 200_000);
    }

    #[test]
    fn test_claude_provider_health_check_with_key() {
        let provider = ClaudeProvider::new("valid-key".into(), "claude-3".into(), ModelRole::Coder);
        // health_check checks if api_key is non-empty
        // We can't easily test the async method synchronously, but we can test the field
        assert!(!provider.api_key.is_empty());
    }

    #[test]
    fn test_claude_provider_name_format() {
        let provider = ClaudeProvider::new("key".into(), "claude-3-5-sonnet-20240620".into(), ModelRole::Planner);
        assert_eq!(provider.name(), "anthropic-claude-claude-3-5-sonnet-20240620");
    }

    #[test]
    fn test_claude_provider_empty_api_key() {
        let provider = ClaudeProvider::new(String::new(), "model".into(), ModelRole::Coder);
        assert!(provider.api_key.is_empty());
        assert!(provider.supports_vision());
    }

    #[test]
    fn test_claude_provider_always_supports_vision() {
        for role in [ModelRole::Planner, ModelRole::Coder, ModelRole::Vision, ModelRole::Reviewer] {
            let provider = ClaudeProvider::new("key".into(), "model".into(), role);
            assert!(provider.supports_vision(), "Claude should always support vision for role {:?}", role);
        }
    }

    #[test]
    fn test_claude_provider_client_built() {
        let _provider = ClaudeProvider::new("key".into(), "model".into(), ModelRole::Coder);
        // The client is built in new(), just verify it doesn't panic
        assert!(true);
    }
}
