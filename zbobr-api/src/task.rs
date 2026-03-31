// -- FixedOffsetTz --

/// Transparent wrapper around `chrono::FixedOffset` with serde/clap support.
/// Accepts `+HHMM` or `+HH:MM` format (e.g. `"+0300"`, `"-05:00"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedOffsetTz(pub chrono::FixedOffset);

impl std::ops::Deref for FixedOffsetTz {
    type Target = chrono::FixedOffset;
    fn deref(&self) -> &chrono::FixedOffset {
        &self.0
    }
}

impl std::fmt::Display for FixedOffsetTz {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for FixedOffsetTz {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.is_empty() {
            return Err("empty timezone string".to_string());
        }
        let sign: i32 = match s.as_bytes()[0] {
            b'+' => 1,
            b'-' => -1,
            _ => return Err(format!("timezone must start with '+' or '-': {s}")),
        };
        let digits = &s[1..];
        let (hours, minutes) = if digits.contains(':') {
            let parts: Vec<&str> = digits.splitn(2, ':').collect();
            (
                parts[0].parse::<i32>().map_err(|e| e.to_string())?,
                parts[1].parse::<i32>().map_err(|e| e.to_string())?,
            )
        } else if digits.len() == 4 {
            (
                digits[..2].parse::<i32>().map_err(|e| e.to_string())?,
                digits[2..].parse::<i32>().map_err(|e| e.to_string())?,
            )
        } else {
            return Err(format!("expected +HHMM or +HH:MM: {s}"));
        };
        let total_seconds = sign * (hours * 3600 + minutes * 60);
        chrono::FixedOffset::east_opt(total_seconds)
            .map(FixedOffsetTz)
            .ok_or_else(|| format!("timezone offset out of range: {s}"))
    }
}

impl serde::Serialize for FixedOffsetTz {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> serde::Deserialize<'de> for FixedOffsetTz {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

// -- TaskIdentity --

/// Bundles task routing info for worktree operations.
/// Only constructible when the work_branch field is set.
/// Repository and branch are now configured in the repo backend.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TaskIdentity {
    pub task_id: u64,
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

// -- Context types --

/// Type of a context record within a stage.
#[derive(
    Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ContextRecordType {
    /// A checkbox item with checked/unchecked state.
    Checkbox(bool),
    /// A success report.
    Success,
    /// A failure report.
    Failure,
    /// A comment.
    Comment,
    /// A question requiring human input.
    Question,
}

impl std::fmt::Display for ContextRecordType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContextRecordType::Checkbox(true) => write!(f, "checkbox(checked)"),
            ContextRecordType::Checkbox(false) => write!(f, "checkbox(unchecked)"),
            ContextRecordType::Success => write!(f, "success"),
            ContextRecordType::Failure => write!(f, "failure"),
            ContextRecordType::Comment => write!(f, "comment"),
            ContextRecordType::Question => write!(f, "question"),
        }
    }
}

/// A single record within a stage context.
#[derive(
    Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize, schemars::JsonSchema,
)]
pub struct ContextRecord {
    /// Unique numeric id, generated as max records id + 1.
    pub id: u64,
    /// Type of this context record.
    pub record_type: ContextRecordType,
    /// Brief description of this record.
    pub brief: String,
    /// Optional link to a long description or report.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report_link: Option<String>,
}

/// Metadata about a stage execution.
#[derive(
    Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize, schemars::JsonSchema,
)]
pub struct StageInfo {
    /// Zbobr instance name that produced this stage.
    pub instance: String,
    /// Pipeline that owns this stage.
    pub pipeline: Pipeline,
    /// Run identifier within the pipeline (monotonically increasing per pipeline).
    pub run_id: u64,
    /// Stage name within the pipeline.
    pub stage: Stage,
    /// Tool used for execution (e.g. copilot, claude).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool: Option<Tool>,
    /// Model used by the tool.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<Model>,
    /// Link to the prompt used for this stage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_link: Option<String>,
    /// Link to the captured output of this stage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_link: Option<String>,
    /// Timestamp when the stage was created.
    #[schemars(with = "String")]
    pub timestamp: chrono::DateTime<chrono::FixedOffset>,
}

