use iced::{Task, Element, Theme, Subscription};
use iced::keyboard;
use iced::event;
use crate::view;
use crate::terminal::TerminalState;
use sheshaaos_ai::session::{ChatSession, StreamHandle};
use sheshaaos_ai::openai::OpenAIProvider;
use sheshaaos_wps::broker::Broker;
use sheshaaos_wconfig::settings::GlobalSettings;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct NexusApp {
    pub terminal: TerminalState,
    pub active_tab: Tab,
    pub ai_input: String,
    pub ai_messages: Vec<ChatMessage>,
    pub ai_stream: Arc<Mutex<Option<StreamHandle>>>,
    pub ai_session: Option<Arc<ChatSession>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Terminal,
    AiChat,
}

#[derive(Debug, Clone)]
pub enum Message {
    KeyPressed(keyboard::Key, keyboard::Modifiers),
    CharInput(char),
    SwitchTab(Tab),
    Tick,
    AiInputChanged(String),
    AiSubmit,
    AiStreamReady,
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String, // "user" or "assistant"
    pub content: String,
    pub is_streaming: bool, // True while receiving chunks
}

impl NexusApp {
    pub fn new() -> (Self, Task<Message>) {
        let mut terminal = TerminalState::new();
        terminal.wire_pty();

        // Initialize AI session with default provider
        let ai_session = Self::create_ai_session();

        (
            NexusApp {
                terminal,
                active_tab: Tab::Terminal,
                ai_input: String::new(),
                ai_messages: vec![
                    ChatMessage {
                        role: "assistant".to_string(),
                        content: "Welcome to SheshaAOS AI!".to_string(),
                        is_streaming: false,
                    },
                    ChatMessage {
                        role: "assistant".to_string(),
                        content: "I can help you analyze errors, write scripts, or answer questions.".to_string(),
                        is_streaming: false,
                    },
                ],
                ai_stream: Arc::new(Mutex::new(None)),
                ai_session,
            },
            Task::none(),
        )
    }

    fn create_ai_session() -> Option<Arc<ChatSession>> {
        // Try to create OpenAI-compatible provider
        let provider = OpenAIProvider::new(
            "http://127.0.0.1:1234/v1".to_string(),
            "".to_string(), // No API key for local server
        );
        
        let broker = Broker::new(100);
        let settings = Arc::new(Mutex::new(GlobalSettings::default()));
        Some(Arc::new(ChatSession::new(Arc::new(provider), settings, broker)))
    }

    pub fn title(&self) -> String {
        format!("SheshaAOS — {}", self.terminal.title())
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::KeyPressed(key, modifiers) => {
                if self.active_tab == Tab::Terminal {
                    self.terminal.handle_key(key, modifiers);
                }
                Task::none()
            }
            Message::CharInput(c) => {
                if self.active_tab == Tab::Terminal {
                    self.terminal.handle_char(c);
                }
                Task::none()
            }
            Message::SwitchTab(tab) => {
                self.active_tab = tab;
                Task::none()
            }
            Message::Tick => {
                // Drain PTY output buffer and parse ANSI sequences
                self.terminal.poll_output();
                
                // Also poll AI stream for new chunks
                let mut stream_guard = self.ai_stream.blocking_lock();
                if let Some(ref mut stream) = *stream_guard {
                    let mut stream_ended = false;
                    while let Some(chunk) = stream.try_recv() {
                        match chunk {
                            Ok(text) => {
                                // Append to the last assistant message
                                if let Some(last) = self.ai_messages.last_mut() {
                                    if last.role == "assistant" && last.is_streaming {
                                        last.content.push_str(&text);
                                    }
                                }
                            }
                            Err(_) => {
                                // Stream ended with error
                                stream_ended = true;
                                break;
                            }
                        }
                    }
                    if stream_ended {
                        *stream_guard = None;
                        // Mark last message as not streaming
                        if let Some(last) = self.ai_messages.last_mut() {
                                    if last.role == "assistant" {
                                        last.is_streaming = false;
                                    }
                                }
                    }
                }
                Task::none()
            }
            Message::AiInputChanged(text) => {
                self.ai_input = text;
                Task::none()
            }
            Message::AiSubmit => {
                if !self.ai_input.is_empty() {
                    if let Some(session) = &self.ai_session {
                        let text = self.ai_input.clone();
                        self.ai_input.clear();
                        
                        // Add user message
                        self.ai_messages.push(ChatMessage {
                            role: "user".to_string(),
                            content: text.clone(),
                            is_streaming: false,
                        });
                        
                        // Add placeholder assistant message (streaming)
                        self.ai_messages.push(ChatMessage {
                            role: "assistant".to_string(),
                            content: String::new(),
                            is_streaming: true,
                        });
                        
                        // Start streaming in background
                        let session = session.clone();
                        let ai_stream = self.ai_stream.clone();
                        let fut = async move {
                            match session.send_message_stream(&text).await {
                                Ok(handle) => {
                                    let mut guard = ai_stream.lock().await;
                                    *guard = Some(handle);
                                }
                                Err(e) => {
                                    // Add error message
                                    eprintln!("AI stream error: {}", e);
                                }
                            }
                        };
                        return Task::future(fut).map(|_| Message::AiStreamReady);
                    } else {
                        // No AI session available
                        self.ai_messages.push(ChatMessage {
                            role: "assistant".to_string(),
                            content: "AI not available (no local LLM server at http://127.0.0.1:1234/v1)".to_string(),
                            is_streaming: false,
                        });
                    }
                }
                Task::none()
            }
            Message::AiStreamReady => Task::none(),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        view::render(self)
    }

    pub fn theme(&self) -> Theme {
        Theme::CatppuccinMocha
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let keyboard_sub = event::listen_with(|event, _status, _window_id| {
            match event {
                iced::Event::Keyboard(keyboard::Event::KeyPressed {
                    key,
                    modifiers,
                    ..
                }) => match &key {
                    keyboard::Key::Named(_) => {
                        Some(Message::KeyPressed(key.clone(), modifiers))
                    }
                    keyboard::Key::Character(c) => {
                        if modifiers.control() || modifiers.alt() {
                            Some(Message::KeyPressed(key.clone(), modifiers))
                        } else {
                            c.as_str().chars().next().map(Message::CharInput)
                        }
                    }
                    _ => None,
                },
                _ => None,
            }
        });

        let tick = iced::time::every(std::time::Duration::from_millis(16))
            .map(|_| Message::Tick);

        Subscription::batch([keyboard_sub, tick])
    }
}
