use std::{path::Path, process::Stdio};

use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, BufReader};
use zbobr_api::{
    task::Tool,
    tool_executor::{ExecutorOutput, ToolExecutor, format_command_for_log},
};

pub mod config;
pub use config::{ZbobrExecutorClaudeArgs, ZbobrExecutorClaudeConfig, ZbobrExecutorClaudeToml};

/// Executor for Claude CLI.
pub struct ClaudeExecutor {
    pub config: ZbobrExecutorClaudeConfig,
}

impl ClaudeExecutor {
    pub fn new(config: ZbobrExecutorClaudeConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl ToolExecutor for ClaudeExecutor {
    async fn execute(
        &self,
        task_id: u64,
        role: &str,
        _port: u16,
        prompt: &str,
        work_dir: &Path,
        mcp_url: &str,
        plan_mode: bool,
        agent_github_token: &str,
        copilot_github_token: &str,
    ) -> anyhow::Result<ExecutorOutput> {
        // Build MCP config for claude
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
            .model_name_for_tool(Tool::Claude)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Model {} is not supported by claude",
                    self.config.default_model
                )
            })?;

        tracing::info!(
            "Starting claude {role} session for task #{task_id} with model {model_name}"
        );
        tracing::info!("MCP endpoint: {mcp_url}");
        tracing::debug!("MCP config JSON: {}", mcp_config_str);

        let permission_mode = if plan_mode { "plan" } else { "acceptEdits" };

        let args = [
            "--model",
            model_name,
            "--mcp-config",
            &mcp_config_str,
            "--permission-mode",
            permission_mode,
            "--allowedTools",
            "mcp__zbobr__*,Bash,WebFetch,WebSearch",
            "--tools",
            "default",
            "--verbose",
            "-p",
            prompt,
        ];

        let mut cmd = tokio::process::Command::new("claude");
        cmd.args(args)
            .current_dir(work_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        tracing::debug!(
            "Command: {}",
            format_command_for_log("claude", &args, work_dir)
        );

        // Set GitHub tokens for claude agent process
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
            let mut collected = Vec::new();
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::info!("[claude] {}", line);
                collected.push(line);
            }
            collected
        });

        let stderr_task = tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            let mut collected = Vec::new();
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::warn!("[claude] {}", line);
                collected.push(line);
            }
            collected
        });

        // Wait for process to complete
        let status = child.wait().await?;

        // Wait for output tasks to finish
        let (stdout_result, stderr_result) = tokio::join!(stdout_task, stderr_task);

        tracing::debug!("Claude finished execution with status: {status}");

        let stdout_lines = stdout_result.unwrap_or_default();
        let stderr_lines = stderr_result.unwrap_or_default();
        let output = combine_output(stdout_lines, stderr_lines);

        Ok(ExecutorOutput {
            output,
            exit_ok: status.success(),
        })
    }
}

/// Combine stdout and stderr lines into a single string.
/// Stderr lines are appended after stdout with a separator if non-empty.
fn combine_output(stdout: Vec<String>, stderr: Vec<String>) -> String {
    if stderr.is_empty() {
        return stdout.join("\n");
    }
    if stdout.is_empty() {
        return stderr.join("\n");
    }
    format!(
        "{}\n--- stderr ---\n{}",
        stdout.join("\n"),
        stderr.join("\n")
    )
}
