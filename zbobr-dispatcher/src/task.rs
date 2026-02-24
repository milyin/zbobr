use std::collections::HashMap;

use crate::Zbobr;

// -- Parameter names enum --

/// Standardized parameter names for task configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Parameter {
    DestinationRepository,
    DestinationBranch,
    WorkBranch,
    PrUrl,
    ResumeSignal,
}

impl Parameter {
    /// Returns the parameter name as a string.
    pub fn name(&self) -> &'static str {
        match self {
            Parameter::DestinationRepository => "destination_repository",
            Parameter::DestinationBranch => "destination_branch",
            Parameter::WorkBranch => "work_branch",
            Parameter::PrUrl => "pr_url",
            Parameter::ResumeSignal => "resume_signal",
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
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub enum Stage {
    Pending,
    Preparation,
    Planning,
    Working,
    Reviewing,
    Merging,
}

impl Stage {
    pub fn milestone_name(&self) -> &'static str {
        match self {
            Stage::Pending => "PENDING",
            Stage::Preparation => "PREPARATION",
            Stage::Planning => "PLANNING",
            Stage::Working => "WORKING",
            Stage::Reviewing => "REVIEWING",
            Stage::Merging => "MERGING",
        }
    }

    pub fn from_milestone_name(name: &str) -> Option<Self> {
        match name {
            "PENDING" => Some(Stage::Pending),
            "PREPARATION" => Some(Stage::Preparation),
            "PLANNING" => Some(Stage::Planning),
            "WORKING" => Some(Stage::Working),
            "REVIEWING" => Some(Stage::Reviewing),
            "MERGING" => Some(Stage::Merging),
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
/// Ordered by priority (highest to lowest).
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
    #[serde(rename = "stop")]
    Stop = 0,
    #[serde(rename = "go_ask")]
    GoAsk = 1, // go ask is higher priority than done because it indicates a need for human input,
    // while done supposes normal completion and less urgency. So go_ask should override done
    // if both are signalled by an agent.
    #[serde(rename = "done")]
    Done = 2,
    #[serde(rename = "go_merge")]
    GoMerge = 3,
    #[serde(rename = "go_review")]
    GoReview = 4,
    #[serde(rename = "go_work")]
    GoWork = 5,
    #[serde(rename = "go_plan")]
    GoPlan = 6,
    #[serde(rename = "go_prepare")]
    GoPrepare = 7,
}

impl Signal {
    /// Returns the plain signal name.
    pub fn name(&self) -> &'static str {
        match self {
            Signal::Stop => "stop",
            Signal::Done => "done",
            Signal::GoAsk => "go_ask",
            Signal::GoMerge => "go_merge",
            Signal::GoReview => "go_review",
            Signal::GoWork => "go_work",
            Signal::GoPlan => "go_plan",
            Signal::GoPrepare => "go_prepare",
        }
    }

    /// Returns all available signals in priority order.
    pub fn all() -> &'static [Signal] {
        &[
            Signal::Stop,
            Signal::Done,
            Signal::GoAsk,
            Signal::GoMerge,
            Signal::GoReview,
            Signal::GoWork,
            Signal::GoPlan,
            Signal::GoPrepare,
        ]
    }

    /// Maps signal to target stage.
    pub fn target_stage(&self) -> Stage {
        match self {
            Signal::Stop | Signal::Done | Signal::GoAsk => Stage::Pending,
            Signal::GoMerge => Stage::Merging,
            Signal::GoReview => Stage::Reviewing,
            Signal::GoWork => Stage::Working,
            Signal::GoPlan => Stage::Planning,
            Signal::GoPrepare => Stage::Preparation,
        }
    }

    /// Maps signal to role for session execution.
    pub fn target_role(&self) -> Option<Role> {
        match self {
            Signal::GoMerge => Some(Role::Merger),
            Signal::GoReview => Some(Role::Reviewer),
            Signal::GoWork => Some(Role::Worker),
            Signal::GoPlan => Some(Role::Planner),
            Signal::GoPrepare => Some(Role::Preparator),
            _ => None,
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
            "stop" => Ok(Signal::Stop),
            "done" => Ok(Signal::Done),
            "goask" | "go-ask" => Ok(Signal::GoAsk),
            "gomerge" | "go-merge" => Ok(Signal::GoMerge),
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

/// A task in the abstract domain (generic, backed by GitHub or Stub).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Task {
    pub id: u64,
    pub title: String,
    pub description: String,
    pub plan: String,
    pub discussion: Vec<String>,
    pub stage: Stage,
    pub tool: Option<Tool>,
    pub model: Option<Model>,
    pub parameters: HashMap<Parameter, String>,
    pub done: bool,
    pub checklist: Vec<ChecklistItem>,
    pub signal: Option<Signal>,
    /// ETag for optimistic locking to prevent concurrent update conflicts.
    /// Used to detect if the task has been modified between read and write operations.
    #[serde(skip)]
    pub etag: Option<String>,
}

/// Task session bound to a specific task, with role-based behavior.
#[derive(Clone)]
pub struct TaskSession {
    zbobr: Zbobr,
    task_id: u64,
}

impl TaskSession {
    pub(crate) fn new(zbobr: Zbobr, task_id: u64) -> Self {
        Self { zbobr, task_id }
    }

    pub fn task_id(&self) -> u64 {
        self.task_id
    }

    /// Create a branch name with the proper prefix for this task.
    pub fn create_branch_name(&self, short_name: &str) -> String {
        format!(
            "{}-{}-{}",
            self.zbobr.config().work_branch_prefix,
            self.task_id,
            short_name
        )
    }

    /// Check whether a branch name starts with this task's prefix.
    pub fn validate_branch_prefix(&self, branch: &str) -> bool {
        let prefix = format!(
            "{}-{}-",
            self.zbobr.config().work_branch_prefix,
            self.task_id
        );
        branch.starts_with(&prefix)
    }

    /// Read the full task state.
    pub async fn get_task(&self) -> anyhow::Result<Task> {
        self.zbobr.get_task(self.task_id).await
    }

    /// Get the current task description.
    pub async fn get_description(&self) -> anyhow::Result<String> {
        Ok(self.get_task().await?.description)
    }

    /// Get the current task plan.
    pub async fn get_plan(&self) -> anyhow::Result<String> {
        Ok(self.get_task().await?.plan)
    }

    /// Get the current task checklist.
    pub async fn get_checklist(&self) -> anyhow::Result<Vec<ChecklistItem>> {
        Ok(self.get_task().await?.checklist)
    }

    /// Atomically read-modify-write the task body (description, parameters, plan, checklist).
    ///
    /// The closure receives a mutable `Task` reference and may modify `description`,
    /// `parameters`, `plan`, and `checklist`. All other `Task` fields are ignored on write.
    ///
    /// Concurrent `modify_task` calls on the same task are serialized by an in-process
    /// per-task mutex, so concurrent MCP tool calls cannot overwrite each other's changes.
    /// Cross-process conflicts are handled by backend-level three-way merge.
    pub async fn modify_task<F>(&self, mutate: F) -> anyhow::Result<()>
    where
        F: FnOnce(&mut Task) + Send + 'static,
    {
        // Acquire per-task lock to serialize concurrent modify_task calls
        let lock = self.zbobr.task_lock(self.task_id);
        let _guard = lock.lock().await;

        self.zbobr
            .task_backend
            .modify_task(
                self.task_id,
                Box::new(move |mut task| {
                    mutate(&mut task);
                    task
                }),
            )
            .await
    }

    /// Update the task description (only the description part, preserving plan/checklist/parameters).
    pub async fn update_description(&self, description: &str) -> anyhow::Result<()> {
        let desc = description.to_string();
        self.modify_task(move |task| {
            task.description = desc;
        })
        .await
    }

    /// Update the task plan (preserving description/checklist/parameters).
    pub async fn update_plan(&self, plan: &str) -> anyhow::Result<()> {
        let plan = plan.to_string();
        self.modify_task(move |task| {
            task.plan = plan;
        })
        .await
    }

    /// Update the task checklist (preserving description/plan/parameters).
    pub async fn update_checklist(&self, checklist: &[ChecklistItem]) -> anyhow::Result<()> {
        let items = checklist.to_vec();
        self.modify_task(move |task| {
            task.checklist = items;
        })
        .await
    }

    /// Get all discussion messages on the task.
    pub async fn get_discussion(&self) -> anyhow::Result<Vec<String>> {
        self.zbobr.get_task_comments(self.task_id).await
    }

    /// Post a message to the task discussion with role and hostname metadata.
    pub async fn post_message(&self, msg: &str, role: &str, hostname: &str) -> anyhow::Result<()> {
        self.zbobr
            .post_task_comment(self.task_id, msg, role, hostname)
            .await
    }

    /// Get the current signal on the task.
    pub async fn get_signal(&self) -> anyhow::Result<Option<Signal>> {
        let task = self.zbobr.get_task(self.task_id).await?;
        Ok(task.signal)
    }

    /// Set signal on the task, respecting priority (higher priority signals cannot be overwritten by lower).
    pub async fn set_signal(&self, new_signal: Signal) -> anyhow::Result<()> {
        self.modify_task(move |task| {
            // Only set if new signal has higher or equal priority (lower enum value)
            if let Some(current_signal) = task.signal
                && new_signal > current_signal
            {
                // new_signal has lower priority, don't overwrite
                return;
            }
            task.signal = Some(new_signal);
        })
        .await
    }

    /// Clear the signal on the task.
    pub async fn clear_signal(&self) -> anyhow::Result<()> {
        self.modify_task(move |task| {
            task.signal = None;
        })
        .await
    }

    /// Transition task to stage based on current signal.
    pub async fn transition_by_signal(&self) -> anyhow::Result<()> {
        self.modify_task(move |task| {
            if let Some(sig) = task.signal {
                task.stage = sig.target_stage();
            }
        })
        .await
    }

    /// Clone target repo and checkout specific branch (read-only, for planner).
    pub async fn request_branch_readonly(
        &self,
        repo: &str,
        branch: &str,
    ) -> anyhow::Result<String> {
        let path = self
            .zbobr
            .clone_readonly(repo, branch, self.task_id)
            .await?;
        let path_str = path.to_string_lossy().to_string();
        Ok(path_str)
    }

    /// Fork target repo, clone locally, checkout specific branch (for worker).
    pub async fn request_branch(&self, repo: &str, branch: &str) -> anyhow::Result<String> {
        let path = self
            .zbobr
            .clone_and_setup(repo, branch, self.task_id)
            .await?;
        let path_str = path.to_string_lossy().to_string();
        Ok(path_str)
    }

    /// Helper: Clone repo and checkout branch from PR.
    /// PR format: "https://github.com/owner/repo/pull/123" or "owner/repo#123"
    pub async fn request_branch_by_pr(&self, pr: &str, readonly: bool) -> anyhow::Result<String> {
        let (repo, branch) = self.zbobr.parse_pr_to_repo_branch(pr).await?;
        if readonly {
            self.request_branch_readonly(&repo, &branch).await
        } else {
            self.request_branch(&repo, &branch).await
        }
    }

    /// Push the current branch to the fork remote.
    /// Validates that the current branch has the correct task prefix.
    pub async fn push_branch(&self, path: &str) -> anyhow::Result<()> {
        let work_dir = std::path::PathBuf::from(path);

        if !work_dir.exists() {
            anyhow::bail!("Work directory does not exist: {}", work_dir.display());
        }

        // Get current branch name
        let output = tokio::process::Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(&work_dir)
            .output()
            .await?;

        if !output.status.success() {
            anyhow::bail!("Failed to get current branch");
        }

        let current_branch = String::from_utf8_lossy(&output.stdout).trim().to_string();

        if !self.validate_branch_prefix(&current_branch) {
            anyhow::bail!(
                "Branch '{}' does not match expected prefix '{}/{}/'. Use create_branch_name to generate a valid branch name.",
                current_branch,
                self.zbobr.config().work_branch_prefix,
                self.task_id
            );
        }

        // Push to fork
        tracing::info!("Pushing branch '{}' to fork", current_branch);
        let status = tokio::process::Command::new("git")
            .args(["push", "-u", "fork", "HEAD", "--force"])
            .current_dir(&work_dir)
            .status()
            .await?;

        if !status.success() {
            anyhow::bail!("Failed to push to fork");
        }

        Ok(())
    }

    /// Push the current branch and create a PR within the fork.
    /// The PR is created in the fork repo with `destination_branch` as base.
    pub async fn push_branch_and_create_pr(
        &self,
        path: &str,
        destination_branch: &str,
    ) -> anyhow::Result<String> {
        // First push the branch
        self.push_branch(path).await?;

        let work_dir = std::path::PathBuf::from(path);

        // Get current branch name (already validated by push_branch)
        let output = tokio::process::Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(&work_dir)
            .output()
            .await?;
        let current_branch = String::from_utf8_lossy(&output.stdout).trim().to_string();

        // Derive repository name from work directory name (workspace/task#/repo)
        let repo_name = work_dir
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow::anyhow!("Could not determine repo name from path: {}", path))?
            .to_string();

        // Build PR metadata from task (decoupled from repo backend)
        let task = self.get_task().await?;
        let pr_title = format!("Fix #{}: {}", self.task_id, task.title);
        let pr_body = format!(
            "Resolves #{}\n\nImplementation for: {}",
            self.task_id, task.title
        );

        // Create PR using the backend (which knows the fork owner)
        let pr_url = self
            .zbobr
            .create_pr_in_fork(
                &repo_name,
                &current_branch,
                destination_branch,
                &pr_title,
                &pr_body,
            )
            .await?;
        Ok(pr_url)
    }

    /// Push the work_branch in the cloned repository. Stashes local changes if a different branch is selected.
    /// Rejects the push if there are uncommitted changes - all work must be committed before pushing.
    /// The work repository has all remote information cleared - only pull_work and push_work know where to push.
    pub async fn push_work(&self) -> anyhow::Result<()> {
        // Get the destination repo (needed to find the cloned path)
        let dest_repo = self
            .get_parameter(Parameter::DestinationRepository)
            .await?
            .ok_or_else(|| anyhow::anyhow!("destination_repository parameter not set"))?;

        // Compute the work directory: workspace/task#<id>/<repo>
        let repo_name = extract_repo_name(&dest_repo).ok_or_else(|| {
            anyhow::anyhow!("Invalid destination_repository format: {}", dest_repo)
        })?;

        let work_dir = self
            .zbobr
            .config()
            .workspaces
            .join(format!("task#{}", self.task_id))
            .join(repo_name);

        if !work_dir.exists() {
            anyhow::bail!("Work directory does not exist: {}", work_dir.display());
        }

        // Get the work_branch name
        let work_branch = self
            .get_parameter(Parameter::WorkBranch)
            .await?
            .ok_or_else(|| anyhow::anyhow!("work_branch parameter not set"))?;

        // Get current branch
        let output = tokio::process::Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(&work_dir)
            .output()
            .await?;

        if !output.status.success() {
            anyhow::bail!("Failed to get current branch");
        }

        let current_branch = String::from_utf8_lossy(&output.stdout).trim().to_string();

        // If on a different branch, stash changes
        if current_branch != work_branch {
            tracing::info!(
                "Stashing changes on branch '{}' before switching to '{}'",
                current_branch,
                work_branch
            );
            let stash_status = tokio::process::Command::new("git")
                .args(["stash"])
                .current_dir(&work_dir)
                .status()
                .await?;

            if !stash_status.success() {
                tracing::warn!("Stash may have failed or nothing to stash");
            }

            // Switch to work branch
            tracing::info!("Switching to branch '{}'", work_branch);
            let checkout_status = tokio::process::Command::new("git")
                .args(["checkout", &work_branch])
                .current_dir(&work_dir)
                .status()
                .await?;

            if !checkout_status.success() {
                anyhow::bail!("Failed to checkout branch '{}'", work_branch);
            }
        }

        // Check for uncommitted changes before pushing
        tracing::info!(
            "Checking for uncommitted changes on branch '{}'",
            work_branch
        );
        let status_output = tokio::process::Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&work_dir)
            .output()
            .await?;

        if !status_output.status.success() {
            anyhow::bail!("Failed to check git status");
        }

        let uncommitted = String::from_utf8_lossy(&status_output.stdout)
            .trim()
            .to_string();
        if !uncommitted.is_empty() {
            anyhow::bail!(
                "Cannot push: there are uncommitted changes on branch '{}'. Please commit all changes before pushing.\n\nUncommitted files:\n{}",
                work_branch,
                uncommitted
            );
        }

        // Push to the configured remote (set by pull_work)
        tracing::info!("Pushing branch '{}' to remote", work_branch);
        let status = tokio::process::Command::new("git")
            .args(["push", "-u", "origin", "HEAD", "--force"])
            .current_dir(&work_dir)
            .status()
            .await?;

        if !status.success() {
            anyhow::bail!("Failed to push work branch");
        }

        Ok(())
    }

    /// Pull a repository, forking if needed. Clones the destination_repository fork, creates and checks out work_branch.
    /// Cleans up remote information - only pull_work and push_work know where to push/pull.
    /// Stashes local changes if a different branch is selected as current.
    /// Also creates a PR from work_branch to destination_branch in the fork repo if all parameters are set.
    pub async fn pull_work(&self) -> anyhow::Result<String> {
        // Get required parameters
        let dest_repo = self
            .get_parameter(Parameter::DestinationRepository)
            .await?
            .ok_or_else(|| anyhow::anyhow!("destination_repository parameter not set"))?;

        let dest_branch = self
            .get_parameter(Parameter::DestinationBranch)
            .await?
            .ok_or_else(|| anyhow::anyhow!("destination_branch parameter not set"))?;

        let work_branch = self
            .get_parameter(Parameter::WorkBranch)
            .await?
            .ok_or_else(|| anyhow::anyhow!("work_branch parameter not set"))?;

        // Clone and setup the repository with forking
        // Ensure the fork in GitHub is synchronized with the destination repository
        tracing::info!(
            "Synchronizing fork for {} on branch {}",
            dest_repo,
            dest_branch
        );
        self.zbobr.sync_fork(&dest_repo, &dest_branch).await?;

        let repo_name = dest_repo.split('/').last().unwrap_or(&dest_repo);
        let path = self.zbobr.config().workspaces.join(format!("task#{}", self.task_id)).join(repo_name);

        // Check if repo exists and is in conflict state
        let is_conflicted = if path.exists() {
            let status = tokio::process::Command::new("git")
                .args(["ls-files", "-u"])
                .current_dir(&path)
                .output()
                .await;
            if let Ok(output) = status {
                !output.stdout.is_empty()
            } else {
                false
            }
        } else {
            false
        };

        if is_conflicted {
            tracing::info!("Repository is in a conflicted state, skipping clone_and_setup and merge");
            return Ok(path.to_string_lossy().to_string());
        }

        let path = self
            .zbobr
            .clone_and_setup(&dest_repo, &dest_branch, self.task_id)
            .await?;

        let path_str = path.to_string_lossy().to_string();

        // No tracking required: local path layout is workspace/task#<id>/<repo>

        // Create the work branch from destination_branch if it doesn't exist.
        // First check whether the branch already exists locally to avoid an error
        // from `git checkout -b` when the branch is present.
        let branch_ref = format!("refs/heads/{}", work_branch);
        let exists_locally = tokio::process::Command::new("git")
            .args(["show-ref", "--verify", &branch_ref])
            .current_dir(&path)
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false);

        let exists_on_remote = if !exists_locally {
            tokio::process::Command::new("git")
                .args(["fetch", "origin", &work_branch])
                .current_dir(&path)
                .status()
                .await
                .map(|s| s.success())
                .unwrap_or(false)
        } else {
            false
        };

        if exists_locally || exists_on_remote {
            if exists_on_remote {
                // Branch exists on remote but not locally; create it tracking the remote
                let checkout_status = tokio::process::Command::new("git")
                    .args(["checkout", "-b", &work_branch, "FETCH_HEAD"])
                    .current_dir(&path)
                    .status()
                    .await?;

                if !checkout_status.success() {
                    anyhow::bail!("Failed to checkout remote work branch '{}'", work_branch);
                }
            } else {
                // Branch exists locally; simply checkout
                let checkout_status = tokio::process::Command::new("git")
                    .args(["checkout", &work_branch])
                    .current_dir(&path)
                    .status()
                    .await?;

                if !checkout_status.success() {
                    anyhow::bail!("Failed to checkout existing work branch '{}'", work_branch);
                }
            }

            // Merge destination branch into work branch to pick up upstream changes.
            // Configure git user first so merge commits (if any) have valid author info.
            let config = self.zbobr.config();
            crate::backend::configure_git_user(
                &path,
                &config.git_user_name,
                &config.git_user_email,
            )
            .await?;

            tracing::info!(
                "Merging destination branch '{}' into work branch '{}'",
                dest_branch,
                work_branch
            );
            let merge_output = tokio::process::Command::new("git")
                .args(["merge", &dest_branch, "--no-edit"])
                .current_dir(&path)
                .output()
                .await?;

            if !merge_output.status.success() {
                let stderr = String::from_utf8_lossy(&merge_output.stderr);
                let stdout = String::from_utf8_lossy(&merge_output.stdout);

                // Check if there are merge conflicts
                let has_conflicts = stderr.contains("CONFLICT")
                    || stdout.contains("CONFLICT")
                    || stderr.contains("Automatic merge failed");

                if has_conflicts {
                    // Don't abort the merge yet - leave it in conflict state for the merger agent
                    // The merger agent will examine the conflicts and try to resolve them

                    let hostname = hostname::get()
                        .ok()
                        .and_then(|h| h.into_string().ok())
                        .unwrap_or_else(|| "unknown".to_string());
                    let user_msg = format!(
                        "⚠️  Merge conflict detected while automatically merging destination branch '{}' into work branch '{}'. \
                         A merger agent has been started to attempt automatic conflict resolution.",
                        dest_branch, work_branch
                    );
                    let _ = self.post_message(&user_msg, "system", &hostname).await;

                    // Signal the merger to handle this
                    let _ = self.set_signal(Signal::GoMerge).await;

                    tracing::info!(
                        "Merge conflict detected for task #{}, signaling GoMerge",
                        self.task_id
                    );
                    
                    // Return early, do not attempt to push or create PR while in conflict state
                    return Ok(path_str);
                } else {
                    // Non-conflict merge failure
                    anyhow::bail!(
                        "Failed to merge '{}' into '{}': {}",
                        dest_branch,
                        work_branch,
                        stderr.trim()
                    );
                }
            } else {
                tracing::info!(
                    "Successfully merged '{}' into '{}'",
                    dest_branch,
                    work_branch
                );
            }
        } else {
            // Branch does not exist locally; create it from current HEAD (destination branch)
            let create_branch = tokio::process::Command::new("git")
                .args(["checkout", "-b", &work_branch])
                .current_dir(&path)
                .status()
                .await?;

            if !create_branch.success() {
                anyhow::bail!("Failed to create work branch '{}'", work_branch);
            }

            // Create a placeholder file to ensure the branch has at least one commit
            // (GitHub PR API rejects branches with no commits between them)
            let config = self.zbobr.config();
            crate::backend::configure_git_user(
                &path,
                &config.git_user_name,
                &config.git_user_email,
            )
            .await?;
            crate::backend::create_placeholder_commit(&path, &work_branch).await?;
        }

        // Clean up remote information - remove fork if it was previously set up
        // Set up fork remote and push work branch (backend handles fork_owner internally)
        self.zbobr
            .setup_fork_remote_and_push(&path, &dest_repo, &work_branch)
            .await?;

        // Create PR from work_branch to destination_branch in the fork repo
        let repo_name = extract_repo_name(&dest_repo).ok_or_else(|| {
            anyhow::anyhow!("Invalid destination_repository format: {}", dest_repo)
        })?;
        if let Err(e) = self
            .create_pr_for_work_branch(&repo_name, &work_branch, &dest_branch)
            .await
        {
            // Log the error and also notify the user via task discussion, but don't fail the pull_work
            tracing::error!(
                "Failed to create PR for work branch {} -> {}: {e}",
                work_branch,
                dest_branch
            );
            let hostname = hostname::get()
                .ok()
                .and_then(|h| h.into_string().ok())
                .unwrap_or_else(|| "unknown".to_string());
            let msg = format!(
                "⚠️  Failed to create PR: {}. You can create it manually or continue working.",
                e
            );
            let _ = self.post_message(&msg, "worker", &hostname).await;
        }

        // Rename 'origin' to a temporary name, configure it with internal credentials, then back to origin
        // This ensures the model can't directly access remote URLs
        tracing::info!("Setting up internal remote for pull_work/push_work only");

        Ok(path_str)
    }

