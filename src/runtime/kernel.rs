use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use chrono::Utc;
use tokio::sync::RwLock;

use crate::{
    error::{NexusError, TaskError},
    events::{Event, EventKind, EventPayload},
    model::{
        registry::ProviderRegistry,
        types::{ChatMessage, ChatRole, CompletionRequest},
    },
    policy::PolicyEngine,
    router::TaskRouter,
    state::{TaskRecord, TaskState},
    task::{TaskId, TaskInput, TaskRequest},
    tools::broker::ToolBroker,
};

#[async_trait]
pub trait EventStore: Send + Sync {
    async fn append(&self, event: Event) -> Result<(), NexusError>;
    async fn get_all_events(&self) -> Result<Vec<Event>, NexusError>;
    async fn get_task_events(&self, task_id: &TaskId) -> Result<Vec<Event>, NexusError>;
}

#[derive(Debug, Default)]
pub struct TaskProjection {
    pub tasks: HashMap<TaskId, TaskRecord>,
}

impl TaskProjection {
    pub fn new() -> Self {
        Self { tasks: HashMap::new() }
    }
}

/// The NexusAOS kernel — owns task lifecycle, policy, and state.
pub struct Kernel {
    event_store: Arc<dyn EventStore>,
    projection: Arc<RwLock<TaskProjection>>,
    policy: RwLock<PolicyEngine>,
    provider_registry: Arc<ProviderRegistry>,
    tool_broker: Arc<ToolBroker>,
}

impl Kernel {
    /// Create a new kernel with the given components.
    pub async fn new(
        event_store: Arc<dyn EventStore>,
        policy: PolicyEngine,
        provider_registry: Arc<ProviderRegistry>,
        tool_broker: Arc<ToolBroker>,
    ) -> Result<Self, NexusError> {
        let kernel = Self {
            event_store,
            projection: Arc::new(RwLock::new(TaskProjection::new())),
            policy: RwLock::new(policy),
            provider_registry,
            tool_broker,
        };
        // Rebuild projection from events could happen here if we used replay engine
        Ok(kernel)
    }

    /// Submit a new task. Returns the TaskId.
    pub async fn submit_task(&self, input: TaskInput) -> Result<TaskId, NexusError> {
        let task_id = TaskId::new();
        let request = TaskRequest::new(input.clone());

        // Policy check for task creation
        let decision = {
            let policy = self.policy.read().await;
            policy.evaluate("task.create")
        };

        if decision.is_denied() {
            return Err(NexusError::Policy(crate::error::PolicyError::Denied {
                reason: "Task creation denied by policy".into(),
            }));
        }

        // Emit TaskCreated event
        let event_payload = EventPayload::TaskCreated {
            request: serde_json::to_value(&request).unwrap_or(serde_json::Value::Null),
        };
        let event =
            Event::new(task_id, EventKind::TaskCreated, event_payload, "kernel".to_string());
        self.emit_event(event).await?;

        // Initialize state in projection
        let record = TaskRecord {
            task_id,
            request,
            current_state: TaskState::Received,
            assigned_role: None,
            state_history: vec![(TaskState::Received, Utc::now())],
        };

        {
            let mut proj = self.projection.write().await;
            proj.tasks.insert(task_id, record);
        }

        // Classify via router
        let has_images = matches!(input, TaskInput::Vision { .. });
        let input_text = match &input {
            TaskInput::Text(t) => t.clone(),
            TaskInput::Vision { text, .. } => text.clone(),
            TaskInput::Multi { .. } => "multi".to_string(),
        };

        let route_decision = TaskRouter::route(&input_text, has_images);

        // Update state to Classified
        let class_payload = EventPayload::StateChanged {
            from: "Received".to_string(),
            to: "Classified".to_string(),
        };
        let class_event =
            Event::new(task_id, EventKind::TaskClassified, class_payload, "router".to_string());
        self.emit_event(class_event).await?;

        {
            let mut proj = self.projection.write().await;
            if let Some(task) = proj.tasks.get_mut(&task_id) {
                task.current_state = TaskState::Classified;
                task.assigned_role = Some(route_decision.primary_role);
                task.state_history.push((TaskState::Classified, Utc::now()));
            }
        }

        Ok(task_id)
    }

