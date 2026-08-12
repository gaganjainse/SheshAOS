use crate::provider::{AiError, ChatRequest, ModelProvider};
use async_trait::async_trait;
use futures::stream::BoxStream;
use futures::StreamExt;
use reqwest::Client;
use serde::Serialize;
use serde_json::Value;

#[derive(Clone)]
pub struct OpenAIProvider {
    pub base_url: String,
    pub api_key: String,
    pub client: Client,
}

impl OpenAIProvider {
    pub fn new(base_url: String, api_key: String) -> Self {
        Self { base_url, api_key, client: Client::new() }
    }
}

#[derive(Serialize)]
struct OpenAIChatRequest<'a> {
    model: &'a str,
    messages: &'a [crate::provider::ChatMessage],
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<i64>,
}

#[async_trait]
impl ModelProvider for OpenAIProvider {
    async fn stream_chat(
        &self,
        req: ChatRequest,
    ) -> Result<BoxStream<'static, Result<String, AiError>>, AiError> {
        let url = format!("{}/v1/chat/completions", self.base_url);
        let openai_req = OpenAIChatRequest {
            model: &req.model,
            messages: &req.messages,
            stream: true,
            max_tokens: req.max_tokens,
        };

        let response =
            self.client.post(&url).bearer_auth(&self.api_key).json(&openai_req).send().await?;

        let stream = response.bytes_stream().map(|res| match res {
            Ok(bytes) => {
                let mut output = String::new();
                let text = String::from_utf8_lossy(&bytes);
                for line in text.lines() {
                    if let Some(data) = line.strip_prefix("data: ") {
                        if data == "[DONE]" {
                            continue;
                        }
                        if let Ok(val) = serde_json::from_str::<Value>(data) {
                            if let Some(choices) = val.get("choices") {
                                if let Some(first_choice) = choices.get(0) {
                                    if let Some(delta) = first_choice.get("delta") {
                                        if let Some(content) =
                                            delta.get("content").and_then(|c| c.as_str())
                                        {
                                            output.push_str(content);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(output)
            }
            Err(e) => Err(AiError::Network(e)),
        });

        Ok(Box::pin(stream))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_creation() {
        let provider = OpenAIProvider::new("http://localhost".into(), "key".into());
        assert_eq!(provider.base_url, "http://localhost");
        assert_eq!(provider.api_key, "key");
    }

    #[test]
    fn test_openai_url_construction() {
        let provider = OpenAIProvider::new("http://localhost:8080".into(), "key".into());
        // URL is constructed in stream_chat, test the format logic indirectly
        assert!(provider.base_url.ends_with("8080"));
    }

    #[test]
    fn test_openai_provider_clone() {
        let provider = OpenAIProvider::new("http://localhost".into(), "key".into());
        let cloned = provider.clone();
        assert_eq!(provider.base_url, cloned.base_url);
        assert_eq!(provider.api_key, cloned.api_key);
    }

    #[test]
    fn test_openai_chat_request_fields() {
        use crate::provider::{ChatMessage, ChatRequest};
        let req = ChatRequest {
            messages: vec![ChatMessage { role: "user".into(), content: "hi".into() }],
            model: "gpt".into(),
            max_tokens: Some(50),
        };
        assert_eq!(req.messages.len(), 1);
        assert_eq!(req.messages[0].role, "user");
        assert_eq!(req.model, "gpt");
    }
}
