use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

use crate::{
    config::ModelProviderConfig,
    error::ProviderError,
    model::{
        provider::ModelProvider,
        types::{CompletionRequest, CompletionResponse},
    },
    state::ModelRole,
};

/// A model provider that speaks the OpenAI-compatible HTTP API.
/// Works with LM Studio, Ollama, vLLM, and any OpenAI-compatible server.
pub struct OpenAiCompatProvider {
    name: String,
    role: ModelRole,
    base_url: String,
    model_id: String,
    max_context: usize,
    supports_vision: bool,
    client: Client,
}

impl OpenAiCompatProvider {
    pub fn new(config: &ModelProviderConfig) -> Result<Self, ProviderError> {
        let role = match config.role.to_lowercase().as_str() {
            "planner" => ModelRole::Planner,
            "coder" => ModelRole::Coder,
            "vision" => ModelRole::Vision,
            "reviewer" => ModelRole::Reviewer,
            _ => return Err(ProviderError::NoProviderForRole { role: config.role.clone() }),
        };

        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(|e| ProviderError::Http(e.to_string()))?;

        Ok(Self {
            name: config.name.clone(),
            role,
            base_url: config.base_url.trim_end_matches('/').to_string(),
            model_id: config.model_id.clone(),
            max_context: config.max_context,
            supports_vision: config.supports_vision,
            client,
        })
    }

    pub fn from_config(config: &ModelProviderConfig) -> Result<Self, ProviderError> {
        Self::new(config)
    }
}

#[derive(Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
    usage: Option<OpenAiUsage>,
    model: String,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct OpenAiMessage {
    content: Option<String>,
}

#[derive(Deserialize)]
struct OpenAiUsage {
    prompt_tokens: usize,
    completion_tokens: usize,
}

#[async_trait]
impl ModelProvider for OpenAiCompatProvider {
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
        self.supports_vision
    }

    async fn health_check(&self) -> Result<bool, ProviderError> {
        let url = format!("{}/v1/models", self.base_url);
        let resp =
            self.client.get(&url).send().await.map_err(|e| ProviderError::Http(e.to_string()))?;
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
        mut request: CompletionRequest,
    ) -> Result<CompletionResponse, ProviderError> {
        // Override model to ensure we request the correct one
        request.model = self.model_id.clone();

        let url = format!("{}/v1/chat/completions", self.base_url);
        let resp = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| ProviderError::Http(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(ProviderError::InferenceFailed(format!("HTTP {}", resp.status())));
        }

        let oa_resp: OpenAiResponse =
            resp.json().await.map_err(|e| ProviderError::MalformedResponse(e.to_string()))?;

        let choice = oa_resp.choices.into_iter().next().ok_or_else(|| {
            ProviderError::MalformedResponse("No choices in response".to_string())
        })?;
        let content = choice.message.content.unwrap_or_default();

        Ok(CompletionResponse {
            content,
            finish_reason: choice.finish_reason,
            prompt_tokens: oa_resp.usage.as_ref().map(|u| u.prompt_tokens),
            completion_tokens: oa_resp.usage.as_ref().map(|u| u.completion_tokens),
            model: oa_resp.model,
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
    fn test_new_provider_from_config() {
        let config = ModelProviderConfig {
            name: "test-openai".to_string(),
            role: "coder".to_string(),
            base_url: "http://localhost:11434/".to_string(),
            model_id: "llama3".to_string(),
            max_context: 4096,
            supports_vision: false,
        };

        let provider = OpenAiCompatProvider::new(&config).unwrap();
        assert_eq!(provider.name(), "test-openai");
        assert_eq!(provider.role(), ModelRole::Coder);
        assert_eq!(provider.base_url, "http://localhost:11434");
    }

    #[test]
    fn test_invalid_role() {
        let config = ModelProviderConfig {
            name: "invalid".to_string(),
            role: "notarole".to_string(),
            base_url: "http://localhost".to_string(),
            model_id: "test".to_string(),
            max_context: 128,
            supports_vision: false,
        };

        assert!(OpenAiCompatProvider::new(&config).is_err());
    }
}
