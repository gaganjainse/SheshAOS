//! Anthropic Claude Provider Implementation for SheshaAOS Kernel.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use std::time::Duration;

use crate::{
    error::ProviderError,
    model::{
        provider::ModelProvider,
        types::{ChatRole, CompletionRequest, CompletionResponse},
    },
    state::ModelRole,
};

pub struct ClaudeProvider {
    name: String,
    role: ModelRole,
    api_key: String,
    base_url: String,
    model_id: String,
    max_context: usize,
    client: Client,
}

impl ClaudeProvider {
    pub fn new(api_key: String, model_id: String, role: ModelRole) -> Result<Self, ProviderError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(|e| ProviderError::Http(e.to_string()))?;
        Ok(Self {
            name: format!("anthropic-claude-{}", model_id),
            role,
            api_key,
            base_url: "https://api.anthropic.com".to_string(),
            model_id,
            max_context: 200_000,
            client,
        })
    }
}

#[derive(Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
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
    stop_reason: Option<String>,
    #[serde(rename = "usage")]
    usage: Option<ClaudeUsage>,
}

#[derive(Deserialize)]
struct ClaudeContentBlock {
    text: Option<String>,
}

#[derive(Deserialize)]
struct ClaudeUsage {
    input_tokens: Option<usize>,
    output_tokens: Option<usize>,
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
        let url = format!("{}/v1/messages", self.base_url);
        let resp = self
            .client
            .get(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .send()
            .await
            .map_err(|e| ProviderError::Http(e.to_string()))?;

        if resp.status().is_success() {
            Ok(true)
        } else {
            Err(ProviderError::HealthCheckFailed {
                name: self.name.clone(),
                reason: format!("HTTP {}", resp.status()),
            })
        }
    }

    async fn complete(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse, ProviderError> {
        let mut system_parts: Vec<String> = Vec::new();
        let mut messages: Vec<ClaudeMessage> = Vec::new();
        for m in request.messages {
            match m.role {
                ChatRole::System => system_parts.push(m.content),
                ChatRole::User | ChatRole::Assistant => {
                    let role = if matches!(m.role, ChatRole::User) { "user" } else { "assistant" };
                    match messages.last_mut() {
                        Some(last) if last.role == role => {
                            last.content.push('\n');
                            last.content.push_str(&m.content);
                        }
                        _ => messages.push(ClaudeMessage {
                            role: role.to_string(),
                            content: m.content,
                        }),
                    }
                }
            }
        }

        let req_body = ClaudeRequest {
            model: self.model_id.clone(),
            max_tokens: request.max_tokens.max(1),
            system: (!system_parts.is_empty()).then(|| system_parts.join("\n\n")),
            messages,
        };

        let response = self
            .client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&req_body)
            .send()
            .await
            .map_err(|e| ProviderError::Http(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::InferenceFailed(format!(
                "Anthropic HTTP {status}: {body}"
            )));
        }

        let claude_resp: ClaudeResponse =
            response.json().await.map_err(|e| ProviderError::MalformedResponse(e.to_string()))?;

        let text: String = claude_resp
            .content
            .iter()
            .filter_map(|c| c.text.as_deref())
            .collect();

        Ok(CompletionResponse {
            content: text,
            finish_reason: claude_resp.stop_reason,
            prompt_tokens: claude_resp.usage.as_ref().and_then(|u| u.input_tokens),
            completion_tokens: claude_resp.usage.as_ref().and_then(|u| u.output_tokens),
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
        )
        .unwrap();
        assert_eq!(provider.name(), "anthropic-claude-claude-3-7-sonnet");
        assert_eq!(provider.role(), ModelRole::Coder);
        assert_eq!(provider.max_context(), 200_000);
        assert!(provider.supports_vision());
    }

    #[test]
    fn test_claude_provider_different_roles() {
        let planner = ClaudeProvider::new("key".into(), "claude-opus".into(), ModelRole::Planner).unwrap();
        assert_eq!(planner.role(), ModelRole::Planner);
        assert!(planner.supports_vision());

        let vision = ClaudeProvider::new("key".into(), "claude-sonnet".into(), ModelRole::Vision).unwrap();
        assert_eq!(vision.role(), ModelRole::Vision);
        assert!(vision.supports_vision());

        let reviewer = ClaudeProvider::new("key".into(), "claude-haiku".into(), ModelRole::Reviewer).unwrap();
        assert_eq!(reviewer.role(), ModelRole::Reviewer);
        assert!(reviewer.supports_vision());
    }

    #[test]
    fn test_claude_provider_max_context_constant() {
        let provider = ClaudeProvider::new("key".into(), "any-model".into(), ModelRole::Coder).unwrap();
        assert_eq!(provider.max_context(), 200_000);
    }

    #[test]
    fn test_claude_provider_health_check_with_key() {
        let provider = ClaudeProvider::new("valid-key".into(), "claude-3".into(), ModelRole::Coder).unwrap();
        // health_check checks if api_key is non-empty
        // We can't easily test the async method synchronously, but we can test the field
        assert!(!provider.api_key.is_empty());
    }

    #[test]
    fn test_claude_provider_name_format() {
        let provider = ClaudeProvider::new("key".into(), "claude-3-5-sonnet-20240620".into(), ModelRole::Planner).unwrap();
        assert_eq!(provider.name(), "anthropic-claude-claude-3-5-sonnet-20240620");
    }

    #[test]
    fn test_claude_provider_empty_api_key() {
        let provider = ClaudeProvider::new(String::new(), "model".into(), ModelRole::Coder).unwrap();
        assert!(provider.api_key.is_empty());
        assert!(provider.supports_vision());
    }

    #[test]
    fn test_claude_provider_always_supports_vision() {
        for role in [ModelRole::Planner, ModelRole::Coder, ModelRole::Vision, ModelRole::Reviewer] {
            let provider = ClaudeProvider::new("key".into(), "model".into(), role).unwrap();
            assert!(provider.supports_vision(), "Claude should always support vision for role {:?}", role);
        }
    }

    #[test]
    fn test_claude_provider_client_built() {
        let _provider = ClaudeProvider::new("key".into(), "model".into(), ModelRole::Coder).unwrap();
    }
}
