// -- TaskIdentity --

/// Bundles task routing info for worktree operations.
/// Only constructible when all three fields (destination_repository, destination_branch, work_branch) are set.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TaskIdentity {
    pub task_id: u64,
    pub destination_repository: String,
    pub destination_branch: String,
    pub work_branch: String,
}

/// Robustly extract the repository name from a string (which could be a URL, local path, or owner/repo).
pub fn extract_repo_name(repo_ref: &str) -> Option<String> {
    let repo_ref = repo_ref.trim_end_matches(".git");
    let repo_ref = repo_ref.trim_end_matches('/');

    // Handle GitHub URLs or similar: https://github.com/owner/repo
    if repo_ref.contains("://") || repo_ref.starts_with("git@") {
        return repo_ref.split('/').next_back().map(|s| s.to_string());
    }

    // Handle local paths: /path/to/repo or ./repo
    if repo_ref.contains('/') {
        return repo_ref.split('/').next_back().map(|s| s.to_string());
    }

    // Fallback: just return the string if it doesn't contain slashes (already a repo name)
    Some(repo_ref.to_string())
}

// -- Checklist item types --

/// A single item in a task's checklist.
#[derive(
    Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize, schemars::JsonSchema,
)]
pub struct ChecklistItem {
    #[schemars(description = "Unique identifier for the checklist item")]
    pub id: String,
    #[schemars(description = "Checkbox state (true = checked, false = unchecked)")]
    pub checked: bool,
    #[schemars(description = "Checklist item text")]
    pub text: String,
}

// -- Comment --

/// A structured comment with metadata.
///
/// The `stage` field identifies which stage posted the comment (e.g. "planning",
/// "working"). The body text may contain `[tool_name]` section headers added by
/// MCP tools (e.g. `[report_success]`, `[report_failure]`).
#[derive(
    Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize, schemars::JsonSchema,
)]
pub struct Comment {
    #[schemars(description = "Timestamp when comment was created")]
    pub timestamp: String,
    #[schemars(description = "Stage that posted this comment")]
    pub stage: String,
    #[schemars(description = "Hostname of the system posting the comment")]
    pub hostname: String,
    #[schemars(description = "Tool that executed this comment (e.g. copilot, claude)")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool: Option<Tool>,
    #[schemars(description = "Model used by the tool")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<Model>,
    #[schemars(description = "Comment text (may contain [tool] section headers)")]
    pub text: String,
    #[schemars(description = "Pipeline name that produced this comment")]
    #[serde(default)]
    pub pipeline: String,
    #[schemars(description = "Monotonic run counter within the pipeline")]
    #[serde(default)]
    pub pipeline_run_id: u64,
    #[schemars(description = "Optional caller pipeline for linked final pipeline reports")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caller_pipeline: Option<String>,
    #[schemars(description = "Optional caller pipeline run id for linked final pipeline reports")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caller_pipeline_run_id: Option<u64>,
}

// -- History helper types --

/// Type of a history record, derived from `[tool_name]` prefix in comment text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HistoryRecordType {
    Task,
    Success,
    Failure,
    Question,
    Error,
    Other,
}

/// Determine the record type from a comment's `[tool_name]` prefix.
pub fn classify_comment(text: &str) -> HistoryRecordType {
    let prefix = text.split('\n').next().unwrap_or("");
    match prefix {
        "[report_results]" | "[report_success]" | "[post_plan]"
        | "[review_accept]" | "[test_accept]" => HistoryRecordType::Success,
        "[report_failure]" | "[ask_planner]"
        | "[review_reject]" | "[test_reject]" => HistoryRecordType::Failure,
        "[ask_user]" | "[stop_with_question]" => HistoryRecordType::Question,
        "[report_error]" | "[stop_with_error]" => HistoryRecordType::Error,
        _ => HistoryRecordType::Other,
    }
}

/// Extract a one-line summary from comment text (first non-prefix line, truncated).
pub fn extract_summary(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    // Skip the [tool_name] prefix line if present
    let content_line = if lines.first().map_or(false, |l| l.starts_with('[') && l.ends_with(']')) {
        lines.get(1).copied().unwrap_or("")
    } else {
        lines.first().copied().unwrap_or("")
    };
    let trimmed = content_line.trim();
    if trimmed.len() > 120 {
        format!("{}...", &trimmed[..120])
    } else {
        trimmed.to_string()
    }
}

