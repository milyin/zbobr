use std::collections::HashMap;

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

// -- Parameter names enum --

/// Standardized parameter names for task configuration.
/// Note: DestinationRepository, DestinationBranch, and WorkBranch have been
/// promoted to first-class fields on Task. Only extensible params remain here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Parameter {
    PrUrl,
}

impl Parameter {
    /// Returns the parameter name as a string.
    pub fn name(&self) -> &'static str {
        match self {
            Parameter::PrUrl => "pr_url",
        }
    }
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

// -- Comment types --

/// Comment type classification.
///
/// Variants:
/// - `Error`    — posted by `report_error` MCP tool when an agent encounters an unrecoverable
///   problem; also posted by the dispatcher/CLI on execution failure.
/// - `Report`   — posted by `report_results` MCP tool to deliver a role's completion output.
/// - `Plan`     — posted by `post_plan` MCP tool (planner role) to record the implementation plan.
/// - `Request`  — posted for user-originated messages and for questions raised by `ask_user` (and
///   similar ASK_xxx MCP tools) that pause the task waiting for a human response.
/// - `Reject`   — posted by reviewer/tester when rejecting work; acts as a context chunk boundary
///   and contains the rejection message. Visible in GET_HISTORY as the first comment of a new chunk.
/// - `Done`     — posted by the dispatcher when a task is accepted and marked complete; acts as a
///   context chunk boundary but is excluded from GET_HISTORY results.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize, schemars::JsonSchema,
)]
pub enum CommentType {
    /// Unrecoverable error from an agent or the dispatcher.
    #[serde(rename = "error")]
    Error,
    /// Completion report from an agent role.
    #[serde(rename = "report")]
    Report,
    /// Implementation plan posted by the planner role.
    #[serde(rename = "plan")]
    Plan,
    /// User message or agent request awaiting a human response (ASK_xxx operations).
    #[serde(rename = "request")]
    Request,
    /// Rejection posted by a reviewer or tester; also serves as a context chunk boundary.
    /// Contains the rejection message and is included in GET_HISTORY as the first comment of a chunk.
    #[serde(rename = "reject")]
    Reject,
    /// Completion marker posted by the dispatcher after a task is accepted and marked done.
    /// Serves as a context chunk boundary; excluded from GET_HISTORY results.
    #[serde(rename = "done")]
    Done,
}

impl CommentType {
    /// Returns the comment type as a string.
    pub fn as_str(&self) -> &'static str {
        match self {
            CommentType::Error => "error",
            CommentType::Report => "report",
            CommentType::Plan => "plan",
            CommentType::Request => "request",
            CommentType::Reject => "reject",
            CommentType::Done => "done",
        }
    }

    /// Parse from string representation, returning `None` on unknown input.
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.to_ascii_lowercase();
        match s.as_str() {
            "error" => Some(CommentType::Error),
            "report" => Some(CommentType::Report),
            "plan" => Some(CommentType::Plan),
            "request" => Some(CommentType::Request),
            "reject" => Some(CommentType::Reject),
            "done" => Some(CommentType::Done),
            _ => None,
        }
    }

    /// Returns `true` for comment types that act as context chunk boundaries
    /// (`Reject` and `Done`). Used by GET_HISTORY to split the comment history into chunks.
    pub fn is_cut(&self) -> bool {
        matches!(self, CommentType::Reject | CommentType::Done)
    }
}

// Implement the standard `FromStr` trait so callers can use `.parse()` and to
// appease the `clippy::should_implement_trait` lint.  The inherent `from_str`
// method above remains available for callers who prefer an `Option`-returning
// convenience.
impl std::str::FromStr for CommentType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        CommentType::parse(s).ok_or(())
    }
}

/// A structured comment with metadata.
#[derive(
    Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize, schemars::JsonSchema,
)]
pub struct Comment {
    #[schemars(description = "Comment type (error, report, plan, or request)")]
    pub comment_type: CommentType,
    #[schemars(description = "Timestamp when comment was created (ISO 8601 format)")]
    pub timestamp: String,
    #[schemars(description = "Role of the comment author (None if user-originated)")]
    pub role: Option<Role>,
    #[schemars(description = "Hostname of the system posting the comment")]
    pub hostname: String,
    #[schemars(description = "Execution tool that produced the comment (if known)")]
    pub tool: Option<Tool>,
    #[schemars(description = "AI model used (if applicable)")]
    pub model: Option<Model>,
    #[schemars(description = "Comment text without signature/tag")]
    pub text: String,
}

/// Result of extracting a history chunk, including navigation metadata.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct HistoryChunk {
    /// Index of the returned chunk (0-based, 0 = oldest).
    pub current_chunk: usize,
    /// Index of the last available chunk.
    pub last_chunk: usize,
    /// Comments in this chunk
    pub comments: Vec<Comment>,
}

