//! `nexusaos run` — Submit a task for execution.

use std::sync::Arc;

use tracing::info;

use crate::{
    config::AppConfig,
    error::NexusError,
    model::{openai_compat::OpenAiCompatProvider, registry::ProviderRegistry},
    policy::{PolicyEngine, PolicyRule, TrustTier},
    runtime::kernel::Kernel,
    storage::event_store::EventStore,
    task::TaskInput,
    tools::{broker::ToolBroker, filesystem::FilesystemTool, git::GitTool, terminal::TerminalTool},
};

/// Execute a task through the kernel.
pub fn execute(
    config_path: &str,
    task: &str,
    background: bool,
    yes: bool,
) -> Result<(), NexusError> {
    info!(task = task, background = background, "Submitting task");

    let config = AppConfig::load(config_path)?;
    let data_dir = config.resolved_data_dir();

    let rt = tokio::runtime::Runtime::new().map_err(|e| {
        NexusError::Config(crate::error::ConfigError::Invalid { message: e.to_string() })
    })?;
    rt.block_on(async {
        // 1. Initialize Event Store
        let events_dir = data_dir.join("events");
        let store = Arc::new(EventStore::open(events_dir).await?);

        // 2. Initialize Policy Engine
        let rules = vec![PolicyRule {
            name: "allow-all".to_string(),
            action_pattern: "*".to_string(),
            decision: "allow".to_string(),
            trust_tier: 0,
            description: None,
        }];

        let trust_tier = if yes { TrustTier::Autonomous } else { TrustTier::Basic };
        let policy = PolicyEngine::new(rules, trust_tier);
        let policy_arc = Arc::new(policy.clone());

        // 3. Initialize Model Registry
        let mut registry = ProviderRegistry::new();
        for p_cfg in &config.model_providers {
            if let Ok(provider) = OpenAiCompatProvider::new(p_cfg) {
                registry.register(Box::new(provider));
            }
        }
        let registry = Arc::new(registry);

        // 4. Initialize Tool Broker
        let mut broker = ToolBroker::new(policy_arc);
        let allowed_paths = vec![data_dir.clone()];
        broker.register(Arc::new(FilesystemTool::new(
            allowed_paths,
            config.tools.filesystem.denied_patterns.clone(),
        )));
        if config.tools.git.enabled {
            broker.register(Arc::new(GitTool::new(data_dir.clone())));
        }
        broker.register(Arc::new(TerminalTool::new(
            config.tools.terminal.timeout_secs,
            config.tools.terminal.denied_prefixes.clone(),
        )));
        let broker = Arc::new(broker);

        // 5. Initialize Kernel
        let kernel = Kernel::new(store, policy, registry, broker).await?;

        // 6. Submit Task
        println!("Submitting task: {}", task);
        let task_input = TaskInput::Text(task.to_string());
        let task_id = kernel.submit_task(task_input).await?;
        println!("Task ID: {}", task_id);

        if background {
            println!("Task submitted to background.");
            return Ok(());
        }

        // 7. Execute Task
        println!("Executing task...");
        match kernel.execute_task(&task_id).await {
            Ok(outcome) => {
                println!("\nTask Execution Summary:");
                println!("  Status:  {}", if outcome.success { "Success" } else { "Failed" });
                if let Some(out) = &outcome.output {
                    println!("  Output:  {}", out);
                }
                if let Some(err) = &outcome.error {
                    println!("  Error:   {}", err);
                }
                println!("  Time:    {}", outcome.completed_at);
            }
            Err(e) => {
                println!("\nTask execution failed: {}", e);
            }
        }

        // Note: Interactive confirmation for tools is not currently bubbled up by Kernel::execute_task
        // If it were, we would loop and prompt here using std::io::stdin().

        Ok::<(), NexusError>(())
    })
}
