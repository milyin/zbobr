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
    /// `true` if the output indicates a quota or account-limit failure (rate
    /// limiting, billing limits, etc.).  Such failures are treated as provider
    /// unavailability and trigger temporary provider exclusion, just like
    /// connectivity failures.
    pub quota_failure: bool,
}

/// Scan combined executor output for known quota / rate-limit / account-limit
/// error signatures.  Returns `true` when any known pattern is found.
///
/// The patterns are deliberately specific to avoid false positives on task
/// output that happens to mention these words in a different context.
pub fn detect_quota_failure(output: &str) -> bool {
    let lower = output.to_lowercase();
    lower.contains("rate limit")
        || lower.contains("too many requests")
        || lower.contains("quota exceeded")
        || lower.contains("usage limit")
        || lower.contains("account limit")
        || lower.contains("rate_limit_error")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_quota_failure_rate_limit() {
        assert!(detect_quota_failure("Error: rate limit exceeded, try again later"));
    }

    #[test]
    fn detect_quota_failure_too_many_requests() {
        assert!(detect_quota_failure("HTTP 429: too many requests"));
    }

    #[test]
    fn detect_quota_failure_quota_exceeded() {
        assert!(detect_quota_failure("API quota exceeded for this billing period"));
    }

    #[test]
    fn detect_quota_failure_usage_limit() {
        // Verifies case-insensitivity: "Usage Limit" with mixed case
        assert!(detect_quota_failure("Your Usage Limit has been reached"));
    }

    #[test]
    fn detect_quota_failure_account_limit() {
        assert!(detect_quota_failure("account limit reached, upgrade your plan"));
    }

    #[test]
    fn detect_quota_failure_rate_limit_error() {
        assert!(detect_quota_failure("Received rate_limit_error from API"));
    }

    #[test]
    fn detect_quota_failure_no_match() {
        assert!(!detect_quota_failure("Command failed with exit code 1"));
    }
}
