use crate::provider::{AiError, ChatMessage, ChatRequest, ModelProvider};
use futures::StreamExt;
use nexusaos_wconfig::settings::GlobalSettings;
use nexusaos_wps::broker::Broker;
use nexusaos_wps::events::{WaveEvent, EVENT_BUILDER_OUTPUT};
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use serde_json::json;

/// A handle for receiving streaming chunks from an AI response.
pub struct StreamHandle {
    pub rx: mpsc::Receiver<Result<String, AiError>>,
}

impl StreamHandle {
    pub fn new(rx: mpsc::Receiver<Result<String, AiError>>) -> Self {
        Self { rx }
    }

    /// Try to receive the next chunk without blocking.
    pub fn try_recv(&mut self) -> Option<Result<String, AiError>> {
        self.rx.try_recv().ok()
    }
}

pub struct ChatSession {
    provider: Arc<dyn ModelProvider>,
    settings: Arc<Mutex<GlobalSettings>>,
    broker: Arc<Broker>,
    pub history: Arc<Mutex<Vec<ChatMessage>>>,
}

impl ChatSession {
    pub fn new(
        provider: Arc<dyn ModelProvider>,
        settings: Arc<Mutex<GlobalSettings>>,
        broker: Arc<Broker>,
    ) -> Self {
        Self {
            provider,
            settings,
            broker,
            history: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Send a message and return a StreamHandle for receiving chunks.
    /// The caller is responsible for polling the stream.
    pub async fn send_message_stream(&self, text: &str) -> Result<StreamHandle, AiError> {
        let req = {
            let mut history = self.history.lock().await;
            history.push(ChatMessage {
                role: "user".to_string(),
                content: text.to_string(),
            });

            let _settings = self.settings.lock().await;
            
            ChatRequest {
                messages: history.clone(),
                model: "default-model".to_string(),
                max_tokens: Some(1024),
            }
        }; // Lock dropped here

        let (tx, rx) = mpsc::channel(32);
        let provider = self.provider.clone();
        let broker = self.broker.clone();
        let history = self.history.clone();

        // Spawn the streaming task
        tokio::spawn(async move {
            let mut stream = match provider.stream_chat(req).await {
                Ok(s) => s,
                Err(e) => {
                    let _ = tx.send(Err(e)).await;
                    return;
                }
            };

            let mut ai_response = String::new();
            while let Some(chunk_res) = stream.next().await {
                match chunk_res {
                    Ok(chunk) => {
                        ai_response.push_str(&chunk);
                        // Publish to broker for other consumers
                        broker.publish(WaveEvent::global(
                            EVENT_BUILDER_OUTPUT,
                            json!({ "content": chunk }),
                        ));
                        // Send to GUI stream
                        if tx.send(Ok(chunk)).await.is_err() {
                            break; // Receiver dropped
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(e)).await;
                        break;
                    }
                }
            }

            // Append complete response to history
            let mut history_lock = history.lock().await;
            history_lock.push(ChatMessage {
                role: "assistant".to_string(),
                content: ai_response,
            });
        });

        Ok(StreamHandle::new(rx))
    }

    /// Legacy method for non-streaming use (blocks until complete).
    pub async fn send_message(&self, text: &str) -> Result<(), AiError> {
        let mut handle = self.send_message_stream(text).await?;
        while let Some(chunk) = handle.rx.recv().await {
            chunk?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use futures::stream::BoxStream;

    struct MockProvider;
    
    #[async_trait]
    impl ModelProvider for MockProvider {
        async fn stream_chat(&self, _req: ChatRequest) -> Result<BoxStream<'static, Result<String, AiError>>, AiError> {
            Ok(Box::pin(futures::stream::iter(vec![
                Ok("Hello ".to_string()),
                Ok("World".to_string()),
            ])))
        }
    }

    #[tokio::test]
    async fn test_chat_session() {
        let provider = Arc::new(MockProvider);
        let settings = Arc::new(Mutex::new(GlobalSettings::default()));
        let broker = Broker::new(10);
        let session = ChatSession::new(provider, settings, broker);

        session.send_message("Hi").await.unwrap();
        let history = session.history.lock().await;
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].role, "user");
        assert_eq!(history[1].content, "Hello World");
    }

    #[tokio::test]
    async fn test_chat_session_stream() {
        let provider = Arc::new(MockProvider);
        let settings = Arc::new(Mutex::new(GlobalSettings::default()));
        let broker = Broker::new(10);
        let session = ChatSession::new(provider, settings, broker);

        let mut handle = session.send_message_stream("Hi").await.unwrap();
        let mut chunks = Vec::new();
        while let Some(chunk) = handle.rx.recv().await {
            chunks.push(chunk.unwrap());
        }
        assert_eq!(chunks, vec!["Hello ", "World"]);
    }
}