/// Context for a single stage execution, containing stage metadata and records.
#[derive(
    Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize, schemars::JsonSchema,
)]
pub struct StageContext {
    /// Stage execution metadata.
    pub info: StageInfo,
    /// Context records produced during this stage.
    #[serde(default)]
    pub records: Vec<ContextRecord>,
}

/// Full task context containing all stage contexts.
#[derive(
    Debug, Clone, PartialEq, Eq, Default, serde::Deserialize, serde::Serialize, schemars::JsonSchema,
)]
pub struct TaskContext {
    /// Ordered list of stage contexts.
    #[serde(default)]
    pub stages: Vec<StageContext>,
}

impl TaskContext {
    /// Returns the next available record id (max existing id + 1).
    pub fn next_id(&self) -> u64 {
        self.stages
            .iter()
            .flat_map(|s| s.records.iter())
            .map(|r| r.id)
            .max()
            .map(|max| max + 1)
            .unwrap_or(1)
    }

    /// Find a context record by its numeric id across all stages.
    /// Returns a reference to the record and its stage index.
    pub fn find_record(&self, id: u64) -> Option<(usize, &ContextRecord)> {
        for (stage_idx, stage) in self.stages.iter().enumerate() {
            if let Some(record) = stage.records.iter().find(|r| r.id == id) {
                return Some((stage_idx, record));
            }
        }
        None
    }

    /// Find a mutable reference to a context record by its numeric id.
    /// Returns a mutable reference to the record and its stage index.
    pub fn find_record_mut(&mut self, id: u64) -> Option<(usize, &mut ContextRecord)> {
        for (stage_idx, stage) in self.stages.iter_mut().enumerate() {
            if let Some(record) = stage.records.iter_mut().find(|r| r.id == id) {
                return Some((stage_idx, record));
            }
        }
        None
    }

    /// Delete a context record by its numeric id.
    /// Returns true if the record was found and deleted.
    pub fn delete_record(&mut self, id: u64) -> bool {
        for stage in &mut self.stages {
            let len_before = stage.records.len();
            stage.records.retain(|r| r.id != id);
            if stage.records.len() < len_before {
                return true;
            }
        }
        false
    }
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
    #[schemars(description = "Timestamp when comment was created", with = "String")]
    pub timestamp: chrono::DateTime<chrono::FixedOffset>,
    #[schemars(description = "Username of the user posting the comment")]
    pub username: String,
    #[schemars(description = "Comment body text")]
    pub body: String,
    #[schemars(description = "URL of this comment (e.g. GitHub comment HTML URL)")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

// -- History helper types --

/// Type of a history record, derived from `[tool_name]` prefix in comment text.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum HistoryRecordType {
    Task,
    Success,
    Failure,
    Progress,
    Question,
    Other,
}

/// Determine the record type from a comment's `[tool_name]` prefix.
pub fn classify_comment(text: &str) -> HistoryRecordType {
    let prefix = text.split('\n').next().unwrap_or("");
    match prefix {
        "[report_results]" | "[report_success]" | "[post_plan]" | "[review_accept]"
        | "[test_accept]" => HistoryRecordType::Success,
        "[report_failure]" | "[ask_planner]" | "[review_reject]" | "[test_reject]" => {
            HistoryRecordType::Failure
        }
        "[report_intermediate]" => HistoryRecordType::Progress,
        "[ask_user]" | "[stop_with_question]" => HistoryRecordType::Question,
        _ => HistoryRecordType::Other,
    }
}

/// A stage name within a pipeline (user-defined, dynamically configured).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Stage(pub String);

