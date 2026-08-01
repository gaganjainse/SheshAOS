use crate::provider::{AiError, ChatRequest, ModelProvider};
use async_trait::async_trait;
use futures::stream::BoxStream;
use futures::StreamExt;
use reqwest::Client;
use serde::Serialize;
use serde_json::Value;

#[derive(Clone)]
pub struct AnthropicProvider {
    pub base_url: String,
    pub api_key: String,
    pub client: Client,
}

impl AnthropicProvider {
    pub fn new(base_url: String, api_key: String) -> Self {
        Self {
            base_url,
            api_key,
            client: Client::new(),
        }
    }
}

#[derive(Serialize)]
struct AnthropicChatRequest<'a> {
    model: &'a str,
    messages: &'a [crate::provider::ChatMessage],
    stream: bool,
    max_tokens: i64,
}

#[async_trait]
impl ModelProvider for AnthropicProvider {
    async fn stream_chat(
        &self,
        req: ChatRequest,
    ) -> Result<BoxStream<'static, Result<String, AiError>>, AiError> {
        let url = format!("{}/v1/messages", self.base_url);
        let anthropic_req = AnthropicChatRequest {
            model: &req.model,
            messages: &req.messages,
            stream: true,
            max_tokens: req.max_tokens.unwrap_or(1024),
        };

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&anthropic_req)
            .send()
            .await?;

        let stream = response.bytes_stream().map(|res| {
            match res {
                Ok(bytes) => {
                    let mut output = String::new();
                    let text = String::from_utf8_lossy(&bytes);
                    for line in text.lines() {
                        if let Some(data) = line.strip_prefix("data: ")
                            && let Ok(val) = serde_json::from_str::<Value>(data)
                            && val.get("type").and_then(|t| t.as_str()) == Some("content_block_delta")
                            && let Some(delta) = val.get("delta")
                            && let Some(content) = delta.get("text").and_then(|c| c.as_str())
                        {
                            output.push_str(content);
                        }
                    }
                    Ok(output)
                }
                Err(e) => Err(AiError::Network(e)),
            }
        });

        Ok(Box::pin(stream))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_creation() {
        let provider = AnthropicProvider::new("http://localhost".into(), "key".into());
        assert_eq!(provider.base_url, "http://localhost");
        assert_eq!(provider.api_key, "key");
    }

    #[test]
    fn test_anthropic_url_construction() {
        let provider = AnthropicProvider::new("http://localhost:8080".into(), "key".into());
        assert!(provider.base_url.ends_with("8080"));
    }

    #[test]
    fn test_anthropic_provider_clone() {
        let provider = AnthropicProvider::new("http://localhost".into(), "key".into());
        let cloned = provider.clone();
        assert_eq!(provider.base_url, cloned.base_url);
        assert_eq!(provider.api_key, cloned.api_key);
    }
}
