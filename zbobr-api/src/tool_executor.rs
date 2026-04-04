use std::path::Path;

use async_trait::async_trait;

/// Format a command with arguments as a copyable command line string.
pub fn format_command_for_log(cmd_name: &str, args: &[&str], task_dir: &Path) -> String {
    format!(
        "cd {:?} && {} {}",
        task_dir,
        cmd_name,
        args.iter()
            .map(|arg| {
                if arg.contains(|c: char| c.is_whitespace() || c == '"' || c == '\'') {
                    format!("\"{}\"", arg.replace('"', "\\\""))
                } else {
                    arg.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    )
}

/// Output produced by an executor run.
///
/// The `output` field is always populated with whatever the process wrote to
/// stdout and stderr (combined), regardless of whether the process succeeded.
/// The outer `anyhow::Result` is only `Err` for I/O-level failures (e.g. the
/// process could not be spawned).  A non-zero exit status is represented by
/// `exit_ok: false` so that callers can always store the captured output.
#[derive(Debug)]
pub struct ExecutorOutput {
    /// Combined stdout (and stderr, if any) from the process.
    pub output: String,
    /// `true` if the process exited with status 0.
    pub exit_ok: bool,
}

/// Trait for executing AI tools with specific configurations.
#[allow(clippy::too_many_arguments)]
#[async_trait]
pub trait ToolExecutor: Send + Sync {
    /// Execute the tool for a given task and role.
    ///
    /// # Arguments
    /// * `task_id` - The task ID
    /// * `role` - The role (Planner or Worker)
    /// * `model` - The AI model to use
    /// * `port` - The MCP server port
    /// * `prompt` - The prompt text for the tool
    /// * `work_dir` - The agent working directory (repo subdirectory)
    /// * `mcp_url` - The MCP server URL
    /// * `agent_github_token` - Read-only GitHub token for agent (passed as GH_TOKEN/GITHUB_TOKEN).
    ///   Security boundary: limits agent's GitHub access to read-only to prevent erroneous
    ///   writes. May be "not-configured" for offline/test runs where GitHub access is not needed.
    /// * `copilot_github_token` - Copilot's GitHub token (passed as COPILOT_GITHUB_TOKEN)
    ///
    /// Returns `Ok(ExecutorOutput)` on both success and process failure so that
    /// captured output is always available for storage.  Returns `Err` only for
    /// I/O-level errors (e.g. the process could not be spawned).
    #[allow(clippy::too_many_arguments)]
    async fn execute(
        &self,
        task_id: u64,
        role: &str,
        model: &str,
        port: u16,
        prompt: &str,
        work_dir: &Path,
        mcp_url: &str,
        plan_mode: bool,
        agent_github_token: &str,
        copilot_github_token: &str,
    ) -> anyhow::Result<ExecutorOutput>;
}