    /// Get the current state of a task.
    pub async fn task_state(&self, id: &TaskId) -> Result<TaskState, NexusError> {
        let proj = self.projection.read().await;
        if let Some(task) = proj.tasks.get(id) {
            Ok(task.current_state)
        } else {
            Err(NexusError::Task(TaskError::NotFound { id: id.to_string() }))
        }
    }

    /// Get all tasks in a given state.
    pub async fn tasks_in_state(&self, state: &TaskState) -> Vec<TaskId> {
        let proj = self.projection.read().await;
        proj.tasks
            .iter()
            .filter(|(_, record)| record.current_state == *state)
            .map(|(id, _)| *id)
            .collect()
    }

    /// Transition a task to a new state (with validation).
    pub async fn transition_task(
        &self,
        task_id: &TaskId,
        new_state: TaskState,
    ) -> Result<(), NexusError> {
        let mut proj = self.projection.write().await;
        let task = proj
            .tasks
            .get_mut(task_id)
            .ok_or_else(|| NexusError::Task(TaskError::NotFound { id: task_id.to_string() }))?;

        let current_state = task.current_state;

        if !current_state.can_transition_to(&new_state) {
            return Err(NexusError::Task(TaskError::InvalidTransition {
                from: current_state.to_string(),
                to: new_state.to_string(),
            }));
        }

        let event_payload = EventPayload::StateChanged {
            from: current_state.to_string(),
            to: new_state.to_string(),
        };
        let event =
            Event::new(*task_id, EventKind::TaskStateChanged, event_payload, "kernel".to_string());
        self.emit_event(event).await?;

        task.current_state = new_state;
        task.state_history.push((new_state, Utc::now()));

        Ok(())
    }

    /// Get task count.
    pub async fn task_count(&self) -> usize {
        let proj = self.projection.read().await;
        proj.tasks.len()
    }