/// An entry on the task's call stack, recording which pipeline to return to
/// and which signal to emit upon return (e.g. "go_working").
#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct StackEntry {
    pub pipeline: String,
    /// Signal to emit when returning to this pipeline (e.g. "go_working").
    #[serde(alias = "stage")]
    pub signal: String,
    /// Caller's pipeline_run_id to restore on return.
    #[serde(default)]
    pub pipeline_run_id: u64,
}

/// A worktree problem detected before stage execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorktreeProblem {
    Undefined,
    Conflict,
}

/// Role for task execution — now a plain string to support configurable roles.
pub type Role = String;


/// AI Tool/Agent to use.
#[derive(
    Debug,
    Clone,
    Copy,
    serde::Serialize,
    serde::Deserialize,
    PartialEq,
    Eq,
    schemars::JsonSchema,
    Default,
)]
pub enum Tool {
    #[serde(rename = "copilot")]
    #[default]
    Copilot,
    #[serde(rename = "claude")]
    Claude,
    #[serde(rename = "mcp-tester")]
    McpTester,
}

impl Tool {
    /// Returns all available tools.
    pub fn all() -> &'static [Tool] {
        &[Tool::Copilot, Tool::Claude, Tool::McpTester]
    }
}

impl std::fmt::Display for Tool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Tool::Copilot => write!(f, "copilot"),
            Tool::Claude => write!(f, "claude"),
            Tool::McpTester => write!(f, "mcp-tester"),
        }
    }
}

impl std::str::FromStr for Tool {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "copilot" => Ok(Tool::Copilot),
            "claude" => Ok(Tool::Claude),
            "mcp-tester" => Ok(Tool::McpTester),
            _ => Err(anyhow::anyhow!("Unknown tool: {}", s)),
        }
    }
}

/// AI Model to use.
#[derive(
    Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq, schemars::JsonSchema, Default,
)]
pub enum Model {
    /// Special sentinel indicating the default model for the current tool.
    /// The concrete mapping is handled outside of the enum (e.g. via
    /// executor configuration).  It serializes to the string "default".
    #[serde(rename = "default")]
    Default,

    // Retired models (kept for backward compatibility, no longer available)
    #[serde(rename = "gpt-4o")]
    Gpt4o,
    #[serde(rename = "claude-3-5-sonnet")]
    Claude35Sonnet,
    #[serde(rename = "claude-3-opus")]
    Claude3Opus,
    // Active models
    #[serde(rename = "gpt-5-mini")]
    #[default]
    Gpt5Mini,
    #[serde(rename = "gpt-5")]
    Gpt5,
    #[serde(rename = "gpt-5.1")]
    Gpt5_1,
    #[serde(rename = "gpt-5.1-codex-mini")]
    Gpt5_1CodexMini,
    #[serde(rename = "gpt-5.1-codex")]
    Gpt5_1Codex,
    #[serde(rename = "gpt-5.1-codex-max")]
    Gpt5_1CodexMax,
    #[serde(rename = "gpt-5.2")]
    Gpt5_2,
    #[serde(rename = "gpt-5.2-codex")]
    Gpt5_2Codex,
    #[serde(rename = "gpt-4.1")]
    Gpt4_1,
    #[serde(rename = "claude-sonnet-4")]
    ClaudeSonnet4,
    #[serde(rename = "claude-haiku-4.5")]
    ClaudeHaiku4_5,
    #[serde(rename = "claude-opus-4.5")]
    ClaudeOpus4_5,
    #[serde(rename = "claude-sonnet-4.5")]
    ClaudeSonnet4_5,
    #[serde(rename = "claude-opus-4.6")]
    ClaudeOpus4_6,
    #[serde(rename = "claude-opus-4.6-fast")]
    ClaudeOpus4_6Fast,
    #[serde(rename = "gemini-3-pro-preview")]
    Gemini3ProPreview,
}