impl Stage {
    pub fn new(s: impl Into<String>) -> Self {
        Stage(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Stage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::ops::Deref for Stage {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl std::borrow::Borrow<str> for Stage {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl From<&str> for Stage {
    fn from(s: &str) -> Self {
        Stage(s.to_string())
    }
}

impl From<String> for Stage {
    fn from(s: String) -> Self {
        Stage(s)
    }
}

impl serde::Serialize for Stage {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> serde::Deserialize<'de> for Stage {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Stage(String::deserialize(deserializer)?))
    }
}

impl schemars::JsonSchema for Stage {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Stage".into()
    }
    fn json_schema(_gen: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({ "type": "string" })
    }
}

/// Task lifecycle state.
///
/// Serialized as a structured string:
/// - `""` (empty)
/// - `"done"`
/// - `"pause"`
/// - `"ready"`
/// - `"pending:{pipeline}"`
/// - `"running:{pipeline}:{stage}"`
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum State {
    #[default]
    Empty,
    Done,
    Pause,
    Ready,
    Pending(Pipeline),
    Running(Pipeline, Stage),
    /// Unrecognized state preserved verbatim to keep compatibility with
    /// external/manual values.
    Unknown(String),
}

impl State {
    pub fn pending(pipeline: impl Into<Pipeline>) -> Self {
        State::Pending(pipeline.into())
    }

    pub fn running(pipeline: impl Into<Pipeline>, stage: impl Into<Stage>) -> Self {
        State::Running(pipeline.into(), stage.into())
    }

    pub fn is_empty(&self) -> bool {
        matches!(self, State::Empty)
    }

    pub fn is_done(&self) -> bool {
        matches!(self, State::Done)
    }

    pub fn is_pause(&self) -> bool {
        matches!(self, State::Pause)
    }

    pub fn is_ready(&self) -> bool {
        matches!(self, State::Ready)
    }

    pub fn is_pending(&self) -> bool {
        matches!(self, State::Pending(_))
    }

    pub fn is_running(&self) -> bool {
        matches!(self, State::Running(_, _))
    }

    pub fn pipeline(&self) -> Option<&Pipeline> {
        match self {
            State::Pending(p) | State::Running(p, _) => Some(p),
            _ => None,
        }
    }

    pub fn stage(&self) -> Option<&Stage> {
        match self {
            State::Running(_, s) => Some(s),
            _ => None,
        }
    }
}

impl State {
    /// Serialize to the canonical string format used for serde.
    fn to_serde_string(&self) -> String {
        match self {
            State::Empty => String::new(),
            State::Done => "done".to_string(),
            State::Pause => "pause".to_string(),
            State::Ready => "ready".to_string(),
            State::Pending(pipeline) => format!("pending:{}", pipeline.as_str()),
            State::Running(pipeline, stage) => {
                format!("running:{}:{}", pipeline.as_str(), stage.as_str())
            }
            State::Unknown(raw) => raw.clone(),
        }
    }
}

impl From<&str> for State {
    fn from(s: &str) -> Self {
        if s.is_empty() {
            return State::Empty;
        }

        // Parse "variant" or "variant:arg1" or "variant:arg1:arg2"
        let mut parts = s.splitn(3, ':');
        match parts.next().unwrap() {
            "done" => State::Done,
            "pause" => State::Pause,
            "ready" => State::Ready,
            "pending" => match parts.next() {
                Some(p) if !p.is_empty() => State::Pending(Pipeline::from(p)),
                _ => State::Unknown(s.to_string()),
            },
            "running" => match (parts.next(), parts.next()) {
                (Some(p), Some(st)) if !p.is_empty() && !st.is_empty() => {
                    State::Running(Pipeline::from(p), Stage::from(st))
                }
                _ => State::Unknown(s.to_string()),
            },
            _ => State::Unknown(s.to_string()),
        }
    }
}

impl From<String> for State {
    fn from(s: String) -> Self {
        State::from(s.as_str())
    }
}

impl std::str::FromStr for State {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(State::from(s))
    }
}

impl serde::Serialize for State {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_serde_string())
    }
}

impl<'de> serde::Deserialize<'de> for State {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(State::from(s))
    }
}

impl schemars::JsonSchema for State {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "State".into()
    }
    fn json_schema(_gen: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({ "type": "string" })
    }
}

/// A pipeline identifier: one of the three built-in pipelines or a custom one.
#[derive(Debug, Clone)]
pub enum Pipeline {
    /// The primary workflow pipeline (name: `"main"`).
    Main,
    /// The merge/conflict-resolution pipeline (name: `"merge"`).
    Merge,
    /// Any other user-defined pipeline.
    Custom(String),
}

impl Pipeline {
    pub const MAIN: &'static str = "main";
    pub const MERGE: &'static str = "merge";