    /// Execute a task through the multi-model workflow (Planner -> Coder -> Reviewer -> Tool Broker).
    pub async fn execute_task(
        &self,
        task_id: &TaskId,
    ) -> Result<crate::task::TaskOutcome, NexusError> {
        // 1. Get request and verify state
        let task = {
            let proj = self.projection.read().await;
            proj.tasks
                .get(task_id)
                .cloned()
                .ok_or_else(|| NexusError::Task(TaskError::NotFound { id: task_id.to_string() }))?
        };

        if task.current_state != TaskState::Classified {
            return Err(NexusError::Task(TaskError::InvalidTransition {
                from: task.current_state.to_string(),
                to: TaskState::Planned.to_string(),
            }));
        }

        let input_text = match &task.request.input {
            TaskInput::Text(t) => t.clone(),
            TaskInput::Vision { text, .. } => text.clone(),
            TaskInput::Multi { .. } => "Multi-part task input".to_string(),
        };

        // 2. Transition Classified -> Planned is done AFTER we get the plan, or before?
        // Let's do it before we call the planner so the state is tracked? Wait, the prompt says "Move state: Classified -> Planned by calling the Planner model provider". We can transition first.

        let planner =
            self.provider_registry.get(&crate::state::ModelRole::Planner).ok_or_else(|| {
                NexusError::Provider(crate::error::ProviderError::Unavailable {
                    name: "Planner".into(),
                })
            })?;

        let req = CompletionRequest::new(
            vec![
                ChatMessage {
                    role: ChatRole::System,
                    content: "You are a planner.".to_string(),
                    images: None,
                },
                ChatMessage { role: ChatRole::User, content: input_text, images: None },
            ],
            planner.name(),
            planner.max_context(),
        );

        let event_req = Event::new(
            *task_id,
            EventKind::ModelRequested,
            EventPayload::ModelRequest {
                role: "Planner".to_string(),
                prompt_tokens: 0,
                context_budget: planner.max_context(),
            },
            "kernel".to_string(),
        );
        self.emit_event(event_req).await?;

        let plan_resp = planner.complete(req).await.map_err(NexusError::Provider)?;

        let event_resp = Event::new(
            *task_id,
            EventKind::ModelResponded,
            EventPayload::ModelResponse {
                role: "Planner".to_string(),
                response_tokens: plan_resp.completion_tokens.unwrap_or(0),
                content: plan_resp.content.clone(),
            },
            "kernel".to_string(),
        );
        self.emit_event(event_resp).await?;

        self.transition_task(task_id, TaskState::Planned).await?;

        let plan = plan_resp.content.to_lowercase();
        let requires_coder = plan.contains("code")
            || plan.contains("edit")
            || task.assigned_role == Some(crate::state::ModelRole::Coder);

        let mut final_output = plan_resp.content;

        if requires_coder {
            self.transition_task(task_id, TaskState::Executing).await?;

            let coder =
                self.provider_registry.get(&crate::state::ModelRole::Coder).ok_or_else(|| {
                    NexusError::Provider(crate::error::ProviderError::Unavailable {
                        name: "Coder".into(),
                    })
                })?;

            let code_req = CompletionRequest::new(
                vec![
                    ChatMessage {
                        role: ChatRole::System,
                        content: "You are a coder.".to_string(),
                        images: None,
                    },
                    ChatMessage {
                        role: ChatRole::User,
                        content: final_output.clone(),
                        images: None,
                    },
                ],
                coder.name(),
                coder.max_context(),
            );

            self.emit_event(Event::new(
                *task_id,
                EventKind::ModelRequested,
                EventPayload::ModelRequest {
                    role: "Coder".to_string(),
                    prompt_tokens: 0,
                    context_budget: coder.max_context(),
                },
                "kernel".to_string(),
            ))
            .await?;

            let code_resp = coder.complete(code_req).await.map_err(NexusError::Provider)?;

            self.emit_event(Event::new(
                *task_id,
                EventKind::ModelResponded,
                EventPayload::ModelResponse {
                    role: "Coder".to_string(),
                    response_tokens: code_resp.completion_tokens.unwrap_or(0),
                    content: code_resp.content.clone(),
                },
                "kernel".to_string(),
            ))
            .await?;

            final_output = code_resp.content.clone();

            // Reviewer
            if let Some(reviewer) = self.provider_registry.get(&crate::state::ModelRole::Reviewer) {
                let rev_req = CompletionRequest::new(
                    vec![
                        ChatMessage {
                            role: ChatRole::System,
                            content: "You are a reviewer.".to_string(),
                            images: None,
                        },
                        ChatMessage {
                            role: ChatRole::User,
                            content: final_output.clone(),
                            images: None,
                        },
                    ],
                    reviewer.name(),
                    reviewer.max_context(),
                );

                self.emit_event(Event::new(
                    *task_id,
                    EventKind::ModelRequested,
                    EventPayload::ModelRequest {
                        role: "Reviewer".to_string(),
                        prompt_tokens: 0,
                        context_budget: reviewer.max_context(),
                    },
                    "kernel".to_string(),
                ))
                .await?;

                let rev_resp = reviewer.complete(rev_req).await.map_err(NexusError::Provider)?;

                self.emit_event(Event::new(
                    *task_id,
                    EventKind::ModelResponded,
                    EventPayload::ModelResponse {
                        role: "Reviewer".to_string(),
                        response_tokens: rev_resp.completion_tokens.unwrap_or(0),
                        content: rev_resp.content.clone(),
                    },
                    "kernel".to_string(),
                ))
                .await?;

                final_output = format!("{}\nReview: {}", final_output, rev_resp.content);
            }
        }

        let mut task_success = true;

        if final_output.contains("TOOL:") {
            let tool_line = final_output
                .lines()
                .find(|l| l.contains("TOOL:"))
                .unwrap_or("");
            let tool_name = tool_line
                .split(':')
                .nth(1)
                .and_then(|s| s.trim().split_whitespace().next())
                .unwrap_or("unknown");

            let tool_req = crate::tools::executor::ToolRequest {
                tool_name: tool_name.to_string(),
                arguments: serde_json::json!({}),
            };
            self.emit_event(Event::new(
                *task_id,
                EventKind::ToolRequested,
                EventPayload::ToolCall {
                    tool_name: tool_name.to_string(),
                    arguments: serde_json::json!({}),
                },
                "kernel".to_string(),
            ))
            .await?;

            match self.tool_broker.execute(&tool_req).await {
                Ok(crate::tools::broker::BrokerResult::Completed(res)) => {
                    self.emit_event(Event::new(
                        *task_id,
                        EventKind::ToolCompleted,
                        EventPayload::ToolResult {
                            tool_name: tool_name.to_string(),
                            success: res.success,
                            output: res.output,
                        },
                        "kernel".to_string(),
                    ))
                    .await?;
                }
                Ok(crate::tools::broker::BrokerResult::Denied(reason)) => {
                    task_success = false;
                    self.emit_event(Event::new(
                        *task_id,
                        EventKind::ToolFailed,
                        EventPayload::ToolResult {
                            tool_name: tool_name.to_string(),
                            success: false,
                            output: format!("Denied: {}", reason),
                        },
                        "kernel".to_string(),
                    ))
                    .await?;
                }
                Ok(crate::tools::broker::BrokerResult::RequiresConfirmation(reason)) => {
                    task_success = false;
                    self.emit_event(Event::new(
                        *task_id,
                        EventKind::ToolFailed,
                        EventPayload::ToolResult {
                            tool_name: tool_name.to_string(),
                            success: false,
                            output: format!("Requires confirmation: {}", reason),
                        },
                        "kernel".to_string(),
                    ))
                    .await?;
                }
                Err(e) => {
                    task_success = false;
                    self.emit_event(Event::new(
                        *task_id,
                        EventKind::ToolFailed,
                        EventPayload::ToolResult {
                            tool_name: tool_name.to_string(),
                            success: false,
                            output: e.to_string(),
                        },
                        "kernel".to_string(),
                    ))
                    .await?;
                }
            }
        }

        let current_state = self.task_state(task_id).await?;
        if current_state == TaskState::Planned {
            self.transition_task(task_id, TaskState::Executing).await?;
        }
        if task_success {
            self.transition_task(task_id, TaskState::Completed).await?;
        } else {
            self.transition_task(task_id, TaskState::Failed).await?;
        }

        Ok(crate::task::TaskOutcome {
            task_id: *task_id,
            success: task_success,
            output: Some(final_output),
            error: if task_success {
                None
            } else {
                Some("Tool execution failed".to_string())
            },
            completed_at: Utc::now(),
        })
    }