impl Model {
    /// Returns all available models.
    pub fn all() -> &'static [Model] {
        &[
            Model::Default,
            Model::Gpt5Mini,
            Model::Gpt5,
            Model::Gpt5_1,
            Model::Gpt5_1CodexMini,
            Model::Gpt5_1Codex,
            Model::Gpt5_1CodexMax,
            Model::Gpt5_2,
            Model::Gpt5_2Codex,
            Model::Gpt4_1,
            Model::ClaudeSonnet4,
            Model::ClaudeHaiku4_5,
            Model::ClaudeOpus4_5,
            Model::ClaudeSonnet4_5,
            Model::ClaudeOpus4_6,
            Model::ClaudeOpus4_6Fast,
            Model::Gemini3ProPreview,
        ]
    }

    pub fn model_name_for_tool(&self, tool: Tool) -> Option<&'static str> {
        match tool {
            Tool::Copilot => match self {
                // cheapest Copilot offering is gpt-5-mini
                Model::Default => Some("gpt-5-mini"),
                Model::Gpt4o => None,          // retired
                Model::Claude35Sonnet => None, // retired
                Model::Claude3Opus => None,    // retired
                Model::Gpt5Mini => Some("gpt-5-mini"),
                Model::Gpt5 => Some("gpt-5"),
                Model::Gpt5_1 => Some("gpt-5.1"),
                Model::Gpt5_1CodexMini => Some("gpt-5.1-codex-mini"),
                Model::Gpt5_1Codex => Some("gpt-5.1-codex"),
                Model::Gpt5_1CodexMax => Some("gpt-5.1-codex-max"),
                Model::Gpt5_2 => Some("gpt-5.2"),
                Model::Gpt5_2Codex => Some("gpt-5.2-codex"),
                Model::Gpt4_1 => Some("gpt-4.1"),
                Model::ClaudeSonnet4 => Some("claude-sonnet-4"),
                Model::ClaudeHaiku4_5 => Some("claude-haiku-4.5"),
                Model::ClaudeOpus4_5 => Some("claude-opus-4.5"),
                Model::ClaudeSonnet4_5 => Some("claude-sonnet-4.5"),
                Model::ClaudeOpus4_6 => Some("claude-opus-4.6"),
                Model::ClaudeOpus4_6Fast => Some("claude-opus-4.6-fast"),
                Model::Gemini3ProPreview => Some("gemini-3-pro-preview"),
            },
            Tool::Claude => match self {
                // cheapest Claude offering is the 3.5 sonnet
                Model::Default => Some("claude-3-5-sonnet"),
                Model::Claude35Sonnet => Some("claude-3-5-sonnet"),
                Model::Claude3Opus => Some("claude-3-opus"),
                Model::ClaudeSonnet4_5 => Some("claude-sonnet-4-5"),
                Model::ClaudeHaiku4_5 => Some("claude-haiku-4-5"),
                Model::ClaudeOpus4_5 => Some("claude-opus-4-5"),
                Model::ClaudeOpus4_6 => Some("claude-opus-4-6"),
                Model::ClaudeOpus4_6Fast => Some("claude-opus-4-6"),
                Model::ClaudeSonnet4 => Some("claude-sonnet-4"),
                _ => None,
            },
            Tool::McpTester => None,
        }
    }
}

impl std::fmt::Display for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Model::Default => "default",
            Model::Gpt4o => "gpt-4o",
            Model::Claude35Sonnet => "claude-3-5-sonnet",
            Model::Claude3Opus => "claude-3-opus",
            Model::Gpt5Mini => "gpt-5-mini",
            Model::Gpt5 => "gpt-5",
            Model::Gpt5_1 => "gpt-5.1",
            Model::Gpt5_1CodexMini => "gpt-5.1-codex-mini",
            Model::Gpt5_1Codex => "gpt-5.1-codex",
            Model::Gpt5_1CodexMax => "gpt-5.1-codex-max",
            Model::Gpt5_2 => "gpt-5.2",
            Model::Gpt5_2Codex => "gpt-5.2-codex",
            Model::Gpt4_1 => "gpt-4.1",
            Model::ClaudeSonnet4 => "claude-sonnet-4",
            Model::ClaudeHaiku4_5 => "claude-haiku-4.5",
            Model::ClaudeOpus4_5 => "claude-opus-4.5",
            Model::ClaudeSonnet4_5 => "claude-sonnet-4.5",
            Model::ClaudeOpus4_6 => "claude-opus-4.6",
            Model::ClaudeOpus4_6Fast => "claude-opus-4.6-fast",
            Model::Gemini3ProPreview => "gemini-3-pro-preview",
        };
        f.write_str(s)
    }
}