    pub fn as_str(&self) -> &str {
        match self {
            Pipeline::Main => Self::MAIN,
            Pipeline::Merge => Self::MERGE,
            Pipeline::Custom(s) => s.as_str(),
        }
    }
}

impl std::fmt::Display for Pipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for Pipeline {
    type Err = std::convert::Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            Self::MAIN => Pipeline::Main,
            Self::MERGE => Pipeline::Merge,
            other => Pipeline::Custom(other.to_string()),
        })
    }
}

impl std::borrow::Borrow<str> for Pipeline {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl PartialEq for Pipeline {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for Pipeline {}

impl std::hash::Hash for Pipeline {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(self.as_str(), state);
    }
}

impl PartialOrd for Pipeline {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Pipeline {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl From<&str> for Pipeline {
    fn from(s: &str) -> Self {
        s.parse().unwrap()
    }
}

impl From<String> for Pipeline {
    fn from(s: String) -> Self {
        s.parse().unwrap()
    }
}

impl serde::Serialize for Pipeline {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for Pipeline {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(s.parse().unwrap())
    }
}

impl schemars::JsonSchema for Pipeline {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Pipeline".into()
    }
    fn json_schema(_gen: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({ "type": "string" })
    }
}

/// Flow-control signal for the task state machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Signal {
    /// Navigate to a specific stage within the current pipeline.
    Go(Stage),
    /// Call a sub-pipeline.
    Call(Pipeline),
    /// Return successfully from a sub-pipeline.
    Return,
    /// Return with failure from a sub-pipeline.
    ReturnFailure,
}

impl Signal {
    pub fn go(stage: impl Into<Stage>) -> Self {
        Signal::Go(stage.into())
    }

    pub fn call(pipeline: impl Into<Pipeline>) -> Self {
        Signal::Call(pipeline.into())
    }

    pub fn go_target(&self) -> Option<&Stage> {
        match self {
            Signal::Go(s) => Some(s),
            _ => None,
        }
    }

    pub fn call_target(&self) -> Option<&Pipeline> {
        match self {
            Signal::Call(p) => Some(p),
            _ => None,
        }
    }
}

impl std::fmt::Display for Signal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Signal::Go(stage) => write!(f, "go_{stage}"),
            Signal::Call(pipeline) => write!(f, "call_{pipeline}"),
            Signal::Return => f.write_str("return"),
            Signal::ReturnFailure => f.write_str("return_failure"),
        }
    }
}

impl std::str::FromStr for Signal {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(stage) = s.strip_prefix("go_") {
            Ok(Signal::Go(Stage::new(stage)))
        } else if let Some(pipeline) = s.strip_prefix("call_") {
            Ok(Signal::Call(Pipeline::from(pipeline)))
        } else if s == "return" {
            Ok(Signal::Return)
        } else if s == "return_failure" {
            Ok(Signal::ReturnFailure)
        } else {
            Err(anyhow::anyhow!("Unknown signal: {}", s))
        }
    }
}

impl serde::Serialize for Signal {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> serde::Deserialize<'de> for Signal {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl schemars::JsonSchema for Signal {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Signal".into()
    }
    fn json_schema(_gen: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({ "type": "string" })
    }
}

/// An entry on the task's call stack, recording which pipeline to return to
/// and which signal to emit upon return (e.g. `Signal::Go("working")`).
#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct StackEntry {
    /// Caller's pipeline to return to after sub-pipeline completion.
    pub pipeline: Pipeline,
    /// Caller's pipeline_run_id to restore on return.
    #[serde(default)]
    pub pipeline_run_id: u64,
    /// Signal to emit when returning to this pipeline.
    #[serde(alias = "stage")]
    pub signal: Signal,
}

