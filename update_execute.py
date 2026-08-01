import sys

with open("/home/gagan/Workspace/NexusAOS/src/runtime/kernel.rs", "r") as f:
    lines = f.readlines()

start_idx = -1
end_idx = -1
for i, line in enumerate(lines):
    if "pub async fn execute_task(" in line:
        start_idx = i
    if start_idx != -1 and line.strip() == "}":
        # check if it closes execute_task or emit_event?
        if "async fn emit_event" in lines[i+2]:
            end_idx = i
            break
        
print(f"start: {start_idx}, end: {end_idx}")

new_execute_task = """    pub async fn execute_task(
        &self,
        task_id: &TaskId,
    ) -> Result<crate::task::TaskOutcome, NexusError> {
        // 1. Get request and verify state
        let task = {
            let proj = self.projection.read().await;
            proj.tasks.get(task_id).cloned().ok_or_else(|| {
                NexusError::Task(TaskError::NotFound {
                    id: task_id.to_string(),
                })
            })?
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

        // Call Planner
        let planner = self.provider_registry.get(&crate::state::ModelRole::Planner)
            .ok_or_else(|| NexusError::Provider(crate::error::ProviderError::NotFound { name: "Planner".into() }))?;

        let req = CompletionRequest::new(vec![
            ChatMessage { role: ChatRole::System, content: "You are a planner.".to_string(), images: None },
            ChatMessage { role: ChatRole::User, content: input_text.clone(), images: None }
        ], planner.name(), 1000);

        self.emit_event(Event::new(*task_id, EventKind::ModelRequested, EventPayload::ModelRequest {
            role: "Planner".to_string(),
            prompt_tokens: 0,
            context_budget: 1000,
        }, "kernel".to_string())).await?;

        let plan_resp = planner.complete(req).await.map_err(|e| {
            NexusError::Provider(e)
        })?;

        self.emit_event(Event::new(*task_id, EventKind::ModelResponded, EventPayload::ModelResponse {
            role: "Planner".to_string(),
            response_tokens: plan_resp.completion_tokens.unwrap_or(0),
            content: plan_resp.content.clone(),
        }, "kernel".to_string())).await?;

        self.transition_task(task_id, TaskState::Planned).await?;

        let plan = plan_resp.content.to_lowercase();
        let requires_coder = plan.contains("code") || plan.contains("edit") || task.assigned_role == Some(crate::state::ModelRole::Coder);

        let mut final_output = plan_resp.content;

        if requires_coder {
            self.transition_task(task_id, TaskState::Executing).await?;

            let coder = self.provider_registry.get(&crate::state::ModelRole::Coder)
                .ok_or_else(|| NexusError::Provider(crate::error::ProviderError::NotFound { name: "Coder".into() }))?;

            let code_req = CompletionRequest::new(vec![
                ChatMessage { role: ChatRole::System, content: "You are a coder.".to_string(), images: None },
                ChatMessage { role: ChatRole::User, content: final_output.clone(), images: None }
            ], coder.name(), 2000);

            self.emit_event(Event::new(*task_id, EventKind::ModelRequested, EventPayload::ModelRequest {
                role: "Coder".to_string(),
                prompt_tokens: 0,
                context_budget: 2000,
            }, "kernel".to_string())).await?;

            let code_resp = coder.complete(code_req).await.map_err(|e| {
                NexusError::Provider(e)
            })?;

            self.emit_event(Event::new(*task_id, EventKind::ModelResponded, EventPayload::ModelResponse {
                role: "Coder".to_string(),
                response_tokens: code_resp.completion_tokens.unwrap_or(0),
                content: code_resp.content.clone(),
            }, "kernel".to_string())).await?;

            final_output = code_resp.content.clone();

            // Reviewer
            if let Some(reviewer) = self.provider_registry.get(&crate::state::ModelRole::Reviewer) {
                let rev_req = CompletionRequest::new(vec![
                    ChatMessage { role: ChatRole::System, content: "You are a reviewer.".to_string(), images: None },
                    ChatMessage { role: ChatRole::User, content: final_output.clone(), images: None }
                ], reviewer.name(), 2000);

                self.emit_event(Event::new(*task_id, EventKind::ModelRequested, EventPayload::ModelRequest {
                    role: "Reviewer".to_string(),
                    prompt_tokens: 0,
                    context_budget: 2000,
                }, "kernel".to_string())).await?;

                let rev_resp = reviewer.complete(rev_req).await.map_err(|e| {
                    NexusError::Provider(e)
                })?;

                self.emit_event(Event::new(*task_id, EventKind::ModelResponded, EventPayload::ModelResponse {
                    role: "Reviewer".to_string(),
                    response_tokens: rev_resp.completion_tokens.unwrap_or(0),
                    content: rev_resp.content.clone(),
                }, "kernel".to_string())).await?;

                final_output = format!("{}\\nReview: {}", final_output, rev_resp.content);
            }
        }

        if final_output.contains("TOOL:") {
            let tool_name = "dummy_tool";
            let tool_req = crate::tools::executor::ToolRequest {
                tool_name: tool_name.to_string(),
                arguments: serde_json::json!({}),
            };
            self.emit_event(Event::new(*task_id, EventKind::ToolRequested, EventPayload::ToolCall {
                tool_name: tool_name.to_string(),
                arguments: serde_json::json!({}),
            }, "kernel".to_string())).await?;

            match self.tool_broker.execute(&tool_req).await {
                Ok(crate::tools::broker::BrokerResult::Completed(res)) => {
                    self.emit_event(Event::new(*task_id, EventKind::ToolCompleted, EventPayload::ToolResult {
                        tool_name: tool_name.to_string(),
                        success: res.success,
                        output: res.output,
                    }, "kernel".to_string())).await?;
                }
                Ok(crate::tools::broker::BrokerResult::Denied(reason)) => {
                    self.emit_event(Event::new(*task_id, EventKind::ToolFailed, EventPayload::ToolResult {
                        tool_name: tool_name.to_string(),
                        success: false,
                        output: format!("Denied: {}", reason),
                    }, "kernel".to_string())).await?;
                }
                Ok(crate::tools::broker::BrokerResult::RequiresConfirmation(reason)) => {
                    self.emit_event(Event::new(*task_id, EventKind::ToolFailed, EventPayload::ToolResult {
                        tool_name: tool_name.to_string(),
                        success: false,
                        output: format!("Requires confirmation: {}", reason),
                    }, "kernel".to_string())).await?;
                }
                Err(e) => {
                    self.emit_event(Event::new(*task_id, EventKind::ToolFailed, EventPayload::ToolResult {
                        tool_name: tool_name.to_string(),
                        success: false,
                        output: e.to_string(),
                    }, "kernel".to_string())).await?;
                }
            }
        }

        let current_state = self.task_state(task_id).await?;
        if current_state == TaskState::Planned {
            self.transition_task(task_id, TaskState::Executing).await?;
        }
        self.transition_task(task_id, TaskState::Completed).await?;

        Ok(crate::task::TaskOutcome {
            task_id: *task_id,
            success: true,
            output: Some(final_output),
            error: None,
            completed_at: chrono::Utc::now(),
        })
    }
"""

if start_idx != -1 and end_idx != -1:
    lines = lines[:start_idx] + [new_execute_task] + lines[end_idx+1:]
    with open("/home/gagan/Workspace/NexusAOS/src/runtime/kernel.rs", "w") as f:
        f.writelines(lines)
else:
    print("Could not find start or end index")