/// Prepend a task description as a synthetic `Request` comment, then extract
/// the chunk at `offset` from the comment history.
///
/// Chunks are delimited by "cut" comments (`Reject` / `Done`).
/// Chunks are numbered 0 to N where 0 is the oldest and N is the newest.
/// When `offset` is `None`, the last (newest) chunk is returned.
///
/// Non-actionable comments (`Error` and `Done`) are filtered out.
/// Returns an empty `comments` vec when the chunk has no actionable messages.
/// Returns `Err` only for hard failures (offset out of range).
pub fn extract_history_chunk(
    mut comments: Vec<Comment>,
    description: &str,
    offset: Option<usize>,
) -> anyhow::Result<HistoryChunk> {
    // Prepend description as synthetic first comment.
    if !description.is_empty() {
        comments.insert(
            0,
            Comment {
                comment_type: CommentType::Request,
                timestamp: String::new(),
                role: None,
                hostname: String::new(),
                tool: None,
                model: None,
                text: description.to_owned(),
            },
        );
    }

    if comments.is_empty() {
        return Ok(HistoryChunk {
            current_chunk: 0,
            last_chunk: 0,
            comments: Vec::new(),
        });
    }

    // Find cut-boundary indices.
    let cut_indices: Vec<usize> = comments
        .iter()
        .enumerate()
        .filter(|(_, c)| c.comment_type.is_cut())
        .map(|(i, _)| i)
        .collect();

    let num_chunks = cut_indices.len() + 1;
    let last_chunk = num_chunks - 1;

    // Resolve target chunk: None or out-of-range defaults to last.
    let target_chunk = match offset {
        None => last_chunk,
        Some(idx) => {
            anyhow::ensure!(
                idx < num_chunks,
                "offset {} out of range: only {} chunk(s) available (0..{})",
                idx,
                num_chunks,
                last_chunk
            );
            idx
        }
    };

    // Extract chunk boundaries.
    let (start_idx, end_idx) = if cut_indices.is_empty() {
        (0, comments.len())
    } else if target_chunk == 0 {
        (0, cut_indices[0])
    } else if target_chunk == last_chunk {
        (cut_indices[target_chunk - 1], comments.len())
    } else {
        (cut_indices[target_chunk - 1], cut_indices[target_chunk])
    };

    // Filter out Error and Done comments.
    let chunk_comments = comments[start_idx..end_idx]
        .iter()
        .filter(|c| c.comment_type != CommentType::Error && c.comment_type != CommentType::Done)
        .cloned()
        .collect();

    Ok(HistoryChunk {
        current_chunk: target_chunk,
        last_chunk,
        comments: chunk_comments,
    })
}

/// Workflow stage (maps to GitHub milestones internally).
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
pub enum Stage {
    Pending,
    Preparing,
    Planning,
    Working,
    Reviewing,
    Testing,
    Merging,
    Done,
}

impl Stage {
    pub fn milestone_name(&self) -> &'static str {
        match self {
            Stage::Pending => "PENDING",
            Stage::Preparing => "PREPARING",
            Stage::Planning => "PLANNING",
            Stage::Working => "WORKING",
            Stage::Reviewing => "REVIEWING",
            Stage::Testing => "TESTING",
            Stage::Merging => "MERGING",
            Stage::Done => "DONE",
        }
    }

    pub fn from_milestone_name(name: &str) -> Option<Self> {
        match name {
            "PENDING" => Some(Stage::Pending),
            "PREPARING" | "PREPARATION" => Some(Stage::Preparing),
            "PLANNING" => Some(Stage::Planning),
            "WORKING" => Some(Stage::Working),
            "REVIEWING" => Some(Stage::Reviewing),
            "TESTING" => Some(Stage::Testing),
            "MERGING" => Some(Stage::Merging),
            "DONE" => Some(Stage::Done),
            _ => None,
        }
    }

    /// Returns a priority value for task selection by stage proximity.
    /// Lower values = higher priority (closer to completion).
    pub fn priority(&self) -> u8 {
        match self {
            Stage::Testing => 0,
            Stage::Reviewing => 1,
            Stage::Merging => 2,
            Stage::Working => 3,
            Stage::Planning => 4,
            Stage::Preparing => 5,
            Stage::Pending => 6,
            Stage::Done => 7,
        }
    }
}

impl std::fmt::Display for Stage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.milestone_name())
    }
}

