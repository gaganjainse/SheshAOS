use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use futures::stream::BoxStream;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AiError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("API error: {0}")]
    Api(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    async fn stream_chat(&self, req: ChatRequest) -> Result<BoxStream<'static, Result<String, AiError>>, AiError>;
}