    // Helper: emit an event
    async fn emit_event(&self, event: Event) -> Result<(), NexusError> {
        self.event_store.append(event).await
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::sync::Mutex;

    use async_trait::async_trait;

    use super::*;
    use crate::{
        model::{
            provider::ModelProvider,
            types::{CompletionRequest, CompletionResponse},
        },
        policy::TrustTier,
    };

    struct MockProvider {
        role: crate::state::ModelRole,
        content: String,
    }
    #[async_trait]
    impl ModelProvider for MockProvider {
        fn name(&self) -> &str {
            "mock"
        }
        fn role(&self) -> crate::state::ModelRole {
            self.role
        }
        fn max_context(&self) -> usize {
            100
        }
        fn supports_vision(&self) -> bool {
            false
        }
        async fn health_check(&self) -> Result<bool, crate::error::ProviderError> {
            Ok(true)
        }
        async fn complete(
            &self,
            _r: CompletionRequest,
        ) -> Result<CompletionResponse, crate::error::ProviderError> {
            Ok(CompletionResponse {
                content: self.content.clone(),
                finish_reason: None,
                prompt_tokens: None,
                completion_tokens: None,
                model: "mock".into(),
            })
        }
        async fn cancel(&self) -> Result<(), crate::error::ProviderError> {
            Ok(())
        }
    }

    struct MockEventStore {
        events: Mutex<Vec<Event>>,
    }

    impl MockEventStore {
        fn new() -> Self {
            Self { events: Mutex::new(Vec::new()) }
        }
    }

    #[async_trait]
    impl EventStore for MockEventStore {
        async fn append(&self, event: Event) -> Result<(), NexusError> {
            self.events.lock().unwrap().push(event);
            Ok(())
        }
        async fn get_all_events(&self) -> Result<Vec<Event>, NexusError> {
            Ok(self.events.lock().unwrap().clone())
        }
        async fn get_task_events(&self, task_id: &TaskId) -> Result<Vec<Event>, NexusError> {
            Ok(self
                .events
                .lock()
                .unwrap()
                .iter()
                .filter(|e| e.task_id == Some(*task_id))
                .cloned()
                .collect())
        }
    }

    #[tokio::test]
    async fn test_submit_task_allowed() {
        let store = Arc::new(MockEventStore::new());
        let rule = crate::policy::PolicyRule {
            name: "allow".into(),
            action_pattern: "*".into(),
            decision: "allow".into(),
            trust_tier: 0,
            description: None,
        };
        let policy = PolicyEngine::new(vec![rule], TrustTier::Autonomous);
        let registry = Arc::new(ProviderRegistry::new());
        let broker = Arc::new(ToolBroker::new(Arc::new(policy.clone())));
        let kernel = Kernel::new(store, policy, registry, broker).await.unwrap();

        let id = kernel.submit_task(TaskInput::Text("test".into())).await.unwrap();
        let state = kernel.task_state(&id).await.unwrap();
        assert_eq!(state, TaskState::Classified);
    }

    #[tokio::test]
    async fn test_submit_task_denied() {
        let store = Arc::new(MockEventStore::new());
        let policy = PolicyEngine::deny_all();
        let registry = Arc::new(ProviderRegistry::new());
        let broker = Arc::new(ToolBroker::new(Arc::new(policy.clone())));
        let kernel = Kernel::new(store, policy, registry, broker).await.unwrap();

        let result = kernel.submit_task(TaskInput::Text("test".into())).await;
        assert!(matches!(result, Err(NexusError::Policy(_))));
    }

    #[tokio::test]
    async fn test_task_transition() {
        let store = Arc::new(MockEventStore::new());
        let rule = crate::policy::PolicyRule {
            name: "allow".into(),
            action_pattern: "*".into(),
            decision: "allow".into(),
            trust_tier: 0,
            description: None,
        };
        let policy = PolicyEngine::new(vec![rule], TrustTier::Autonomous);
        let registry = Arc::new(ProviderRegistry::new());
        let broker = Arc::new(ToolBroker::new(Arc::new(policy.clone())));
        let kernel = Kernel::new(store, policy, registry, broker).await.unwrap();

        let id = kernel.submit_task(TaskInput::Text("test".into())).await.unwrap();
        kernel.transition_task(&id, TaskState::Planned).await.unwrap();
        assert_eq!(kernel.task_state(&id).await.unwrap(), TaskState::Planned);
    }

    #[tokio::test]
    async fn test_invalid_task_transition() {
        let store = Arc::new(MockEventStore::new());
        let rule = crate::policy::PolicyRule {
            name: "allow".into(),
            action_pattern: "*".into(),
            decision: "allow".into(),
            trust_tier: 0,
            description: None,
        };
        let policy = PolicyEngine::new(vec![rule], TrustTier::Autonomous);
        let registry = Arc::new(ProviderRegistry::new());
        let broker = Arc::new(ToolBroker::new(Arc::new(policy.clone())));
        let kernel = Kernel::new(store, policy, registry, broker).await.unwrap();

        let id = kernel.submit_task(TaskInput::Text("test".into())).await.unwrap();
        // Classified -> Completed is invalid
        let result = kernel.transition_task(&id, TaskState::Completed).await;
        assert!(matches!(result, Err(NexusError::Task(_))));
    }

    #[tokio::test]
    async fn test_execute_task() {
        let store = Arc::new(MockEventStore::new());
        let rule = crate::policy::PolicyRule {
            name: "allow".into(),
            action_pattern: "*".into(),
            decision: "allow".into(),
            trust_tier: 0,
            description: None,
        };
        let policy = PolicyEngine::new(vec![rule], TrustTier::Autonomous);

        let mut registry = ProviderRegistry::new();
        registry.register(Box::new(MockProvider {
            role: crate::state::ModelRole::Planner,
            content: "Need to write some code to fix this. TOOL: dummy".into(),
        }));
        registry.register(Box::new(MockProvider {
            role: crate::state::ModelRole::Coder,
            content: "Here is the code.".into(),
        }));
        registry.register(Box::new(MockProvider {
            role: crate::state::ModelRole::Reviewer,
            content: "Looks good.".into(),
        }));

        let broker = Arc::new(ToolBroker::new(Arc::new(policy.clone())));
        let kernel = Kernel::new(store, policy, Arc::new(registry), broker).await.unwrap();

        let id = kernel.submit_task(TaskInput::Text("fix this".into())).await.unwrap();

        // Execute task should run Planner -> Coder -> Reviewer
        let outcome = kernel.execute_task(&id).await.unwrap();
        assert!(outcome.success);
        let final_output = outcome.output.unwrap();
        assert!(final_output.contains("Here is the code."));
        assert!(final_output.contains("Review: Looks good."));

        let state = kernel.task_state(&id).await.unwrap();
        assert_eq!(state, TaskState::Completed);
    }
}