impl std::str::FromStr for Model {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('.', "-").as_str() {
            "default" => Ok(Model::Default),
            "gpt-4o" | "gpt4o" => Ok(Model::Gpt4o),
            "claude-3-5-sonnet" | "claude35sonnet" => Ok(Model::Claude35Sonnet),
            "claude-3-opus" | "claude3opus" => Ok(Model::Claude3Opus),
            "gpt-5-mini" | "gpt5mini" => Ok(Model::Gpt5Mini),
            "gpt-5" => Ok(Model::Gpt5),
            "gpt-5-1" => Ok(Model::Gpt5_1),
            "gpt-5-1-codex-mini" => Ok(Model::Gpt5_1CodexMini),
            "gpt-5-1-codex" => Ok(Model::Gpt5_1Codex),
            "gpt-5-1-codex-max" => Ok(Model::Gpt5_1CodexMax),
            "gpt-5-2" => Ok(Model::Gpt5_2),
            "gpt-5-2-codex" => Ok(Model::Gpt5_2Codex),
            "gpt-4-1" => Ok(Model::Gpt4_1),
            "claude-sonnet-4" => Ok(Model::ClaudeSonnet4),
            "claude-haiku-4-5" => Ok(Model::ClaudeHaiku4_5),
            "claude-opus-4-5" => Ok(Model::ClaudeOpus4_5),
            "claude-sonnet-4-5" => Ok(Model::ClaudeSonnet4_5),
            "claude-opus-4-6" => Ok(Model::ClaudeOpus4_6),
            "claude-opus-4-6-fast" => Ok(Model::ClaudeOpus4_6Fast),
            "gemini-3-pro-preview" => Ok(Model::Gemini3ProPreview),
            _ => Err(anyhow::anyhow!("Unknown model: {}", s)),
        }
    }
}

/// Tag for comment formatting. Contains pipeline info, stage name, hostname, and optional tool/model.
///
/// Format: `// {pipeline}:{run_id}:{stage} by {hostname}[:{tool}[:{model}]]`
/// Example: `// main:3:working by myhost:copilot:gpt-5-mini`
/// Example (no tool): `// main:3:working by myhost`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentTag {
    pub pipeline: String,
    pub pipeline_run_id: u64,
    pub stage: String,
    pub hostname: String,
    pub tool: Option<Tool>,
    pub model: Option<Model>,
    pub caller_pipeline: Option<String>,
    pub caller_pipeline_run_id: Option<u64>,
}

impl CommentTag {
    pub fn new(
        pipeline: String,
        pipeline_run_id: u64,
        stage: String,
        hostname: String,
        tool: Option<Tool>,
        model: Option<Model>,
    ) -> Self {
        Self {
            pipeline,
            pipeline_run_id,
            stage,
            hostname,
            tool,
            model,
            caller_pipeline: None,
            caller_pipeline_run_id: None,
        }
    }

    pub fn with_caller(mut self, caller_pipeline: String, caller_pipeline_run_id: u64) -> Self {
        self.caller_pipeline = Some(caller_pipeline);
        self.caller_pipeline_run_id = Some(caller_pipeline_run_id);
        self
    }
}

impl std::fmt::Display for CommentTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const FOR_SEPARATOR: &str = " for ";

        write!(f, "// {}:{}:{} by {}", self.pipeline, self.pipeline_run_id, self.stage, self.hostname)?;
        if let Some(ref tool) = self.tool {
            write!(f, ":{tool}")?;
            if let Some(ref model) = self.model {
                write!(f, ":{model}")?;
            }
        }
        if let (Some(caller_pipeline), Some(caller_run_id)) =
            (&self.caller_pipeline, self.caller_pipeline_run_id)
        {
            write!(f, "{FOR_SEPARATOR}{caller_pipeline}:{caller_run_id}")?;
        }
        Ok(())
    }
}