/// Role for task execution (planner, worker, reviewer, merger, or user).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub enum Role {
    #[serde(rename = "preparator")]
    Preparator,
    #[serde(rename = "planner")]
    Planner,
    #[serde(rename = "worker")]
    Worker,
    #[serde(rename = "reviewer")]
    Reviewer,
    #[serde(rename = "tester")]
    Tester,
    #[serde(rename = "merger")]
    Merger,
}

impl Role {
    /// Returns the role name as a string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Preparator => "preparator",
            Role::Planner => "planner",
            Role::Worker => "worker",
            Role::Reviewer => "reviewer",
            Role::Tester => "tester",
            Role::Merger => "merger",
        }
    }
}

// ---------------------------------------------------------------------------
// Conversion helpers
// ---------------------------------------------------------------------------

impl From<Role> for Stage {
    fn from(role: Role) -> Stage {
        match role {
            Role::Preparator => Stage::Preparing,
            Role::Planner => Stage::Planning,
            Role::Worker => Stage::Working,
            Role::Reviewer => Stage::Reviewing,
            Role::Tester => Stage::Testing,
            Role::Merger => Stage::Merging,
        }
    }
}

impl std::convert::TryFrom<Stage> for Role {
    type Error = anyhow::Error;

    fn try_from(stage: Stage) -> Result<Self, Self::Error> {
        match stage {
            Stage::Preparing => Ok(Role::Preparator),
            Stage::Planning => Ok(Role::Planner),
            Stage::Working => Ok(Role::Worker),
            Stage::Reviewing => Ok(Role::Reviewer),
            Stage::Testing => Ok(Role::Tester),
            Stage::Merging => Ok(Role::Merger),
            other => Err(anyhow::anyhow!(
                "cannot convert stage {:?} into a role",
                other
            )),
        }
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for Role {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "preparator" => Ok(Role::Preparator),
            "planner" => Ok(Role::Planner),
            "worker" => Ok(Role::Worker),
            "reviewer" => Ok(Role::Reviewer),
            "merger" => Ok(Role::Merger),
            _ => Err(anyhow::anyhow!("Unknown role: {}", s)),
        }
    }
}

/// Signal for task flow control (mapped to labels in GitHub backend).
/// Ordered by priority (highest to lowest): GoPrepare > GoPlan > GoWork > GoReview > GoTest.
///
/// Note: there is no GoMerge signal. Merging is triggered by the `conflict`
/// flag on the Task struct, which is set automatically when a work branch
/// diverges from its base branch.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
pub enum Signal {
    #[serde(rename = "go_prepare")]
    GoPrepare = 1,
    #[serde(rename = "go_plan")]
    GoPlan = 2,
    #[serde(rename = "go_work")]
    GoWork = 3,
    #[serde(rename = "go_review")]
    GoReview = 4,
    #[serde(rename = "go_test")]
    GoTest = 5,
}

impl Signal {
    /// Returns the plain signal name.
    pub fn name(&self) -> &'static str {
        match self {
            Signal::GoReview => "go_review",
            Signal::GoTest => "go_test",
            Signal::GoWork => "go_work",
            Signal::GoPlan => "go_plan",
            Signal::GoPrepare => "go_prepare",
        }
    }

    /// Returns all available signals in priority order.
    pub fn all() -> &'static [Signal] {
        &[
            Signal::GoPrepare,
            Signal::GoPlan,
            Signal::GoWork,
            Signal::GoReview,
            Signal::GoTest,
        ]
    }

    /// Maps signal to the role that should execute the session.
    pub fn target_role(&self) -> Role {
        match self {
            Signal::GoReview => Role::Reviewer,
            Signal::GoTest => Role::Tester,
            Signal::GoWork => Role::Worker,
            Signal::GoPlan => Role::Planner,
            Signal::GoPrepare => Role::Preparator,
        }
    }
}

impl std::fmt::Display for Signal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl std::str::FromStr for Signal {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('_', "").as_str() {
            "goreview" | "go-review" => Ok(Signal::GoReview),
            "gotest" | "go-test" => Ok(Signal::GoTest),
            "gowork" | "go-work" => Ok(Signal::GoWork),
            "goplan" | "go-plan" => Ok(Signal::GoPlan),
            "goprepare" | "go-prepare" => Ok(Signal::GoPrepare),
            _ => Err(anyhow::anyhow!("Unknown signal: {}", s)),
        }
    }
}

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

