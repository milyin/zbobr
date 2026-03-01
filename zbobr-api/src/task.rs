use std::collections::HashMap;

// -- Parameter names enum --

/// Standardized parameter names for task configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Parameter {
    DestinationRepository,
    DestinationBranch,
    WorkBranch,
    PrUrl,
}

impl Parameter {
    /// Returns the parameter name as a string.
    pub fn name(&self) -> &'static str {
        match self {
            Parameter::DestinationRepository => "destination_repository",
            Parameter::DestinationBranch => "destination_branch",
            Parameter::WorkBranch => "work_branch",
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
            "MERGING" => Some(Stage::Merging),
            "DONE" => Some(Stage::Done),
            _ => None,
        }
    }
}

impl std::fmt::Display for Stage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.milestone_name())
    }
}

/// Role for task execution (planner, worker, reviewer, or merger).
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
/// Ordered by priority (highest to lowest): GoPrepare > GoPlan > GoWork > GoReview.
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
}

impl Signal {
    /// Returns the plain signal name.
    pub fn name(&self) -> &'static str {
        match self {
            Signal::GoReview => "go_review",
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
        ]
    }

    /// Maps signal to the role that should execute the session.
    pub fn target_role(&self) -> Role {
        match self {
            Signal::GoReview => Role::Reviewer,
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
    #[serde(rename = "gpt-4o")]
    Gpt4o,
    #[serde(rename = "gpt-5-mini")]
    #[default]
    Gpt5Mini,
    #[serde(rename = "claude-3-5-sonnet")]
    Claude35Sonnet,
    #[serde(rename = "claude-3-opus")]
    Claude3Opus,
    #[serde(rename = "claude-sonnet-4.5")]
    ClaudeSonnet4_5,
    #[serde(rename = "claude-haiku-4.5")]
    ClaudeHaiku4_5,
    #[serde(rename = "claude-opus-4.6")]
    ClaudeOpus4_6,
    #[serde(rename = "claude-opus-4.5")]
    ClaudeOpus4_5,
    #[serde(rename = "claude-sonnet-4")]
    ClaudeSonnet4,
    #[serde(rename = "gemini-3-pro-preview")]
    Gemini3ProPreview,
    #[serde(rename = "gpt-5.2-codex")]
    Gpt5_2Codex,
    #[serde(rename = "gpt-5.2")]
    Gpt5_2,
    #[serde(rename = "gpt-5.1-codex-max")]
    Gpt5_1CodexMax,
    #[serde(rename = "gpt-5.1-codex")]
    Gpt5_1Codex,
}

impl Model {
    /// Returns all available models.
    pub fn all() -> &'static [Model] {
        &[
            Model::Gpt4o,
            Model::Gpt5Mini,
            Model::Claude35Sonnet,
            Model::Claude3Opus,
            Model::ClaudeSonnet4_5,
            Model::ClaudeHaiku4_5,
            Model::ClaudeOpus4_6,
            Model::ClaudeOpus4_5,
            Model::ClaudeSonnet4,
            Model::Gemini3ProPreview,
            Model::Gpt5_2Codex,
            Model::Gpt5_2,
            Model::Gpt5_1CodexMax,
            Model::Gpt5_1Codex,
        ]
    }

    pub fn model_name_for_tool(&self, tool: Tool) -> Option<&'static str> {
        match tool {
            Tool::Copilot => match self {
                Model::Gpt4o => Some("gpt-4o"),
                Model::Gpt5Mini => Some("gpt-5-mini"),
                Model::Claude35Sonnet => Some("claude-3-5-sonnet"),
                Model::Claude3Opus => Some("claude-3-opus"),
                Model::ClaudeSonnet4_5 => Some("claude-sonnet-4.5"),
                Model::ClaudeHaiku4_5 => Some("claude-haiku-4.5"),
                Model::ClaudeOpus4_6 => Some("claude-opus-4.6"),
                Model::ClaudeOpus4_5 => Some("claude-opus-4.5"),
                Model::ClaudeSonnet4 => Some("claude-sonnet-4"),
                Model::Gemini3ProPreview => Some("gemini-3-pro-preview"),
                Model::Gpt5_2Codex => Some("gpt-5.2-codex"),
                Model::Gpt5_2 => Some("gpt-5.2"),
                Model::Gpt5_1CodexMax => Some("gpt-5.1-codex-max"),
                Model::Gpt5_1Codex => Some("gpt-5.1-codex"),
            },
            Tool::Claude => match self {
                Model::Claude35Sonnet => Some("sonnet"),
                Model::Claude3Opus => Some("opus"),
                _ => None,
            },
            Tool::McpTester => None,
        }
    }
}

impl std::fmt::Display for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Model::Gpt4o => "gpt-4o",
            Model::Gpt5Mini => "gpt-5-mini",
            Model::Claude35Sonnet => "claude-3-5-sonnet",
            Model::Claude3Opus => "claude-3-opus",
            Model::ClaudeSonnet4_5 => "claude-sonnet-4.5",
            Model::ClaudeHaiku4_5 => "claude-haiku-4.5",
            Model::ClaudeOpus4_6 => "claude-opus-4.6",
            Model::ClaudeOpus4_5 => "claude-opus-4.5",
            Model::ClaudeSonnet4 => "claude-sonnet-4",
            Model::Gemini3ProPreview => "gemini-3-pro-preview",
            Model::Gpt5_2Codex => "gpt-5.2-codex",
            Model::Gpt5_2 => "gpt-5.2",
            Model::Gpt5_1CodexMax => "gpt-5.1-codex-max",
            Model::Gpt5_1Codex => "gpt-5.1-codex",
        };
        f.write_str(s)
    }
}

impl std::str::FromStr for Model {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('.', "-").as_str() {
            "gpt-4o" | "gpt4o" => Ok(Model::Gpt4o),
            "gpt-5-mini" | "gpt5mini" | "gpt-5" => Ok(Model::Gpt5Mini),
            "claude-3-5-sonnet" | "claude35sonnet" | "claude-3.5-sonnet" => {
                Ok(Model::Claude35Sonnet)
            }
            "claude-3-opus" | "claude3opus" => Ok(Model::Claude3Opus),
            "claude-sonnet-4.5" | "claude-sonnet-4-5" => Ok(Model::ClaudeSonnet4_5),
            "claude-haiku-4.5" | "claude-haiku-4-5" => Ok(Model::ClaudeHaiku4_5),
            "claude-opus-4.6" | "claude-opus-4-6" => Ok(Model::ClaudeOpus4_6),
            "claude-opus-4.5" | "claude-opus-4-5" => Ok(Model::ClaudeOpus4_5),
            "claude-sonnet-4" => Ok(Model::ClaudeSonnet4),
            "gemini-3-pro-preview" => Ok(Model::Gemini3ProPreview),
            "gpt-5.2-codex" | "gpt-5-2-codex" => Ok(Model::Gpt5_2Codex),
            "gpt-5.2" | "gpt-5-2" => Ok(Model::Gpt5_2),
            "gpt-5.1-codex-max" | "gpt-5-1-codex-max" => Ok(Model::Gpt5_1CodexMax),
            "gpt-5.1-codex" | "gpt-5-1-codex" => Ok(Model::Gpt5_1Codex),
            _ => Err(anyhow::anyhow!("Unknown model: {}", s)),
        }
    }
}

/// A task in the abstract domain (generic, backed by GitHub or Filesystem).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Task {
    pub id: u64,
    pub title: String,
    pub description: String,
    pub plan: String,
    pub stage: Stage,
    pub tool: Option<Tool>,
    pub model: Option<Model>,
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