/// A worktree problem detected before stage execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorktreeProblem {
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
    #[serde(rename = "gpt-5.4")]
    Gpt5_4,
    #[serde(rename = "gpt-5.3-codex")]
    Gpt5_3Codex,
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
    #[serde(rename = "claude-sonnet-4.6")]
    ClaudeSonnet4_6,
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
            Model::Gpt5_4,
            Model::Gpt5_3Codex,
            Model::Gpt4_1,
            Model::ClaudeSonnet4,
            Model::ClaudeHaiku4_5,
            Model::ClaudeOpus4_5,
            Model::ClaudeSonnet4_5,
            Model::ClaudeSonnet4_6,
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
                Model::Gpt5_4 => Some("gpt-5.4"),
                Model::Gpt5_3Codex => Some("gpt-5.3-codex"),
                Model::Gpt4_1 => Some("gpt-4.1"),
                Model::ClaudeSonnet4 => Some("claude-sonnet-4"),
                Model::ClaudeHaiku4_5 => Some("claude-haiku-4.5"),
                Model::ClaudeOpus4_5 => Some("claude-opus-4.5"),
                Model::ClaudeSonnet4_5 => Some("claude-sonnet-4.5"),
                Model::ClaudeSonnet4_6 => Some("claude-sonnet-4.6"),
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
                Model::ClaudeSonnet4_6 => Some("claude-sonnet-4-6"),
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
            Model::Gpt5_4 => "gpt-5.4",
            Model::Gpt5_3Codex => "gpt-5.3-codex",
            Model::Gpt4_1 => "gpt-4.1",
            Model::ClaudeSonnet4 => "claude-sonnet-4",
            Model::ClaudeHaiku4_5 => "claude-haiku-4.5",
            Model::ClaudeOpus4_5 => "claude-opus-4.5",
            Model::ClaudeSonnet4_5 => "claude-sonnet-4.5",
            Model::ClaudeSonnet4_6 => "claude-sonnet-4.6",
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
            "gpt-5-4" => Ok(Model::Gpt5_4),
            "gpt-5-3-codex" => Ok(Model::Gpt5_3Codex),
            "gpt-4-1" => Ok(Model::Gpt4_1),
            "claude-sonnet-4" => Ok(Model::ClaudeSonnet4),
            "claude-haiku-4-5" => Ok(Model::ClaudeHaiku4_5),
            "claude-opus-4-5" => Ok(Model::ClaudeOpus4_5),
            "claude-sonnet-4-5" => Ok(Model::ClaudeSonnet4_5),
            "claude-sonnet-4-6" => Ok(Model::ClaudeSonnet4_6),
            "claude-opus-4-6" => Ok(Model::ClaudeOpus4_6),
            "claude-opus-4-6-fast" => Ok(Model::ClaudeOpus4_6Fast),
            "gemini-3-pro-preview" => Ok(Model::Gemini3ProPreview),
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
    /// Task state.
    pub state: State,
    pub destination_repository: Option<String>,
    pub destination_branch: Option<String>,
    pub work_branch: Option<String>,
    pub pr_url: Option<String>,
    /// Structured task context containing stage execution history and records.
    #[serde(default)]
    pub context: TaskContext,
    /// Signal for flow control: go_{stage}, call_{pipeline}, return
    pub signal: Option<Signal>,
    /// Call stack for pipeline call/return semantics.
    #[serde(default)]
    pub stack: Vec<StackEntry>,
    /// Status message stored in the STATUS section of the task body.
    /// Contains the last error or question with icon, timestamp, and message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    pub pause: bool,
    /// When true the dispatcher will automatically set the pause flag any time
    /// the task's state is changed.  This gives human operators an opportunity to
    /// review a transition before the next processing step occurs.
    pub confirm: bool,
    /// Current/latest pipeline run counter. Incremented on each new pipeline call.
    #[serde(default)]
    pub pipeline_run_id: u64,
    /// Counter that is automatically incremented on each stage passed.
    #[serde(default)]
    pub stage_count: u64,
    /// Maximum number of stages this task is allowed to pass before being auto-paused.
    /// 0 means no limit. Can be set individually per task (overrides the global default).
    #[serde(default)]
    pub max_stage_count: u64,
    /// Whether the task is closed (e.g. GitHub issue closed, fs task marked closed).
    /// Closed tasks are not listed by `list_tasks` and their worktrees can be cleaned up.
    #[serde(default)]
    pub closed: bool,
    /// ETag for optimistic locking to prevent concurrent update conflicts.
    /// Used to detect if the task has been modified between read and write operations.
    #[serde(skip)]
    pub etag: Option<String>,
}

