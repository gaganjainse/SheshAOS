use std::time::Duration;

use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde::Deserialize;
use tokio::time::timeout;

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

    async fn complete_stream(
        &self,
        mut request: CompletionRequest,
        on_token: &mut (dyn for<'a> FnMut(&'a str) + Send + Sync),
    ) -> Result<CompletionResponse, ProviderError> {
        request.model = self.model_id.clone();
        let url = format!("{}/v1/chat/completions", self.base_url);

        let mut payload = serde_json::to_value(&request)
            .map_err(|e| ProviderError::MalformedResponse(e.to_string()))?;
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("stream".to_string(), serde_json::Value::Bool(true));
        }

        let resp = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| ProviderError::Http(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(ProviderError::InferenceFailed(format!("HTTP {}", resp.status())));
        }

        let mut stream = resp.bytes_stream();
        let mut full_content = String::new();
        let mut buffer = String::new();
        let mut done_received = false;
        const STREAM_TIMEOUT: Duration = Duration::from_secs(60);

        loop {
            let chunk_result = match timeout(STREAM_TIMEOUT, stream.next()).await {
                Ok(Some(Ok(chunk))) => chunk,
                Ok(Some(Err(e))) => {
                    return Err(ProviderError::Http(e.to_string()));
                }
                Ok(None) => {
                    break;
                }
                Err(_) => {
                    return Err(ProviderError::Http("Stream timeout".to_string()));
                }
            };

            let text = String::from_utf8_lossy(&chunk_result);
            buffer.push_str(&text);

            while let Some(pos) = buffer.find('\n') {
                let line = buffer[..pos].trim().to_string();
                buffer.drain(..=pos);

                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        done_received = true;
                        break;
                    }
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(data)
                        && let Some(token) = val["choices"][0]["delta"]["content"].as_str()
                    {
                        let token_str = token.to_string();
                        full_content.push_str(&token_str);
                        on_token(&token_str);
                    }
                }
            }
            
            if done_received {
                break;
            }
        }

        if !done_received && !buffer.is_empty() {
            // Handle case where stream ends without [DONE] marker
            tracing::warn!("Stream ended without [DONE] marker, processing remaining buffer");
            for line in buffer.lines() {
                let line = line.trim();
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        done_received = true;
                        break;
                    }
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(data)
                        && let Some(token) = val["choices"][0]["delta"]["content"].as_str()
                    {
                        let token_str = token.to_string();
                        full_content.push_str(&token_str);
                        on_token(&token_str);
                    }
                }
            }
        }

        if !done_received {
            tracing::warn!("Stream completed without [DONE] marker");
        }

        Ok(CompletionResponse {
            content: full_content,
            finish_reason: Some("stop".to_string()),
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
    fn test_new_provider_from_config() {
        let config = ModelProviderConfig {
            name: "test-openai".to_string(),
            role: "coder".to_string(),
            base_url: "http://localhost:11434/".to_string(),
            model_id: "llama3".to_string(),
            max_context: 4096,
            supports_vision: false,
            api_key: "".into(),
            provider_kind: "openai".into(),
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
            api_key: "".into(),
            provider_kind: "openai".into(),
        };

        assert!(OpenAiCompatProvider::new(&config).is_err());
    }

    #[test]
    fn test_from_config_alias() {
        let config = ModelProviderConfig {
            name: "test".to_string(),
            role: "planner".to_string(),
            base_url: "http://localhost:11434".to_string(),
            model_id: "llama3".to_string(),
            max_context: 4096,
            supports_vision: false,
            api_key: "".into(),
            provider_kind: "openai".into(),
        };
        let provider = OpenAiCompatProvider::new(&config).unwrap();
        assert_eq!(provider.name(), "test");
        assert_eq!(provider.role(), ModelRole::Planner);
    }

    #[test]
    fn test_base_url_trailing_slash_trimmed() {
        let config = ModelProviderConfig {
            name: "test".to_string(),
            role: "coder".to_string(),
            base_url: "http://localhost:11434/".to_string(),
            model_id: "llama3".to_string(),
            max_context: 4096,
            supports_vision: false,
            api_key: "".into(),
            provider_kind: "openai".into(),
        };
        let provider = OpenAiCompatProvider::new(&config).unwrap();
        assert_eq!(provider.base_url, "http://localhost:11434");
    }

    #[test]
    fn test_base_url_no_trailing_slash() {
        let config = ModelProviderConfig {
            name: "test".to_string(),
            role: "coder".to_string(),
            base_url: "http://localhost:11434".to_string(),
            model_id: "llama3".to_string(),
            max_context: 4096,
            supports_vision: false,
            api_key: "".into(),
            provider_kind: "openai".into(),
        };
        let provider = OpenAiCompatProvider::new(&config).unwrap();
        assert_eq!(provider.base_url, "http://localhost:11434");
    }

    #[test]
    fn test_provider_uppercase_role() {
        let config = ModelProviderConfig {
            name: "test".to_string(),
            role: "PLANNER".to_string(),
            base_url: "http://localhost:11434".to_string(),
            model_id: "llama3".to_string(),
            max_context: 4096,
            supports_vision: false,
            api_key: "".into(),
            provider_kind: "openai".into(),
        };
        let provider = OpenAiCompatProvider::new(&config).unwrap();
        assert_eq!(provider.role(), ModelRole::Planner);
    }

    #[test]
    fn test_provider_mixed_case_role() {
        let config = ModelProviderConfig {
            name: "test".to_string(),
            role: "CoDeR".to_string(),
            base_url: "http://localhost:11434".to_string(),
            model_id: "llama3".to_string(),
            max_context: 4096,
            supports_vision: false,
            api_key: "".into(),
            provider_kind: "openai".into(),
        };
        let provider = OpenAiCompatProvider::new(&config).unwrap();
        assert_eq!(provider.role(), ModelRole::Coder);
    }

    #[test]
    fn test_provider_max_context() {
        let config = ModelProviderConfig {
            name: "test".to_string(),
            role: "vision".to_string(),
            base_url: "http://localhost:11434".to_string(),
            model_id: "llava".to_string(),
            max_context: 8192,
            supports_vision: true,
            api_key: "".into(),
            provider_kind: "openai".into(),
        };
        let provider = OpenAiCompatProvider::new(&config).unwrap();
        assert_eq!(provider.max_context(), 8192);
    }

    #[test]
    fn test_provider_supports_vision() {
        let config = ModelProviderConfig {
            name: "test".to_string(),
            role: "vision".to_string(),
            base_url: "http://localhost:11434".to_string(),
            model_id: "llava".to_string(),
            max_context: 4096,
            supports_vision: true,
            api_key: "".into(),
            provider_kind: "openai".into(),
        };
        let provider = OpenAiCompatProvider::new(&config).unwrap();
        assert!(provider.supports_vision());
    }

    #[test]
    fn test_provider_no_vision() {
        let config = ModelProviderConfig {
            name: "test".to_string(),
            role: "coder".to_string(),
            base_url: "http://localhost:11434".to_string(),
            model_id: "llama3".to_string(),
            max_context: 4096,
            supports_vision: false,
            api_key: "".into(),
            provider_kind: "openai".into(),
        };
        let provider = OpenAiCompatProvider::new(&config).unwrap();
        assert!(!provider.supports_vision());
    }

    #[test]
    fn test_provider_name() {
        let config = ModelProviderConfig {
            name: "my-provider".to_string(),
            role: "planner".to_string(),
            base_url: "http://localhost:11434".to_string(),
            model_id: "llama3".to_string(),
            max_context: 4096,
            supports_vision: false,
            api_key: "".into(),
            provider_kind: "openai".into(),
        };
        let provider = OpenAiCompatProvider::new(&config).unwrap();
        assert_eq!(provider.name(), "my-provider");
    }

    #[test]
    fn test_provider_model_id_stored() {
        let config = ModelProviderConfig {
            name: "test".to_string(),
            role: "planner".to_string(),
            base_url: "http://localhost:11434".to_string(),
            model_id: "llama3-70b".to_string(),
            max_context: 4096,
            supports_vision: false,
            api_key: "".into(),
            provider_kind: "openai".into(),
        };
        let provider = OpenAiCompatProvider::new(&config).unwrap();
        assert_eq!(provider.model_id, "llama3-70b");
    }

    #[test]
    fn test_client_builder_success() {
        let config = ModelProviderConfig {
            name: "test".to_string(),
            role: "planner".to_string(),
            base_url: "http://localhost:11434".to_string(),
            model_id: "llama3".to_string(),
            max_context: 4096,
            supports_vision: false,
            api_key: "".into(),
            provider_kind: "openai".into(),
        };
        let provider = OpenAiCompatProvider::new(&config).unwrap();
        // Verify the provider was constructed successfully with a client
        assert_eq!(provider.name(), "test");
    }

    #[test]
    fn test_all_roles() {
        let roles = vec![("planner", ModelRole::Planner), ("coder", ModelRole::Coder), ("vision", ModelRole::Vision), ("reviewer", ModelRole::Reviewer)];
        for (role_str, expected) in roles {
            let config = ModelProviderConfig {
                name: "test".to_string(),
                role: role_str.to_string(),
                base_url: "http://localhost".to_string(),
                model_id: "model".to_string(),
                max_context: 4096,
                supports_vision: false,
                api_key: String::new(),
                provider_kind: "openai".to_string(),
            };
            let provider = OpenAiCompatProvider::new(&config).unwrap();
            assert_eq!(provider.role(), expected);
        }
    }
}
