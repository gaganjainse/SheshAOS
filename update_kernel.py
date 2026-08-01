import re

with open("/home/gagan/Workspace/NexusAOS/src/runtime/kernel.rs", "r") as f:
    content = f.read()

# Make sure to add imports
imports = """use crate::model::registry::ProviderRegistry;
use crate::tools::broker::ToolBroker;
use crate::task::TaskOutcome;
use crate::model::types::{CompletionRequest, ChatMessage, ChatRole};
"""

content = content.replace("use std::collections::HashMap;", imports + "use std::collections::HashMap;")

# Update Kernel struct
kernel_struct_old = """pub struct Kernel {
    event_store: Arc<dyn EventStore>,
    projection: Arc<RwLock<TaskProjection>>,
    policy: RwLock<PolicyEngine>,
}"""

kernel_struct_new = """pub struct Kernel {
    event_store: Arc<dyn EventStore>,
    projection: Arc<RwLock<TaskProjection>>,
    policy: RwLock<PolicyEngine>,
    provider_registry: Arc<ProviderRegistry>,
    tool_broker: Arc<ToolBroker>,
}"""
content = content.replace(kernel_struct_old, kernel_struct_new)

kernel_new_old = """pub async fn new(
        event_store: Arc<dyn EventStore>,
        policy: PolicyEngine,
    ) -> Result<Self, NexusError> {
        let kernel = Self {
            event_store,
            projection: Arc::new(RwLock::new(TaskProjection::new())),
            policy: RwLock::new(policy),
        };"""

kernel_new_new = """pub async fn new(
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
        };"""
content = content.replace(kernel_new_old, kernel_new_new)

with open("/home/gagan/Workspace/NexusAOS/src/runtime/kernel.rs", "w") as f:
    f.write(content)
