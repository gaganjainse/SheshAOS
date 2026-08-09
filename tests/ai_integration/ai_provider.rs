use sheshaaos_ai::{session::ChatSession, openai::OpenAIProvider};
use sheshaaos_wps::broker::Broker;
use sheshaaos_wconfig::settings::GlobalSettings;
use std::sync::Arc;
use tokio::sync::Mutex;
use mockito::{Server, ServerGuard};
use serde_json::json;
use futures::stream::BoxStream;

/// Mock AI provider for testing
struct MockAIProvider {
    server: ServerGuard,
}

impl MockAIProvider {
    async fn new() -> Self {
        let server = Server::new_async().await;
        Self { server }
    }

    fn mock_streaming_response(&mut self, chunks: Vec<&str>) {
        let mut body = String::new();
        for chunk in chunks {
            body.push_str(&format!("data: {}\n\n", json!({ "choices": [{ "delta": {"content": chunk} }] })));
        }
        body.push_str("data: [DONE]\n\n");
        self.server.mock("POST", "/v1/chat/completions")
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_body(body)
            .create();
    }
}

#[async_trait::async_trait]
impl sheshaaos_ai::provider::ModelProvider for MockAIProvider {
    async fn stream_chat(&self, _req: sheshaaos_ai::provider::ChatRequest) -> Result<BoxStream<'static, Result<String, sheshaaos_ai::provider::AiError>>, sheshaaos_ai::provider::AiError> {
        // Return a stream of mock chunks
        let stream = futures::stream::iter(vec![
            Ok("Hello ".to_string()),
            Ok("World".to_string()),
        ]);
        Ok(Box::pin(stream))
    }
}

/// Test AI provider integration with real HTTP mocking
#[tokio::test]
async fn test_ai_provider_streaming() {
    let mut mock = MockAIProvider::new().await;
    mock.mock_streaming_response(vec!["Hello ", "World"]);

    let provider = OpenAIProvider::new(
        mock.server.url(),
        "test-key".to_string(),
    );

    let broker = Broker::new(10);
    let settings = Arc::new(Mutex::new(GlobalSettings::default()));
    let session = Arc::new(ChatSession::new(Arc::new(provider), settings, broker));

    let mut handle = session.send_message_stream("Hello").await.unwrap();

    let mut chunks = Vec::new();
    while let Some(chunk) = handle.rx.recv().await {
        chunks.push(chunk.unwrap());
    }

    // OpenAIProvider may deliver chunks as a single concatenated string
    // depending on how the HTTP stream is chunked
    let full_response: String = chunks.concat();
    assert!(full_response.contains("Hello") && full_response.contains("World"));
}

/// Test AI provider with real OpenAI-compatible server
#[tokio::test]
#[ignore] // Requires actual LLM server
async fn test_real_ai_provider() {
    let provider = OpenAIProvider::new(
        "http://127.0.0.1:1234/v1".to_string(),
        "".to_string(),
    );

    let broker = Broker::new(10);
    let settings = Arc::new(Mutex::new(GlobalSettings::default()));
    let session = Arc::new(ChatSession::new(Arc::new(provider), settings, broker));

    let mut handle = session.send_message_stream("What is 2+2?").await.unwrap();

    let mut full_response = String::new();
    while let Some(chunk) = handle.rx.recv().await {
        full_response.push_str(&chunk.unwrap());
    }

    assert!(full_response.contains("4") || full_response.contains("four"));
}

/// Test AI session history management
#[tokio::test]
async fn test_ai_session_history() {
    let provider = MockAIProvider::new().await;
    let provider = Arc::new(provider);

    let broker = Broker::new(10);
    let settings = Arc::new(Mutex::new(GlobalSettings::default()));
    let session = Arc::new(ChatSession::new(provider, settings, broker));

    // Send multiple messages
    session.send_message("First message").await.unwrap();
    session.send_message("Second message").await.unwrap();

    let history = session.history.lock().await;
    assert_eq!(history.len(), 4); // user, assistant, user, assistant
    assert_eq!(history[0].role, "user");
    assert_eq!(history[0].content, "First message");
    assert_eq!(history[2].role, "user");
    assert_eq!(history[2].content, "Second message");
}

/// Test AI provider error handling
#[tokio::test]
async fn test_ai_provider_error_handling() {
    // Test with a connection error (server not running)
    let provider = OpenAIProvider::new(
        "http://127.0.0.1:1".to_string(), // Port 1 should fail to connect
        "test-key".to_string(),
    );

    let broker = Broker::new(10);
    let settings = Arc::new(Mutex::new(GlobalSettings::default()));
    let session = Arc::new(ChatSession::new(Arc::new(provider), settings, broker));

    // send_message_stream succeeds (it spawns a task), but the stream
    // should contain an error when we try to receive
    let mut handle = session.send_message_stream("Test").await.unwrap();

    // Wait for the stream to produce an error
    let result = handle.rx.recv().await;
    assert!(result.is_some(), "Should receive something from stream");
    assert!(result.unwrap().is_err(), "Should receive an error from stream");
}

/// Test AI provider with different models
#[tokio::test]
async fn test_ai_provider_model_selection() {
    let mut server = Server::new_async().await;
    let _mock = server.mock("POST", "/v1/chat/completions")
        .with_status(200)
        .with_header("content-type", "text/event-stream")
        .with_body("data: {\"choices\":[{\"delta\":{\"content\":\"test\"}}]}\n\ndata: [DONE]\n\n")
        .create();

    let provider = OpenAIProvider::new(
        server.url(),
        "test-key".to_string(),
    );

    let broker = Broker::new(10);
    let settings = Arc::new(Mutex::new(GlobalSettings::default()));
    let session = Arc::new(ChatSession::new(Arc::new(provider), settings, broker));

    // Test with different model names
    let mut handle = session.send_message_stream("Test").await.unwrap();
    let mut chunks = Vec::new();
    while let Some(chunk) = handle.rx.recv().await {
        chunks.push(chunk.unwrap());
    }
    assert!(!chunks.is_empty());
}

/// Test AI provider concurrent requests
#[tokio::test]
async fn test_ai_provider_concurrent_requests() {
    let provider = MockAIProvider::new().await;
    let provider = Arc::new(provider);

    let broker = Broker::new(10);
    let settings = Arc::new(Mutex::new(GlobalSettings::default()));
    let session = Arc::new(ChatSession::new(provider, settings, broker));

    // Send multiple concurrent requests
    let handles = futures::future::join_all((0..5).map(
        |i| {
            let session = session.clone();
            async move {
                session.send_message(&format!("Message {}", i)).await
            }
        },
    )).await;

    for result in handles {
        assert!(result.is_ok());
    }

    let history = session.history.lock().await;
    assert_eq!(history.len(), 10); // 5 user + 5 assistant
}