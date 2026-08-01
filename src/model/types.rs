use serde::{Deserialize, Serialize};

/// Role in a chat conversation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    System,
    User,
    Assistant,
}

/// A single message in a chat conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
    // Optional image data for vision models
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<String>>, // base64-encoded
}

/// Request for a completion from a model provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    pub messages: Vec<ChatMessage>,
    pub max_tokens: usize,
    pub temperature: f32,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
}

impl CompletionRequest {
    pub fn new(messages: Vec<ChatMessage>, model: &str, max_tokens: usize) -> Self {
        Self { messages, max_tokens, temperature: 0.7, model: model.to_string(), stop: None }
    }
}

/// Response from a model provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub content: String,
    pub finish_reason: Option<String>,
    pub prompt_tokens: Option<usize>,
    pub completion_tokens: Option<usize>,
    pub model: String,
}

/// Usage/token stats
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_role_serde() {
        let role = ChatRole::System;
        let serialized = serde_json::to_string(&role).unwrap();
        assert_eq!(serialized, "\"system\"");
        let deserialized: ChatRole = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, role);
    }

    #[test]
    fn test_completion_request_new() {
        let req = CompletionRequest::new(vec![], "gpt-4", 100);
        assert_eq!(req.temperature, 0.7);
        assert_eq!(req.model, "gpt-4");
        assert_eq!(req.max_tokens, 100);
    }
}