impl Task {
    /// Returns a TaskIdentity if the work_branch field is set.
    pub fn identity(&self) -> Option<TaskIdentity> {
        Some(TaskIdentity {
            task_id: self.id,
            work_branch: self.work_branch.clone()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_serde_format() {
        assert_eq!(State::Empty.to_serde_string(), "");
        assert_eq!(State::Done.to_serde_string(), "done");
        assert_eq!(State::Pause.to_serde_string(), "pause");
        assert_eq!(State::Ready.to_serde_string(), "ready");
        assert_eq!(
            State::Pending(Pipeline::Main).to_serde_string(),
            "pending:main"
        );
        assert_eq!(
            State::Pending(Pipeline::Custom("foo".into())).to_serde_string(),
            "pending:foo"
        );
        assert_eq!(
            State::Running(Pipeline::Main, Stage::from("working")).to_serde_string(),
            "running:main:working"
        );
        assert_eq!(State::Unknown("custom".into()).to_serde_string(), "custom");
    }

    #[test]
    fn state_parse() {
        assert_eq!(State::from(""), State::Empty);
        assert_eq!(State::from("done"), State::Done);
        assert_eq!(State::from("pause"), State::Pause);
        assert_eq!(State::from("ready"), State::Ready);
        assert_eq!(State::from("pending:main"), State::Pending(Pipeline::Main));
        assert_eq!(
            State::from("pending:foo"),
            State::Pending(Pipeline::Custom("foo".into()))
        );
        assert_eq!(
            State::from("running:main:working"),
            State::Running(Pipeline::Main, Stage::from("working"))
        );
    }

    #[test]
    fn state_parse_incomplete_is_unknown() {
        // pending without pipeline
        assert_eq!(State::from("pending"), State::Unknown("pending".into()));
        assert_eq!(State::from("pending:"), State::Unknown("pending:".into()));
        // running without stage
        assert_eq!(
            State::from("running:main"),
            State::Unknown("running:main".into())
        );
        // running without pipeline or stage
        assert_eq!(State::from("running"), State::Unknown("running".into()));
    }

    #[test]
    fn state_roundtrip_serde_parse() {
        let states = vec![
            State::Empty,
            State::Done,
            State::Pause,
            State::Ready,
            State::Pending(Pipeline::Main),
            State::Pending(Pipeline::Merge),
            State::Pending(Pipeline::Custom("custom".into())),
            State::Running(Pipeline::Main, Stage::from("working")),
            State::Running(Pipeline::Merge, Stage::from("reviewing")),
        ];
        for state in states {
            let s = state.to_serde_string();
            let parsed = State::from(s.as_str());
            assert_eq!(parsed, state, "roundtrip failed for {s:?}");
        }
    }

    #[test]
    fn state_is_pending() {
        assert!(State::Pending(Pipeline::Main).is_pending());
        assert!(State::Pending(Pipeline::Custom("x".into())).is_pending());
        assert!(!State::Done.is_pending());
        assert!(!State::Running(Pipeline::Main, Stage::from("s")).is_pending());
    }

    #[test]
    fn state_is_running() {
        assert!(State::Running(Pipeline::Main, Stage::from("s")).is_running());
        assert!(!State::Pending(Pipeline::Main).is_running());
        assert!(!State::Done.is_running());
    }

    fn make_stage_info(pipeline: &str, stage: &str) -> StageInfo {
        StageInfo {
            instance: "default".to_string(),
            pipeline: Pipeline::from(pipeline),
            run_id: 1,
            stage: Stage::from(stage),
            tool: None,
            model: None,
            prompt_link: None,
            output_link: None,
            timestamp: "2026-03-25T00:00:00Z".parse().unwrap(),
        }
    }

    fn make_record(id: u64, record_type: ContextRecordType, brief: &str) -> ContextRecord {
        ContextRecord {
            id,
            record_type,
            brief: brief.to_string(),
            report_link: None,
        }
    }

    #[test]
    fn task_context_default_is_empty() {
        let ctx = TaskContext::default();
        assert!(ctx.stages.is_empty());
        assert_eq!(ctx.next_id(), 1);
    }

    #[test]
    fn task_context_next_id() {
        let mut ctx = TaskContext::default();
        assert_eq!(ctx.next_id(), 1);

        ctx.stages.push(StageContext {
            info: make_stage_info("main", "working"),
            records: vec![
                make_record(1, ContextRecordType::Checkbox(false), "first"),
                make_record(3, ContextRecordType::Success, "done"),
            ],
        });
        assert_eq!(ctx.next_id(), 4);

        ctx.stages.push(StageContext {
            info: make_stage_info("main", "review"),
            records: vec![make_record(5, ContextRecordType::Failure, "fail")],
        });
        assert_eq!(ctx.next_id(), 6);
    }

    #[test]
    fn task_context_find_record() {
        let ctx = TaskContext {
            stages: vec![
                StageContext {
                    info: make_stage_info("main", "working"),
                    records: vec![
                        make_record(1, ContextRecordType::Checkbox(false), "a"),
                        make_record(2, ContextRecordType::Success, "b"),
                    ],
                },
                StageContext {
                    info: make_stage_info("main", "review"),
                    records: vec![make_record(3, ContextRecordType::Question, "c")],
                },
            ],
        };

        let (stage_idx, record) = ctx.find_record(2).unwrap();
        assert_eq!(stage_idx, 0);
        assert_eq!(record.brief, "b");

        let (stage_idx, record) = ctx.find_record(3).unwrap();
        assert_eq!(stage_idx, 1);
        assert_eq!(record.brief, "c");

        assert!(ctx.find_record(99).is_none());
    }

    #[test]
    fn task_context_find_record_mut() {
        let mut ctx = TaskContext {
            stages: vec![StageContext {
                info: make_stage_info("main", "working"),
                records: vec![make_record(1, ContextRecordType::Checkbox(false), "item")],
            }],
        };

        let (_, record) = ctx.find_record_mut(1).unwrap();
        record.record_type = ContextRecordType::Checkbox(true);

        let (_, record) = ctx.find_record(1).unwrap();
        assert_eq!(record.record_type, ContextRecordType::Checkbox(true));
    }

    #[test]
    fn task_context_delete_record() {
        let mut ctx = TaskContext {
            stages: vec![StageContext {
                info: make_stage_info("main", "working"),
                records: vec![
                    make_record(1, ContextRecordType::Checkbox(false), "a"),
                    make_record(2, ContextRecordType::Success, "b"),
                ],
            }],
        };

        assert!(ctx.delete_record(1));
        assert_eq!(ctx.stages[0].records.len(), 1);
        assert_eq!(ctx.stages[0].records[0].id, 2);

        assert!(!ctx.delete_record(99));
        assert_eq!(ctx.stages[0].records.len(), 1);
    }

    #[test]
    fn context_record_type_display() {
        assert_eq!(
            ContextRecordType::Checkbox(true).to_string(),
            "checkbox(checked)"
        );
        assert_eq!(
            ContextRecordType::Checkbox(false).to_string(),
            "checkbox(unchecked)"
        );
        assert_eq!(ContextRecordType::Success.to_string(), "success");
        assert_eq!(ContextRecordType::Failure.to_string(), "failure");
        assert_eq!(ContextRecordType::Comment.to_string(), "comment");
        assert_eq!(ContextRecordType::Question.to_string(), "question");
    }

    fn make_task(id: u64, work_branch: Option<&str>) -> Task {
        Task {
            id,
            title: String::new(),
            description: String::new(),
            state: State::Empty,
            destination_repository: None,
            destination_branch: None,
            work_branch: work_branch.map(|s| s.to_string()),
            pr_url: None,
            context: TaskContext::default(),
            signal: None,
            stack: Vec::new(),
            status: None,
            pause: false,
            confirm: false,
            pipeline_run_id: 0,
            stage_count: 0,
            max_stage_count: 0,
            closed: false,
            etag: None,
        }
    }

    #[test]
    fn identity_returns_some_when_work_branch_set() {
        let task = make_task(42, Some("zbobr_fix-123-my-feature"));
        let identity = task.identity().expect("identity should be Some");
        assert_eq!(identity.task_id, 42);
        assert_eq!(identity.work_branch, "zbobr_fix-123-my-feature");
    }

    #[test]
    fn identity_returns_none_when_work_branch_missing() {
        let task = make_task(42, None);
        assert!(task.identity().is_none());
    }
}
