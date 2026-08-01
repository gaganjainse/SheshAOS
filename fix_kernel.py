import re

with open("/home/gagan/Workspace/NexusAOS/src/runtime/kernel.rs", "r") as f:
    content = f.read()

# Add imports if not present
if "use crate::model::types::{" not in content:
    content = content.replace("use crate::task::{TaskId, TaskInput, TaskRequest};",
                              "use crate::task::{TaskId, TaskInput, TaskRequest};\nuse crate::model::types::{CompletionRequest, ChatMessage, ChatRole};")

# Fix ProviderError::HealthCheckFailed -> NotFound
content = content.replace('crate::error::ProviderError::HealthCheckFailed("Planner not found".into())',
                          'crate::error::ProviderError::NotFound { name: "Planner".into() }')
content = content.replace('crate::error::ProviderError::HealthCheckFailed("Coder not found".into())',
                          'crate::error::ProviderError::NotFound { name: "Coder".into() }')

# Remove duplicate CompletionRequest import in tests if it is there
content = content.replace("use crate::model::types::CompletionRequest;\n    \n    struct MockProvider", "struct MockProvider")

with open("/home/gagan/Workspace/NexusAOS/src/runtime/kernel.rs", "w") as f:
    f.write(content)
