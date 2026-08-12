use async_trait::async_trait;
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AiError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("API error: {0}")]
    Api(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    pub model: String,
    pub max_tokens: Option<i64>,
}

#[async_trait]
pub trait ModelProvider: Send + Sync {
    /// Returns a stream of text chunks.
    async fn stream_chat(
        &self,
        req: ChatRequest,
    ) -> Result<BoxStream<'static, Result<String, AiError>>, AiError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ai_error_api_display() {
        let err = AiError::Api("bad request".to_string());
        assert_eq!(err.to_string(), "API error: bad request");
    }

    #[test]
    fn test_chat_message_serde_roundtrip() {
        let msg = ChatMessage { role: "user".to_string(), content: "hello".to_string() };
        let json = serde_json::to_string(&msg).unwrap();
        let decoded: ChatMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn test_chat_request_construction() {
        let req = ChatRequest { messages: vec![], model: "gpt".to_string(), max_tokens: Some(100) };
        assert!(req.messages.is_empty());
        assert_eq!(req.model, "gpt");
        assert_eq!(req.max_tokens, Some(100));
    }

    #[test]
    fn test_chat_request_with_none_max_tokens() {
        let req = ChatRequest { messages: vec![], model: "gpt".to_string(), max_tokens: None };
        assert!(req.max_tokens.is_none());
    }

    #[test]
    fn test_chat_message_clone() {
        let msg = ChatMessage { role: "assistant".to_string(), content: "response".to_string() };
        let cloned = msg.clone();
        assert_eq!(msg, cloned);
    }
}
