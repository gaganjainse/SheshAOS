use std::{collections::HashMap, sync::Arc};

use tracing::{error, info, warn};

use super::executor::{ToolExecutor, ToolRequest, ToolResult};
use crate::{
    error::ToolError,
    policy::{PolicyDecision, PolicyEngine},
};

/// The result of a broker dispatch.
#[derive(Debug)]
pub enum BrokerResult {
    /// Tool executed successfully.
    Completed(ToolResult),
    /// Tool requires confirmation before execution.
    RequiresConfirmation(String),
    /// Tool was denied by policy.
    Denied(String),
}

/// Dispatches tool calls through policy checks and logging.
pub struct ToolBroker {
    executors: HashMap<String, Arc<dyn ToolExecutor>>,
    policy: Arc<PolicyEngine>,
}

impl ToolBroker {
    pub fn new(policy: Arc<PolicyEngine>) -> Self {
        Self { executors: HashMap::new(), policy }
    }

    /// Register a tool executor.
    pub fn register(&mut self, executor: Arc<dyn ToolExecutor>) {
        let name = executor.name().to_string();
        self.executors.insert(name, executor);
    }

    /// Execute a tool call with policy checks.
    /// Returns PolicyDecision if confirmation is needed, or executes if allowed.
    pub async fn execute(&self, request: &ToolRequest) -> Result<BrokerResult, ToolError> {
        let action = format!("{}.execute", request.tool_name);
        let decision = self.policy.evaluate(&action);

        info!(tool = %request.tool_name, action = %action, "Tool execution requested");

        match decision {
            PolicyDecision::Deny(reason) => {
                warn!(tool = %request.tool_name, reason = %reason, "Tool execution denied by policy");
                Ok(BrokerResult::Denied(reason))
            }
            PolicyDecision::RequireConfirmation(reason) => {
                info!(tool = %request.tool_name, reason = %reason, "Tool execution requires confirmation");
                Ok(BrokerResult::RequiresConfirmation(reason))
            }
            PolicyDecision::Allow => {
                let executor = self.executors.get(&request.tool_name).ok_or_else(|| {
                    error!(tool = %request.tool_name, "Tool not found");
                    ToolError::NotFound { name: request.tool_name.clone() }
                })?;

                info!(tool = %request.tool_name, "Executing tool");
                let result = executor.execute(request).await?;
                info!(tool = %request.tool_name, success = %result.success, "Tool execution completed");
                Ok(BrokerResult::Completed(result))
            }
        }
    }

    /// List available tools.
    pub fn available_tools(&self) -> Vec<String> {
        self.executors.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use serde_json::json;

    use super::*;
    use crate::policy::{PolicyRule, TrustTier};

    struct DummyTool;
    #[async_trait]
    impl ToolExecutor for DummyTool {
        fn name(&self) -> &str {
            "dummy"
        }
        fn description(&self) -> &str {
            "dummy"
        }
        fn is_destructive(&self) -> bool {
            false
        }
        async fn execute(&self, _req: &ToolRequest) -> Result<ToolResult, ToolError> {
            Ok(ToolResult { success: true, output: "ok".to_string(), data: None })
        }
    }

    #[tokio::test]
    async fn test_broker() {
        let rule = PolicyRule {
            name: "allow-dummy".to_string(),
            action_pattern: "dummy.execute".to_string(),
            decision: "allow".to_string(),
            trust_tier: 0,
            description: None,
        };
        let policy = PolicyEngine::new(vec![rule], TrustTier::Basic);

        let mut broker = ToolBroker::new(Arc::new(policy));
        broker.register(Arc::new(DummyTool));

        let req = ToolRequest { tool_name: "dummy".to_string(), arguments: json!({}) };

        let res = broker.execute(&req).await.unwrap();
        match res {
            BrokerResult::Completed(tr) => assert_eq!(tr.output, "ok"),
            _ => panic!("Expected Completed"),
        }
    }
}