    /// Create a PR from work_branch to destination_branch in the fork repo.
    async fn create_pr_for_work_branch(
        &self,
        repo_name: &str,
        work_branch: &str,
        destination_branch: &str,
    ) -> anyhow::Result<()> {
        tracing::info!(
            "Creating PR for repo '{}' from {} to {}",
            repo_name,
            work_branch,
            destination_branch
        );

        // If a PR URL is already stored in the task parameters, verify it exists
        // and skip creating a new PR when that's the case.
        if let Ok(Some(existing_pr)) = self.get_parameter(Parameter::PrUrl).await {
            match self.zbobr.parse_pr_to_repo_branch(&existing_pr).await {
                Ok((_repo, _branch)) => {
                    tracing::info!(
                        "PR already exists for task {}: {}",
                        self.task_id,
                        existing_pr
                    );
                    return Ok(());
                }
                Err(e) => {
                    tracing::info!(
                        "Stored pr_url '{}' could not be verified: {}. Creating new PR.",
                        existing_pr,
                        e
                    );
                }
            }
        }
        // Guard: work_branch must be a branch name or owner:branch, but not a full repo path
        // (e.g. owner/repo/branch) which is invalid for the Pulls API head field.
        let slash_count = work_branch.chars().filter(|c| *c == '/').count();
        if slash_count >= 2 {
            anyhow::bail!(
                "work_branch has invalid format '{}'. Use a branch name like 'feature/x' or 'owner:branch', not 'owner/repo/branch'.",
                work_branch
            );
        }

        // Build PR metadata from task (decoupled from repo backend)
        let task = self.get_task().await?;
        let pr_title = format!("Fix #{}: {}", self.task_id, task.title);
        let pr_body = format!(
            "Resolves #{}\n\nImplementation for: {}",
            self.task_id, task.title
        );

        let pr_url = self
            .zbobr
            .create_pr_in_fork(
                repo_name,
                work_branch,
                destination_branch,
                &pr_title,
                &pr_body,
            )
            .await?;

        // Store the PR URL in the task
        self.set_parameter(Parameter::PrUrl, Some(pr_url)).await?;

        Ok(())
    }