impl std::str::FromStr for CommentTag {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        const BY_SEPARATOR: &str = " by ";
        const FOR_SEPARATOR: &str = " for ";

        let s = s.trim_start_matches("//").trim_start();

        let (prefix, suffix) = s
            .split_once(BY_SEPARATOR)
            .ok_or_else(|| anyhow::anyhow!("Invalid tag format: {}", s))?;

        let prefix_parts: Vec<&str> = prefix.split(':').collect();
        if prefix_parts.len() < 3 {
            return Err(anyhow::anyhow!("Invalid tag prefix: {}", prefix));
        }
        let pipeline = prefix_parts[0].to_string();
        let pipeline_run_id = prefix_parts[1].parse::<u64>()
            .map_err(|_| anyhow::anyhow!("Invalid run id in tag prefix: {}", prefix_parts[1]))?;
        let stage = prefix_parts[2].to_string();

        let (host_part, caller_part) = if let Some((lhs, rhs)) = suffix.split_once(FOR_SEPARATOR) {
            (lhs, Some(rhs))
        } else {
            (suffix, None)
        };

        let suffix_parts: Vec<&str> = host_part.split(':').collect();
        let hostname = suffix_parts
            .first()
            .copied()
            .unwrap_or_default()
            .to_string();
        if hostname.is_empty() {
            return Err(anyhow::anyhow!("Invalid hostname in tag: {}", s));
        }

        let mut tool: Option<Tool> = None;
        let mut model: Option<Model> = None;
        for part in &suffix_parts[1..] {
            if tool.is_none() {
                if let Ok(t) = part.parse::<Tool>() {
                    tool = Some(t);
                }
            } else if model.is_none() {
                if let Ok(m) = part.parse::<Model>() {
                    model = Some(m);
                }
            }
        }

        let (caller_pipeline, caller_pipeline_run_id) = if let Some(caller) = caller_part {
            let caller_parts: Vec<&str> = caller.split(':').collect();
            if caller_parts.len() != 2 {
                return Err(anyhow::anyhow!("Invalid caller suffix: {}", caller));
            }
            let caller_pipeline = caller_parts[0].to_string();
            let caller_pipeline_run_id = caller_parts[1]
                .parse::<u64>()
                .map_err(|_| anyhow::anyhow!("Invalid caller run id: {}", caller_parts[1]))?;
            (Some(caller_pipeline), Some(caller_pipeline_run_id))
        } else {
            (None, None)
        };

        Ok(CommentTag {
            pipeline,
            pipeline_run_id,
            stage,
            hostname,
            tool,
            model,
            caller_pipeline,
            caller_pipeline_run_id,
        })
    }
}

/// A task in the abstract domain (generic, backed by GitHub or Filesystem).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Task {
    pub id: u64,
    pub title: String,
    pub description: String,
    /// Task state: empty | "DONE" | "PAUSE" | "READY" | "{PIPELINE}_PENDING" | "{PIPELINE}_{STAGE}"
    pub state: String,
    pub destination_repository: Option<String>,
    pub destination_branch: Option<String>,
    pub work_branch: Option<String>,
    pub pr_url: Option<String>,
    pub checklist: Vec<ChecklistItem>,
    /// Signal for flow control: go_{stage}, call_{pipeline}, return
    pub signal: Option<String>,
    /// Call stack for pipeline call/return semantics.
    #[serde(default)]
    pub stack: Vec<StackEntry>,
    pub pause: bool,
    /// When true the dispatcher will automatically set the pause flag any time
    /// the task's state is changed.  This gives human operators an opportunity to
    /// review a transition before the next processing step occurs.
    pub confirm: bool,
    /// Current/latest pipeline run counter. Incremented on each new pipeline call.
    #[serde(default)]
    pub pipeline_run_id: u64,
    /// ETag for optimistic locking to prevent concurrent update conflicts.
    /// Used to detect if the task has been modified between read and write operations.
    #[serde(skip)]
    pub etag: Option<String>,
}

