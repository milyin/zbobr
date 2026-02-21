use std::{path::Path, process::Stdio};

use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, BufReader};

use zbobr_dispatcher::task::{Model, Role};
use zbobr_dispatcher::tool_executor::{ToolExecutor, format_command_for_log};

pub mod config;
pub use config::{
    ZbobrExecutorMcpTesterArgs, ZbobrExecutorMcpTesterConfig, ZbobrExecutorMcpTesterToml,
};

/// Executor that runs mcp-tester to validate MCP servers per stage.
pub struct McpTesterExecutor {
    pub config: ZbobrExecutorMcpTesterConfig,
}

#[async_trait]
impl ToolExecutor for McpTesterExecutor {
    async fn execute(
        &self,
        task_id: u64,
        role: Role,
        _model: &Model,
        _port: u16,
        _prompt: &str,
        task_dir: &Path,
        mcp_url: &str,
        _agent_github_token: &str,
        _copilot_github_token: &str,
    ) -> anyhow::Result<()> {
        let scenario_path = self.config.scenario_for_role(role).ok_or_else(|| {
            anyhow::anyhow!(
                "No scenario file configured for role {:?} in [executor.mcp-tester]",
                role
            )
        })?;

        tracing::info!(
            "Starting mcp-tester for task #{task_id} role {role:?} with scenario {}",
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
            .current_dir(task_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        tracing::debug!(
            "Command: {}",
            format_command_for_log("mcp-tester", &args, task_dir)
        );

        let mut child = cmd.spawn()?;

        let stdout = child.stdout.take().expect("stdout was piped");
        let stderr = child.stderr.take().expect("stderr was piped");

        // Spawn tasks to stream stdout and stderr
        let stdout_task = tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::info!("[mcp-tester] {}", line);
            }
        });

        let stderr_task = tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::warn!("[mcp-tester] {}", line);
            }
        });

        // Wait for process to complete
        let status = child.wait().await?;

        // Wait for output tasks to finish
        let _ = tokio::join!(stdout_task, stderr_task);

        tracing::debug!("mcp-tester finished execution with status: {status}");

        if !status.success() {
            tracing::error!("mcp-tester exited with status: {status}");
            anyhow::bail!("mcp-tester exited with status: {status}");
        }

        Ok(())
    }
}