    /// Mark task as done (sets signal to Done). Stage transition will be handled by main loop.
    pub async fn mark_done(&self) -> anyhow::Result<()> {
        self.set_signal(Signal::Done).await?;
        Ok(())
    }

    /// Get a task parameter value. Parameters are stored in the task's parameters HashMap.
    pub async fn get_parameter(&self, param: Parameter) -> anyhow::Result<Option<String>> {
        let task = self.zbobr.get_task(self.task_id).await?;
        Ok(task.parameters.get(&param).cloned())
    }

    /// Set a task parameter value with automatic conflict detection.
    /// Parameters are stored in the visible PARAMETERS section.
    pub async fn set_parameter(
        &self,
        param: Parameter,
        value: Option<String>,
    ) -> anyhow::Result<()> {
        self.modify_task(move |task| {
            if let Some(v) = value {
                task.parameters.insert(param, v);
            } else {
                task.parameters.remove(&param);
            }
        })
        .await
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_milestone_names() {
        assert_eq!(Stage::Pending.milestone_name(), "PENDING");
        assert_eq!(Stage::Planning.milestone_name(), "PLANNING");
        assert_eq!(Stage::Working.milestone_name(), "WORKING");
        assert_eq!(Stage::Reviewing.milestone_name(), "REVIEWING");
        assert_eq!(Stage::Preparation.milestone_name(), "PREPARATION");
        assert_eq!(Stage::Merging.milestone_name(), "MERGING");
    }

    #[test]
    fn stage_display() {
        assert_eq!(Stage::Planning.to_string(), "PLANNING");
        assert_eq!(Stage::Working.to_string(), "WORKING");
        assert_eq!(Stage::Reviewing.to_string(), "REVIEWING");
    }

    #[test]
    fn stage_roundtrip_serde() {
        let stage = Stage::Planning;
        let json = serde_json::to_string(&stage).unwrap();
        let back: Stage = serde_json::from_str(&json).unwrap();
        assert_eq!(back, stage);
    }

    #[test]
    fn task_serde() {
        let task = Task {
            id: 42,
            title: "Test task".to_string(),
            description: "Do something".to_string(),
            plan: String::new(),
            discussion: vec!["Hello".to_string()],
            stage: Stage::Planning,
            tool: Some(Tool::Claude),
            model: Some(Model::Claude3Opus),
            parameters: HashMap::new(),
            done: false,
            checklist: vec![],
            signal: None,
            etag: None,
        };
        let json = serde_json::to_string(&task).unwrap();
        let back: Task = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, 42);
        assert_eq!(back.title, "Test task");
        assert_eq!(back.stage, Stage::Planning);
        assert_eq!(back.tool, Some(Tool::Claude));
        assert_eq!(back.model, Some(Model::Claude3Opus));
        assert!(!back.done);
    }

    #[test]
    fn model_mapping() {
        assert_eq!(
            Model::Gpt4o.model_name_for_tool(Tool::Copilot),
            Some("gpt-4o")
        );
        assert_eq!(
            Model::ClaudeSonnet4_5.model_name_for_tool(Tool::Copilot),
            Some("claude-sonnet-4.5")
        );
        assert_eq!(
            Model::Claude35Sonnet.model_name_for_tool(Tool::Claude),
            Some("sonnet")
        );
        assert_eq!(Model::Gpt5_2.model_name_for_tool(Tool::Claude), None);
    }

    #[test]
    fn model_parsing() {
        assert_eq!("gpt-5.2".parse::<Model>().unwrap(), Model::Gpt5_2);
        assert_eq!("GPT-5.2".parse::<Model>().unwrap(), Model::Gpt5_2);
        assert_eq!("gpt-5-2".parse::<Model>().unwrap(), Model::Gpt5_2);
        assert_eq!(
            "claude-sonnet-4.5".parse::<Model>().unwrap(),
            Model::ClaudeSonnet4_5
        );
        assert!("invalid-model".parse::<Model>().is_err());
    }
}