/// Filter comments for a specific pipeline run.
///
/// Comments with `pipeline_run_id == 0` (user comments) inherit the run ID
/// of the previous comment at retrieval time.
pub fn filter_comments_for_run(comments: &[Comment], target_run_id: u64) -> Vec<&Comment> {
    let mut result = Vec::new();
    let mut current_run_id: u64 = 0;
    for comment in comments {
        let effective = if comment.pipeline_run_id > 0 {
            current_run_id = comment.pipeline_run_id;
            comment.pipeline_run_id
        } else {
            current_run_id // user comment inherits previous
        };
        let caller_match = comment.caller_pipeline_run_id == Some(target_run_id);
        if effective == target_run_id || caller_match {
            result.push(comment);
        }
    }
    result
}

impl Task {
    /// Returns a TaskIdentity if all three routing fields are set.
    pub fn identity(&self) -> Option<TaskIdentity> {
        Some(TaskIdentity {
            task_id: self.id,
            destination_repository: self.destination_repository.clone()?,
            destination_branch: self.destination_branch.clone()?,
            work_branch: self.work_branch.clone()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_comment(text: &str, pipeline: &str, run_id: u64) -> Comment {
        Comment {
            timestamp: String::new(),
            stage: "s".into(),
            hostname: "h".into(),
            tool: None,
            model: None,
            text: text.into(),
            pipeline: pipeline.into(),
            pipeline_run_id: run_id,
            caller_pipeline: None,
            caller_pipeline_run_id: None,
        }
    }

    fn make_comment_for(text: &str, pipeline: &str, run_id: u64, caller_run_id: u64) -> Comment {
        Comment {
            timestamp: String::new(),
            stage: "s".into(),
            hostname: "h".into(),
            tool: None,
            model: None,
            text: text.into(),
            pipeline: pipeline.into(),
            pipeline_run_id: run_id,
            caller_pipeline: Some("main".into()),
            caller_pipeline_run_id: Some(caller_run_id),
        }
    }

    #[test]
    fn filter_separates_pipeline_runs() {
        let comments = vec![
            make_comment("main work", "main", 1),
            make_comment("sub work", "sub", 2),
            make_comment("more sub", "sub", 2),
            make_comment("back to main", "main", 1),
        ];
        let run1: Vec<_> = filter_comments_for_run(&comments, 1)
            .iter()
            .map(|c| c.text.as_str())
            .collect();
        let run2: Vec<_> = filter_comments_for_run(&comments, 2)
            .iter()
            .map(|c| c.text.as_str())
            .collect();
        assert_eq!(run1, vec!["main work", "back to main"]);
        assert_eq!(run2, vec!["sub work", "more sub"]);
    }

    #[test]
    fn filter_user_comments_inherit_run_id() {
        let comments = vec![
            make_comment("agent start", "main", 1),
            make_comment("user reply", "", 0),   // user comment
            make_comment("agent in sub", "sub", 2),
            make_comment("user in sub", "", 0),   // user comment
            make_comment("agent back", "main", 1),
        ];
        let run1: Vec<_> = filter_comments_for_run(&comments, 1)
            .iter()
            .map(|c| c.text.as_str())
            .collect();
        let run2: Vec<_> = filter_comments_for_run(&comments, 2)
            .iter()
            .map(|c| c.text.as_str())
            .collect();
        assert_eq!(run1, vec!["agent start", "user reply", "agent back"]);
        assert_eq!(run2, vec!["agent in sub", "user in sub"]);
    }

    #[test]
    fn filter_empty_comments() {
        let comments: Vec<Comment> = vec![];
        assert!(filter_comments_for_run(&comments, 1).is_empty());
    }

    #[test]
    fn filter_no_matching_run() {
        let comments = vec![
            make_comment("a", "main", 1),
            make_comment("b", "main", 1),
        ];
        assert!(filter_comments_for_run(&comments, 99).is_empty());
    }

    #[test]
    fn filter_matches_caller_linked_comments() {
        let comments = vec![
            make_comment("main work", "main", 1),
            make_comment_for("sub final report", "sub", 2, 1),
            make_comment("next main", "main", 1),
        ];
        let run1: Vec<_> = filter_comments_for_run(&comments, 1)
            .iter()
            .map(|c| c.text.as_str())
            .collect();
        assert_eq!(run1, vec!["main work", "sub final report", "next main"]);
    }
}
