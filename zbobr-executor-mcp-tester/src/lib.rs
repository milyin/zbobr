use std::{path::Path, process::Stdio};

use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, BufReader};
use zbobr_api::{
    task::Role,
    tool_executor::{ToolExecutor, format_command_for_log},
};

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
        _port: u16,
        _prompt: &str,
        work_dir: &Path,
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

        let stdout_buf = std::sync::Arc::new(tokio::sync::Mutex::new(Vec::<String>::new()));
        let stderr_buf = std::sync::Arc::new(tokio::sync::Mutex::new(Vec::<String>::new()));

        // Spawn tasks to stream stdout and stderr
        let stdout_buf2 = stdout_buf.clone();
        let stdout_task = tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::info!("[mcp-tester] {}", line);
                stdout_buf2.lock().await.push(line);
            }
        });

        let stderr_buf2 = stderr_buf.clone();
        let stderr_task = tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::warn!("[mcp-tester] {}", line);
                stderr_buf2.lock().await.push(line);
            }
        });

        // Wait for process to complete
        let status = child.wait().await?;

        // Wait for output tasks to finish
        let _ = tokio::join!(stdout_task, stderr_task);

        tracing::debug!("mcp-tester finished execution with status: {status}");

        if !status.success() {
            tracing::error!("mcp-tester exited with status: {status}");
            eprintln!("=== mcp-tester stdout ===");
            for line in stdout_buf.lock().await.iter() {
                eprintln!("{line}");
            }
            eprintln!("=== mcp-tester stderr ===");
            for line in stderr_buf.lock().await.iter() {
                eprintln!("{line}");
            }
            anyhow::bail!("mcp-tester exited with status: {status}");
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::path::Path;

    use zbobr_api::Role;

    use super::*;

    #[tokio::test]
    async fn execute_without_scenario_fails() {
        // Build an empty configuration (no scenarios provided).
        let config = ZbobrExecutorMcpTesterConfig::default();
        let executor = McpTesterExecutor { config };

        let result = executor
            .execute(
                42,
                Role::Preparator,
                0,
                "",
                Path::new("."),
                "http://example.com",
                "",
                "",
            )
            .await;

        assert!(result.is_err(), "expected error when scenario missing");
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("No scenario file configured"));
    }
}
