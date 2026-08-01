import re

with open("/home/gagan/Workspace/NexusAOS/src/runtime/kernel.rs", "r") as f:
    content = f.read()

content = content.replace('crate::error::ProviderError::NotFound { name: "Planner".into() }',
                          'crate::error::ProviderError::Unavailable { name: "Planner".into() }')
content = content.replace('crate::error::ProviderError::NotFound { name: "Coder".into() }',
                          'crate::error::ProviderError::Unavailable { name: "Coder".into() }')
content = content.replace('crate::error::ProviderError::NotFound', 'crate::error::ProviderError::Unavailable')

with open("/home/gagan/Workspace/NexusAOS/src/runtime/kernel.rs", "w") as f:
    f.write(content)
