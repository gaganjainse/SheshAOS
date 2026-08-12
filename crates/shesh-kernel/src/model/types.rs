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

    #[test]
    fn test_chat_role_all_variants() {
        let roles = vec![ChatRole::System, ChatRole::User, ChatRole::Assistant];
        for role in roles {
            let json = serde_json::to_string(&role).unwrap();
            let back: ChatRole = serde_json::from_str(&json).unwrap();
            assert_eq!(role, back);
        }
    }

    #[test]
    fn test_chat_role_serde_strings() {
        assert_eq!(serde_json::to_string(&ChatRole::System).unwrap(), "\"system\"");
        assert_eq!(serde_json::to_string(&ChatRole::User).unwrap(), "\"user\"");
        assert_eq!(serde_json::to_string(&ChatRole::Assistant).unwrap(), "\"assistant\"");
    }

    #[test]
    fn test_chat_message_construction() {
        let msg = ChatMessage { role: ChatRole::User, content: "hello".into(), images: None };
        assert_eq!(msg.role, ChatRole::User);
        assert_eq!(msg.content, "hello");
        assert!(msg.images.is_none());
    }

    #[test]
    fn test_chat_message_with_images() {
        let msg = ChatMessage {
            role: ChatRole::User,
            content: "describe".into(),
            images: Some(vec!["base64data".into()]),
        };
        assert!(msg.images.is_some());
        assert_eq!(msg.images.unwrap().len(), 1);
    }

    #[test]
    fn test_chat_message_serde() {
        let msg =
            ChatMessage { role: ChatRole::Assistant, content: "response".into(), images: None };
        let json = serde_json::to_string(&msg).unwrap();
        let back: ChatMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg.role, back.role);
        assert_eq!(msg.content, back.content);
    }

    #[test]
    fn test_completion_request_with_stop_sequences() {
        let req = CompletionRequest {
            messages: vec![],
            max_tokens: 100,
            temperature: 0.5,
            model: "test".into(),
            stop: Some(vec!["\n".into(), "END".into()]),
        };
        assert_eq!(req.stop, Some(vec!["\n".into(), "END".into()]));
        assert_eq!(req.temperature, 0.5);
    }

    #[test]
    fn test_completion_request_serde() {
        let req = CompletionRequest::new(
            vec![ChatMessage { role: ChatRole::User, content: "hi".into(), images: None }],
            "gpt-4",
            50,
        );
        let json = serde_json::to_string(&req).unwrap();
        let back: CompletionRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req.model, back.model);
        assert_eq!(req.max_tokens, back.max_tokens);
    }

    #[test]
    fn test_completion_response_construction() {
        let resp = CompletionResponse {
            content: "hello".into(),
            finish_reason: Some("stop".into()),
            prompt_tokens: Some(10),
            completion_tokens: Some(20),
            model: "gpt-4".into(),
        };
        assert_eq!(resp.content, "hello");
        assert_eq!(resp.finish_reason, Some("stop".into()));
        assert_eq!(resp.prompt_tokens, Some(10));
        assert_eq!(resp.completion_tokens, Some(20));
    }

    #[test]
    fn test_completion_response_serde() {
        let resp = CompletionResponse {
            content: "hi".into(),
            finish_reason: None,
            prompt_tokens: None,
            completion_tokens: None,
            model: "m".into(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let back: CompletionResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(resp.content, back.content);
        assert_eq!(resp.model, back.model);
    }

    #[test]
    fn test_token_usage_default() {
        let usage = TokenUsage::default();
        assert_eq!(usage.prompt_tokens, 0);
        assert_eq!(usage.completion_tokens, 0);
        assert_eq!(usage.total_tokens, 0);
    }

    #[test]
    fn test_token_usage_construction() {
        let usage = TokenUsage { prompt_tokens: 100, completion_tokens: 50, total_tokens: 150 };
        assert_eq!(usage.prompt_tokens, 100);
        assert_eq!(usage.total_tokens, 150);
    }

    #[test]
    fn test_token_usage_serde() {
        let usage = TokenUsage { prompt_tokens: 10, completion_tokens: 20, total_tokens: 30 };
        let json = serde_json::to_string(&usage).unwrap();
        let back: TokenUsage = serde_json::from_str(&json).unwrap();
        assert_eq!(usage.total_tokens, back.total_tokens);
    }

    #[test]
    fn test_completion_request_multiple_messages() {
        let messages = vec![
            ChatMessage { role: ChatRole::System, content: "sys".into(), images: None },
            ChatMessage { role: ChatRole::User, content: "user1".into(), images: None },
            ChatMessage { role: ChatRole::Assistant, content: "asst1".into(), images: None },
            ChatMessage { role: ChatRole::User, content: "user2".into(), images: None },
        ];
        let req = CompletionRequest::new(messages.clone(), "model", 200);
        assert_eq!(req.messages.len(), 4);
    }

    #[test]
    fn test_chat_message_fields() {
        let msg1 = ChatMessage { role: ChatRole::User, content: "hi".into(), images: None };
        let msg2 = ChatMessage { role: ChatRole::User, content: "hi".into(), images: None };
        assert_eq!(msg1.role, msg2.role);
        assert_eq!(msg1.content, msg2.content);
    }

    #[test]
    fn test_completion_response_fields() {
        let r1 = CompletionResponse {
            content: "a".into(),
            finish_reason: None,
            prompt_tokens: None,
            completion_tokens: None,
            model: "m".into(),
        };
        let r2 = CompletionResponse {
            content: "a".into(),
            finish_reason: None,
            prompt_tokens: None,
            completion_tokens: None,
            model: "m".into(),
        };
        assert_eq!(r1.content, r2.content);
        assert_eq!(r1.model, r2.model);
    }
}
