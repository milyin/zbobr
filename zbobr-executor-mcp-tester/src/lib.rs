use std::{path::Path, process::Stdio};

use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, BufReader};
use zbobr_api::tool_executor::{ExecutorOutput, ToolExecutor, format_command_for_log};

pub mod config;
pub use config::{
    ZbobrExecutorMcpTesterArgs, ZbobrExecutorMcpTesterConfig, ZbobrExecutorMcpTesterToml,
};

/// Executor that runs mcp-tester to validate MCP servers per stage.
pub struct McpTesterExecutor {
    pub config: ZbobrExecutorMcpTesterConfig,
}

impl McpTesterExecutor {
    pub fn new(config: ZbobrExecutorMcpTesterConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl ToolExecutor for McpTesterExecutor {
    async fn execute(
        &self,
        task_id: u64,
        role: &str,
        _port: u16,
        _prompt: &str,
        work_dir: &Path,
        mcp_url: &str,
        _plan_mode: bool,
        _agent_github_token: &str,
        _copilot_github_token: &str,
    ) -> anyhow::Result<ExecutorOutput> {
        let scenario_path = self.config.scenario_for_stage(role).ok_or_else(|| {
            anyhow::anyhow!(
                "No scenario file configured for stage '{}' in [executor.mcp-tester]",
                role
            )
        })?;

        tracing::info!(
            "Starting mcp-tester for task #{task_id} stage '{role}' with scenario {}",
            scenario_path.display()
        );
        tracing::info!("MCP endpoint: {mcp_url}");

        let scenario_str = scenario_path.to_str().ok_or_else(|| {
            anyhow::anyhow!(
                "Scenario path is not valid UTF-8: {}",
                scenario_path.display()
            )
        })?;

        let args = ["scenario", mcp_url, scenario_str, "--detailed"];

        let mut cmd = tokio::process::Command::new("mcp-tester");
        cmd.args(args)
            .current_dir(work_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        tracing::debug!(
            "Command: {}",
            format_command_for_log("mcp-tester", &args, work_dir)
        );

        let mut child = cmd.spawn()?;

        let stdout = child.stdout.take().expect("stdout was piped");
        let stderr = child.stderr.take().expect("stderr was piped");

        // Spawn tasks to stream stdout and stderr
        let stdout_task = tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            let mut collected = Vec::new();
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::info!("[mcp-tester] {}", line);
                collected.push(line);
            }
            collected
        });

        let stderr_task = tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            let mut collected = Vec::new();
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::warn!("[mcp-tester] {}", line);
                collected.push(line);
            }
            collected
        });

        // Wait for process to complete
        let status = child.wait().await?;

        // Wait for output tasks to finish
        let (stdout_result, stderr_result) = tokio::join!(stdout_task, stderr_task);

        tracing::debug!("mcp-tester finished execution with status: {status}");

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

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[tokio::test]
    async fn execute_without_scenario_fails() {
        // Build an empty configuration (no scenarios provided).
        let config = ZbobrExecutorMcpTesterConfig::default();
        let executor = McpTesterExecutor { config };

        let result = executor
            .execute(
                42,
                "preparation",
                0,
                "",
                Path::new("."),
                "http://example.com",
                false,
                "",
                "",
            )
            .await;

        assert!(result.is_err(), "expected error when scenario missing");
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("No scenario file configured"));
    }
}