/// Tag for GitHub-specific comment formatting (e.g., `// REPORT role:host:model`).
///
/// The `model` field is optional and is mainly used by MCP handlers to record the
/// concrete LLM model that generated the message (for example, a Copilot or
/// Claude session).  Dispatcher-originated messages normally leave this field
/// unset, so that comments created by internal code do not imply any model.
/// Agents should supply the model explicitly when they know it; the tag merely
/// serializes whatever value is provided.
///
/// This type handles only serialization/deserialization.  Logic for deciding when
/// to include a model (and what value to use) lives in the dispatcher and MCP
/// helpers rather than here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentTag {
    pub comment_type: CommentType,
    pub role: Option<Role>,
    pub hostname: String,
    /// Which tool executed the action that produced this comment (if any).
    pub tool: Option<Tool>,
    pub model: Option<Model>,
}

impl CommentTag {
    /// Create a new CommentTag.
    pub fn new(
        comment_type: CommentType,
        role: Option<Role>,
        hostname: String,
        tool: Option<Tool>,
        model: Option<Model>,
    ) -> Self {
        Self {
            comment_type,
            role,
            hostname,
            tool,
            model,
        }
    }
}

impl std::fmt::Display for CommentTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tag_type = match self.comment_type {
            CommentType::Error => "ERROR",
            CommentType::Report => "REPORT",
            CommentType::Plan => "PLAN",
            CommentType::Request => "REQUEST",
            CommentType::Reject => "REJECT",
            CommentType::Done => "DONE",
        };

        // All tag types now follow the same serialization rules.  REQUEST no
        // longer has special handling because it may carry a role/host/model,
        // and we want to be able to see `// REQUEST planner:foo:bar` or
        // `// REQUEST user:host` in the log.
        // role is always present now
        let role = self
            .role
            .as_ref()
            .map(|r| r.to_string())
            .unwrap_or_else(|| "user".to_string());

        // Serialization includes optional tool and model.  Maintain backward
        // compatibility by emitting only hostname:model when tool is absent.
        match (&self.tool, &self.model) {
            (Some(tool), Some(model)) =>
                write!(f, "// {} {}:{}:{}:{}", tag_type, role, self.hostname, tool, model),
            (Some(tool), None) =>
                write!(f, "// {} {}:{}:{}", tag_type, role, self.hostname, tool),
            (None, Some(model)) =>
                write!(f, "// {} {}:{}:{}", tag_type, role, self.hostname, model),
            (None, None) => write!(f, "// {} {}:{}", tag_type, role, self.hostname),
        }
    }
}

impl std::str::FromStr for CommentTag {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim_start_matches("//").trim_start();
        let (tag_type_str, rest) = if let Some(pos) = s.find(' ') {
            (&s[..pos], &s[pos + 1..])
        } else {
            (s, "")
        };

        let comment_type = CommentType::parse(tag_type_str)
            .ok_or_else(|| anyhow::anyhow!("Unknown comment type: {}", tag_type_str))?;

let (role, hostname, tool, model) = if rest.is_empty() {
            // no metadata supplied; default to user/request with empty host
            (None, String::new(), None, None)
        } else {
            let parts: Vec<&str> = rest.split(':').collect();
            if parts.len() < 2 {
                return Err(anyhow::anyhow!("Invalid tag format: {}", s));
            }

            let role = Role::from_str(parts[0]).ok();
            let hostname = parts[1].to_string();

            // backwards compatibility: three parts used to mean role:host:model
            let (tool, model) = if parts.len() == 3 {
                (None,
                 if !parts[2].is_empty() && parts[2] != "unknown" {
                     Some(Model::from_str(parts[2])?)
                 } else {
                     None
                 })
            } else {
                let tool = if parts.len() > 2 && !parts[2].is_empty() {
                    Some(Tool::from_str(parts[2])?)
                } else {
                    None
                };
                let model = if parts.len() > 3 && !parts[3].is_empty() && parts[3] != "unknown" {
                    Some(Model::from_str(parts[3])?)
                } else {
                    None
                };
                (tool, model)
            };

            (role, hostname, tool, model)
        };

        Ok(CommentTag {
            comment_type,
            role,
            hostname,
            tool,
            model,
        })
    }
}

/// A task in the abstract domain (generic, backed by GitHub or Filesystem).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Task {
    pub id: u64,
    pub title: String,
    pub description: String,
    pub stage: Stage,
    pub destination_repository: Option<String>,
    pub destination_branch: Option<String>,
    pub work_branch: Option<String>,
    pub parameters: HashMap<Parameter, String>,
    pub checklist: Vec<ChecklistItem>,
    pub signal: Option<Signal>,
    pub conflict: bool,
    pub pause: bool,
    /// When true the dispatcher will automatically set the pause flag any time
    /// the task's stage is changed.  This gives human operators an opportunity to
    /// review a transition before the next processing step occurs.
    pub confirm: bool,
    /// ETag for optimistic locking to prevent concurrent update conflicts.
    /// Used to detect if the task has been modified between read and write operations.
    #[serde(skip)]
    pub etag: Option<String>,
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
