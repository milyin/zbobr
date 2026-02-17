use std::{path::Path, process::Stdio};

use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, BufReader};

use zbobr_dispatcher::tool_executor::{ToolExecutor, format_command_for_log};
use zbobr_dispatcher::task::{Model, Role, Tool};

/// Executor for GitHub Copilot CLI.
pub struct CopilotExecutor;

#[async_trait]
impl ToolExecutor for CopilotExecutor {
    async fn execute(
        &self,
        task_id: u64,
        role: Role,
        model: &Model,
        _port: u16,
        prompt: &str,
        task_dir: &Path,
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

        let model_name = model
            .model_name_for_tool(Tool::Copilot)
            .ok_or_else(|| anyhow::anyhow!("Model {} is not supported by copilot", model))?;

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
            .current_dir(task_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        tracing::debug!(
            "Command: {}",
            format_command_for_log("copilot", &args, task_dir)
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
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::warn!("[copilot] {}", line);
            }
        });

        // Wait for process to complete
        let status = child.wait().await?;

        // Wait for output tasks to finish
        let _ = tokio::join!(stdout_task, stderr_task);

        tracing::debug!("Copilot finished execution with status: {status}");

        if !status.success() {
            tracing::error!("copilot exited with status: {status}");
            return Err(anyhow::anyhow!("copilot exited with status: {status}"));
        }

        Ok(())
    }
}
