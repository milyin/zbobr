use std::{path::Path, process::Stdio};

use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, BufReader};
use zbobr_api::{
    task::Tool,
    tool_executor::{ToolExecutor, format_command_for_log},
};

pub mod config;
pub use config::{ZbobrExecutorCopilotArgs, ZbobrExecutorCopilotConfig, ZbobrExecutorCopilotToml};

/// Executor for GitHub Copilot CLI.
pub struct CopilotExecutor {
    pub config: ZbobrExecutorCopilotConfig,
}

#[async_trait]
impl ToolExecutor for CopilotExecutor {
    async fn execute(
        &self,
        task_id: u64,
        role: &str,
        _port: u16,
        prompt: &str,
        work_dir: &Path,
        mcp_url: &str,
        agent_github_token: &str,
        copilot_github_token: &str,
    ) -> anyhow::Result<()> {
        // Build MCP config for copilot
        let mcp_config = serde_json::json!({
            "mcpServers": {
                "zbobr": {
                    "type": "http",
                    "url": mcp_url,
                    "tools": ["*"]
                }
            }
        });
        let mcp_config_str = serde_json::to_string(&mcp_config)?;

        let model_name = self
            .config
            .default_model
            .model_name_for_tool(Tool::Copilot)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Model {} is not supported by copilot",
                    self.config.default_model
                )
            })?;

        tracing::info!(
            "Starting copilot {role} session for task #{task_id} with model {model_name}"
        );
        tracing::info!("MCP endpoint: {mcp_url}");
        tracing::debug!("MCP config JSON: {}", mcp_config_str);

        let args = [
            "--model",
            model_name,
            "--additional-mcp-config",
            &mcp_config_str,
            "--no-ask-user",
            "--allow-all-tools",
            "--allow-all-urls",
            "-p",
            prompt,
        ];

        let mut cmd = tokio::process::Command::new("copilot");
        cmd.args(args)
            .current_dir(work_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        tracing::debug!(
            "Command: {}",
            format_command_for_log("copilot", &args, work_dir)
        );

        // Set GitHub tokens for copilot agent process
        tracing::info!("Setting GH_TOKEN for agent and COPILOT_GITHUB_TOKEN for copilot");
        cmd.env("GH_TOKEN", agent_github_token)
            .env("GITHUB_TOKEN", agent_github_token)
            .env("COPILOT_GITHUB_TOKEN", copilot_github_token);

        let mut child = cmd.spawn()?;

        let stdout = child.stdout.take().expect("stdout was piped");
        let stderr = child.stderr.take().expect("stderr was piped");

        // Spawn tasks to stream stdout and stderr
        let stdout_task = tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::info!("[copilot] {}", line);
            }
        });

        let stderr_task = tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            let mut collected = Vec::new();
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::warn!("[copilot] {}", line);
                collected.push(line);
            }
            collected
        });

        // Wait for process to complete
        let status = child.wait().await?;

        // Wait for output tasks to finish
        let (_, stderr_result) = tokio::join!(stdout_task, stderr_task);

        tracing::debug!("Copilot finished execution with status: {status}");

        if !status.success() {
            let error_context = match stderr_result {
                Ok(lines) if !lines.is_empty() => {
                    format!("\nCopilot output:\n{}", lines.join("\n"))
                }
                _ => String::new(),
            };
            anyhow::bail!("copilot exited with status: {status}{error_context}");
        }

        Ok(())
    }
}
