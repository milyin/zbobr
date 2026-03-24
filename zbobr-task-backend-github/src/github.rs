use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use zbobr_api::{
    Comment, CommentTag, Model, Task, Tool,
    backend::TaskBackend,
    comment_tag,
    task::{Pipeline, StackEntry, Stage, State},
};

// -- Label prefix constants (GitHub-backend-specific) --

const STATE_PREFIX: &str = "state:";
const PIPELINE_PREFIX: &str = "pipeline:";
const STAGE_PREFIX: &str = "stage:";
const SIGNAL_PREFIX: &str = "signal:";
const FLAG_PREFIX: &str = "flag:";

use crate::{
    config::ZbobrTaskBackendGithubConfig,
    separator::{
        merge_concurrent_description_updates, parse_description_full, serialize_description_full,
    },
};

/// Format an octocrab error as a concise human-readable string without snafu backtraces.
///
/// Using `{e}` directly in log macros triggers snafu's Display which includes a full
/// `Backtrace` (containing `std::panicking` frames from the Rust runtime), making logs
/// look like panics. This function extracts just the meaningful message.
fn format_octocrab_error(e: &octocrab::Error) -> String {
    match e {
        octocrab::Error::GitHub { source, .. } => {
            format!("HTTP {} - {}", source.status_code, source.message)
        }
        octocrab::Error::Serde { source, .. } => {
            // A Serde error here typically means GitHub returned a non-JSON body
            // (e.g. an HTML error page from a 502 Bad Gateway).
            format!(
                "GitHub returned a non-JSON response (server may be returning an error page): {source}"
            )
        }
        other => {
            // For other variants, walk the std::error::Error source chain to get the
            // underlying message without the snafu wrapper's backtrace in Display.
            use std::error::Error;
            other
                .source()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "unknown GitHub client error".to_string())
        }
    }
}

/// Convert an octocrab error into an anyhow::Error with detailed information.
fn octocrab_to_anyhow(e: octocrab::Error) -> anyhow::Error {
    anyhow::anyhow!("GitHub API error: {}", format_octocrab_error(&e))
}

fn is_transient_octocrab_error(error: &octocrab::Error) -> bool {
    match error {
        octocrab::Error::GitHub { source, .. } => source.status_code.is_server_error(),
        _ => true,
    }
}

/// Retry a GitHub API operation up to 3 times on transient errors.
async fn retry_github<T, F, Fut>(op_name: &str, mut f: F) -> anyhow::Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, octocrab::Error>>,
{
    let mut attempt = 0u64;
    loop {
        attempt += 1;
        match f().await {
            Ok(value) => return Ok(value),
            Err(e) => {
                if attempt < 3 && is_transient_octocrab_error(&e) {
                    tracing::warn!(
                        "Transient GitHub error during {op_name} (attempt {attempt}/3): {}",
                        format_octocrab_error(&e)
                    );
                    tokio::time::sleep(Duration::from_millis(250 * attempt)).await;
                    continue;
                }
                return Err(octocrab_to_anyhow(e));
            }
        }
    }
}

// Tag parsing is handled by `CommentTag::from_str` and associated helpers
// in the tests; the old `parse_comment_tag` helper has been removed.
// -- Shared response types --

#[derive(Debug, serde::Deserialize)]
struct IssueResponse {
    number: u64,
    title: String,
    body: Option<String>,
    #[allow(dead_code)]
    state: String,
    labels: Vec<IssueLabel>,
}

#[derive(Debug, serde::Deserialize)]
struct IssueLabel {
    name: String,
}

#[derive(Debug, serde::Deserialize)]
struct CommentResponse {
    body: Option<String>,
    created_at: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
struct RepoResponse {
    full_name: String,
}

#[derive(Debug, serde::Deserialize)]
struct ContentResponse {
    content: Option<String>,
}

/// Extract report filename from a trailing markdown link in comment body.
///
/// Looks for a last line matching `[{filename}]({url})` where the url contains
/// `/reports/task_`. Returns (clean_body, Some(filename)) or (original, None).
fn extract_report_link(text: &str) -> (String, Option<String>) {
    let trimmed = text.trim_end();
    if let Some(last_newline) = trimmed.rfind('\n') {
        let last_line = trimmed[last_newline + 1..].trim();
        if let Some(report_name) = parse_report_link_line(last_line) {
            let clean = trimmed[..last_newline].trim_end().to_string();
            return (clean, Some(report_name));
        }
    } else if let Some(report_name) = parse_report_link_line(trimmed) {
        return (String::new(), Some(report_name));
    }
    (text.to_string(), None)
}

/// Parse a single line as a report link: `[{filename}]({url containing /reports/task_})`.
fn parse_report_link_line(line: &str) -> Option<String> {
    let line = line.trim();
    if !line.starts_with('[') {
        return None;
    }
    let close_bracket = line.find(']')?;
    let filename = &line[1..close_bracket];
    let rest = &line[close_bracket + 1..];
    if !rest.starts_with('(') || !rest.ends_with(')') {
        return None;
    }
    let url = &rest[1..rest.len() - 1];
    if url.contains("/reports/task_") {
        Some(filename.to_string())
    } else {
        None
    }
}

/// Format a clickable markdown link to a report file in the GitHub repo.
fn format_report_link(
    owner: &str,
    repo: &str,
    branch: &str,
    reports_path: &str,
    task_id: u64,
    filename: &str,
) -> String {
    format!(
        "\n\n[{filename}](https://github.com/{owner}/{repo}/blob/{branch}/{reports_path}/task_{task_id}/{filename})"
    )
}

// ============================================================================
// GitHubTaskBackend
// ============================================================================

/// Duration to wait after writing to a GitHub issue before allowing reads.
/// This handles GitHub API eventual consistency for list/filter queries.
const COOLING_DURATION: Duration = Duration::from_secs(3);

pub struct ZbobrTaskBackendGithubImpl {
    backend_config: ZbobrTaskBackendGithubConfig,
    octocrab: octocrab::Octocrab,
    cooling_deadlines: Mutex<HashMap<u64, tokio::time::Instant>>,
    /// Per-task mutexes to serialize concurrent read-modify-write cycles
    /// for the same task within this process.
    task_locks: std::sync::Mutex<HashMap<u64, Arc<tokio::sync::Mutex<()>>>>,
}

impl ZbobrTaskBackendGithubImpl {
    pub fn from_config(backend_config: ZbobrTaskBackendGithubConfig) -> anyhow::Result<Self> {
        backend_config.validate()?;
        let octocrab = octocrab::Octocrab::builder()
            .personal_token(backend_config.github_token.clone())
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build octocrab client: {e}"))?;
        Ok(Self {
            backend_config,
            octocrab,
            cooling_deadlines: Mutex::new(HashMap::new()),
            task_locks: std::sync::Mutex::new(HashMap::new()),
        })
    }

    /// Convert a Signal to its GitHub label representation.
    fn signal_to_label(signal: &zbobr_api::Signal) -> String {
        format!("{SIGNAL_PREFIX}{signal}")
    }

    /// Parse a GitHub label string back to a Signal.
    fn label_to_signal(label: &str) -> Option<zbobr_api::Signal> {
        label.strip_prefix(SIGNAL_PREFIX)?.parse().ok()
    }

    /// Convert a state to its GitHub label representations.
    fn state_to_labels(state: &State) -> Vec<String> {
        let state_label = |name: &str| format!("{}{name}", STATE_PREFIX);
        let pipeline_label = |p: &Pipeline| format!("{}{}", PIPELINE_PREFIX, p.as_str());
        let stage_label = |s: &Stage| format!("{}{}", STAGE_PREFIX, s.as_str());

        match state {
            State::Empty => vec![],
            State::Done => vec![state_label(State::LABEL_DONE)],
            State::Pause => vec![state_label(State::LABEL_PAUSE)],
            State::Ready => vec![state_label(State::LABEL_READY)],
            State::Pending(pipeline) => vec![
                state_label(State::LABEL_PENDING),
                pipeline_label(pipeline),
            ],
            State::Running(pipeline, stage) => vec![
                state_label(State::LABEL_RUNNING),
                pipeline_label(pipeline),
                stage_label(stage),
            ],
            State::Unknown(raw) => vec![state_label(raw)],
        }
    }

    /// Parse a State from GitHub issue labels.
    fn labels_to_state(labels: &[IssueLabel]) -> State {
        let mut state_value: Option<&str> = None;
        let mut pipeline_value: Option<&str> = None;
        let mut stage_value: Option<&str> = None;

        for label in labels {
            if let Some(v) = label.name.strip_prefix(STATE_PREFIX) {
                state_value = Some(v);
            } else if let Some(v) = label.name.strip_prefix(PIPELINE_PREFIX) {
                pipeline_value = Some(v);
            } else if let Some(v) = label.name.strip_prefix(STAGE_PREFIX) {
                stage_value = Some(v);
            }
        }

        match state_value {
            None => State::Empty,
            Some(v) if v == State::LABEL_DONE => State::Done,
            Some(v) if v == State::LABEL_PAUSE => State::Pause,
            Some(v) if v == State::LABEL_READY => State::Ready,
            Some(v) if v == State::LABEL_PENDING => match pipeline_value {
                Some(p) => State::Pending(Pipeline::from(p)),
                None => State::Unknown(format!("{}{}", STATE_PREFIX, State::LABEL_PENDING)),
            },
            Some(v) if v == State::LABEL_RUNNING => match (pipeline_value, stage_value) {
                (Some(p), Some(s)) => {
                    State::Running(Pipeline::from(p), Stage::from(s))
                }
                (None, Some(s)) => {
                    State::Unknown(format!("{}{}, {}{s}", STATE_PREFIX, State::LABEL_RUNNING, STAGE_PREFIX))
                }
                (Some(p), None) => {
                    State::Unknown(format!("{}{}, {}{p}", STATE_PREFIX, State::LABEL_RUNNING, PIPELINE_PREFIX))
                }
                (None, None) => State::Unknown(format!("{}{}", STATE_PREFIX, State::LABEL_RUNNING)),
            },
            Some(other) => State::Unknown(format!("{}{other}", STATE_PREFIX)),
        }
    }

    /// Return the GitHub label color for a state-related label.
    fn state_label_color(label: &str) -> &'static str {
        if let Some(state_name) = label.strip_prefix(STATE_PREFIX) {
            match state_name {
                v if v == State::LABEL_DONE => "0e8a16",    // green
                v if v == State::LABEL_READY => "0075ca",   // blue
                v if v == State::LABEL_PAUSE => "e4e669",   // yellow
                v if v == State::LABEL_PENDING => "d3d3d3", // gray
                v if v == State::LABEL_RUNNING => "c2e0c6", // light green
                _ => "ededed",
            }
        } else if label.starts_with(PIPELINE_PREFIX) || label.starts_with(STAGE_PREFIX) {
            "ededed"
        } else {
            "ededed" // fallback light gray
        }
    }

    /// Convert a flag name to its GitHub label representation.
    fn flag_to_label(name: &str) -> String {
        format!("{FLAG_PREFIX}{name}")
    }

    /// Parse a GitHub label string back to a flag name.
    fn label_to_flag(label: &str) -> Option<&str> {
        label.strip_prefix(FLAG_PREFIX)
    }

    fn parse_repo(&self) -> anyhow::Result<(&str, &str)> {
        self.backend_config.parse_repo()
    }

    /// Returns the configured reports branch, if any.
    fn reports_branch(&self) -> Option<&str> {
        self.backend_config.reports_branch.as_deref()
    }

    /// Returns the configured reports path prefix (default: "reports").
    fn reports_path(&self) -> &str {
        self.backend_config
            .reports_path
            .as_deref()
            .unwrap_or("reports")
    }

    /// Low-level: write the raw serialized task body.
    async fn update_task_description(&self, id: u64, description: &str) -> anyhow::Result<()> {
        let (owner, repo) = self.parse_repo()?;
        let url = format!("/repos/{owner}/{repo}/issues/{id}");
        let body = serde_json::json!({ "body": description });
        retry_github("update issue body", || {
            self.octocrab.patch(url.clone(), Some(&body))
        })
        .await
        .map(|_: serde_json::Value| ())?;
        Ok(())
    }

    /// Apply a state change on a GitHub issue via labels.
    async fn apply_state_change(&self, id: u64, state: &State) -> anyhow::Result<()> {
        let (owner, repo) = self.parse_repo()?;

        // Fetch current labels and remove all existing state:/pipeline:/stage: labels
        let issue: IssueResponse = retry_github("get issue labels", || {
            self.octocrab
                .get(format!("/repos/{owner}/{repo}/issues/{id}"), None::<&()>)
        })
        .await?;
        for label in &issue.labels {
            if label.name.starts_with(STATE_PREFIX)
                || label.name.starts_with(PIPELINE_PREFIX)
                || label.name.starts_with(STAGE_PREFIX)
            {
                let _ = retry_github("remove state label", || async {
                    self.octocrab
                        .issues(owner, repo)
                        .remove_label(id, &label.name)
                        .await
                })
                .await;
            }
        }

        // Add new state labels if not empty
        let new_labels = Self::state_to_labels(state);
        if !new_labels.is_empty() {
            // Ensure all labels exist before assigning them
            for label in &new_labels {
                let color = Self::state_label_color(label);
                let desc = format!("State: {label}");
                self.ensure_label_exists(label, color, &desc).await?;
            }
            retry_github("add state labels", || async {
                self.octocrab
                    .issues(owner, repo)
                    .add_labels(id, &new_labels)
                    .await
            })
            .await?;
        }

        Ok(())
    }

    /// Apply a signal change on a GitHub issue (remove old signal labels, add new one).
    async fn apply_signal_change(
        &self,
        id: u64,
        signal: Option<&zbobr_api::Signal>,
    ) -> anyhow::Result<()> {
        let (owner, repo) = self.parse_repo()?;

        // Fetch current labels and remove all existing signal: labels
        let issue: IssueResponse = retry_github("get issue labels", || {
            self.octocrab
                .get(format!("/repos/{owner}/{repo}/issues/{id}"), None::<&()>)
        })
        .await?;
        for label in &issue.labels {
            if Self::label_to_signal(&label.name).is_some() {
                let _ = retry_github("remove signal label", || async {
                    self.octocrab
                        .issues(owner, repo)
                        .remove_label(id, &label.name)
                        .await
                })
                .await;
            }
        }

        // Add new signal label if provided
        if let Some(sig) = signal {
            let label = Self::signal_to_label(sig);
            let labels: Vec<String> = vec![label];
            retry_github("add signal label", || async {
                self.octocrab
                    .issues(owner, repo)
                    .add_labels(id, &labels)
                    .await
            })
            .await?;
        }

        Ok(())
    }

    /// Apply flag changes on a GitHub issue (sync pause/confirm labels).
    async fn apply_flag_change(&self, id: u64, pause: bool, confirm: bool) -> anyhow::Result<()> {
        let (owner, repo) = self.parse_repo()?;

        for (flag_name, desired) in [("pause", pause), ("confirm", confirm)] {
            let label = Self::flag_to_label(flag_name);
            if desired {
                let labels: Vec<String> = vec![label];
                let _ = retry_github("add flag label", || async {
                    self.octocrab
                        .issues(owner, repo)
                        .add_labels(id, &labels)
                        .await
                })
                .await;
            } else {
                let _ = retry_github("remove flag label", || async {
                    self.octocrab
                        .issues(owner, repo)
                        .remove_label(id, &label)
                        .await
                })
                .await;
            }
        }

        Ok(())
    }

    /// List all labels in the repository.
    async fn list_labels(&self) -> anyhow::Result<Vec<String>> {
        let (owner, repo) = self.parse_repo()?;
        let labels: Vec<octocrab::models::Label> = retry_github("list labels", || async {
            self.octocrab
                .issues(owner, repo)
                .list_labels_for_repo()
                .per_page(100)
                .send()
                .await
        })
        .await?
        .items;
        Ok(labels.into_iter().map(|l| l.name).collect())
    }

    /// Create a label in the repository.
    async fn create_label(&self, name: &str, color: &str, description: &str) -> anyhow::Result<()> {
        let (owner, repo) = self.parse_repo()?;
        retry_github("create label", || async {
            self.octocrab
                .issues(owner, repo)
                .create_label(name, color, description)
                .await
        })
        .await?;
        Ok(())
    }

    /// Ensure a label exists in the repository.
    /// Creates the label if it doesn't exist; silently ignores 422 (already exists) errors.
    async fn ensure_label_exists(
        &self,
        name: &str,
        color: &str,
        description: &str,
    ) -> anyhow::Result<()> {
        let (owner, repo) = self.parse_repo()?;
        match self
            .octocrab
            .issues(owner, repo)
            .create_label(name, color, description)
            .await
        {
            Ok(_) => {
                tracing::debug!("Created label '{name}'");
                Ok(())
            }
            Err(octocrab::Error::GitHub { source, .. })
                if source.status_code.as_u16() == 422 =>
            {
                tracing::debug!("Label '{name}' already exists");
                Ok(())
            }
            Err(e) => Err(octocrab_to_anyhow(e)),
        }
    }

    /// Update a label's color and description.
    async fn update_label(&self, name: &str, color: &str, description: &str) -> anyhow::Result<()> {
        let (owner, repo) = self.parse_repo()?;
        let url = format!("/repos/{owner}/{repo}/labels/{name}");
        let body = serde_json::json!({
            "color": color,
            "description": description,
        });
        retry_github("update label", || {
            self.octocrab.patch(url.clone(), Some(&body))
        })
        .await
        .map(|_: serde_json::Value| ())?;
        Ok(())
    }

    /// Delete a label from the repository.
    async fn delete_label(&self, name: &str) -> anyhow::Result<()> {
        let (owner, repo) = self.parse_repo()?;
        let url = format!("/repos/{owner}/{repo}/labels/{name}");
        retry_github("delete label", || {
            self.octocrab._delete(url.clone(), None::<&()>)
        })
        .await
        .map(|_| ())?;
        Ok(())
    }

    /// Ensure the task repo exists (used internally by setup).
    async fn ensure_task_repo_exists(&self) -> anyhow::Result<()> {
        let (owner, repo) = self.parse_repo()?;
        let exists = retry_github("check task repo exists", || {
            self.octocrab
                .get::<RepoResponse, _, _>(format!("/repos/{owner}/{repo}"), None::<&()>)
        })
        .await
        .is_ok();

        if !exists {
            tracing::info!("Task repo {owner}/{repo} does not exist, creating...");
            // Try creating as org repo first, fall back to user repo
            let org_url = format!("/orgs/{owner}/repos");
            let org_body = serde_json::json!({
                "name": repo,
                "private": true,
                "auto_init": false,
            });
            let result = retry_github("create org repo", || async {
                self.octocrab.post(org_url.clone(), Some(&org_body)).await
            })
            .await;

            match result {
                Ok(_v) => {
                    let _: serde_json::Value = _v;
                    tracing::info!("Created private org repo {owner}/{repo}");
                }
                Err(_) => {
                    // Fall back to user repo
                    let user_url = "/user/repos".to_string();
                    let user_body = serde_json::json!({
                        "name": repo,
                        "private": true,
                        "auto_init": false,
                    });
                    retry_github("create user repo", || async {
                        self.octocrab.post(user_url.clone(), Some(&user_body)).await
                    })
                    .await
                    .map(|_: serde_json::Value| ())?;
                    tracing::info!("Created private user repo {owner}/{repo}");
                }
            }
            // Wait for repo init
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
        Ok(())
    }

    async fn setup(&self, force: bool, signal_labels: &[String]) -> anyhow::Result<()> {
        tracing::info!(
            "Setting up GitHub repo: {} (force: {})",
            self.backend_config.github_repo,
            force
        );

        // Ensure the task repo exists
        self.ensure_task_repo_exists().await?;

        // Create flag labels
        let existing_labels = self.list_labels().await?;

        const FLAG_LABEL_COLOR: &str = "f9d0c4";

        for flag_name in ["pause", "confirm"] {
            let flag_label = Self::flag_to_label(flag_name);
            let flag_desc = format!("Flag: {}", flag_name);
            if !existing_labels.contains(&flag_label) {
                tracing::info!("Creating label '{flag_label}'");
                self.create_label(&flag_label, FLAG_LABEL_COLOR, &flag_desc)
                    .await?;
            } else if force {
                tracing::info!("Updating label '{flag_label}' (force)");
                self.update_label(&flag_label, FLAG_LABEL_COLOR, &flag_desc)
                    .await?;
            } else {
                tracing::info!("Label '{flag_label}' already exists");
            }
        }

        // Sync signal labels: delete obsolete, create missing
        const SIGNAL_LABEL_COLOR: &str = "c2e0c6";

        let existing_signal_labels: Vec<String> = existing_labels
            .iter()
            .filter(|l| l.starts_with(SIGNAL_PREFIX))
            .cloned()
            .collect();

        // Delete obsolete signal labels (exist in repo but not in required set)
        for label in &existing_signal_labels {
            if !signal_labels.contains(label) {
                tracing::info!("Deleting obsolete signal label '{label}'");
                self.delete_label(label).await?;
            }
        }

        // Create missing signal labels (required but not in repo)
        for label in signal_labels {
            if !existing_signal_labels.contains(label) {
                let desc = format!("Signal: {}", label.strip_prefix(SIGNAL_PREFIX).unwrap_or(label));
                tracing::info!("Creating signal label '{label}'");
                self.create_label(label, SIGNAL_LABEL_COLOR, &desc).await?;
            } else if force {
                let desc = format!("Signal: {}", label.strip_prefix(SIGNAL_PREFIX).unwrap_or(label));
                tracing::info!("Updating signal label '{label}' (force)");
                self.update_label(label, SIGNAL_LABEL_COLOR, &desc).await?;
            } else {
                tracing::info!("Signal label '{label}' already exists");
            }
        }

        // Create state labels programmatically from type constants
        let state_labels: Vec<String> = State::ALL_LABEL_NAMES
            .iter()
            .map(|name| format!("{}{name}", STATE_PREFIX))
            .chain(
                [Pipeline::MAIN, Pipeline::MERGE]
                    .iter()
                    .map(|name| format!("{}{name}", PIPELINE_PREFIX)),
            )
            .collect();

        for label_name in &state_labels {
            let color = Self::state_label_color(label_name);
            let desc = format!(
                "State: {}",
                label_name
            );
            if !existing_labels.contains(label_name) {
                tracing::info!("Creating label '{label_name}'");
                self.create_label(label_name, color, &desc).await?;
            } else if force {
                tracing::info!("Updating label '{label_name}' (force)");
                self.update_label(label_name, color, &desc).await?;
            } else {
                tracing::info!("Label '{label_name}' already exists");
            }
        }

        tracing::info!(
            "GitHub setup complete for {}",
            self.backend_config.github_repo
        );
        Ok(())
    }

    /// Parse an IssueResponse into a Task.
    fn issue_to_task(issue: IssueResponse) -> Task {
        let body = issue.body.unwrap_or_default();
        let (description, params_map, checklist) = parse_description_full(&body);

        // Promoted fields: read from params_map where they were stored
        let destination_repository = params_map.get("destination_repository").cloned();
        let destination_branch = params_map.get("destination_branch").cloned();
        let work_branch = params_map.get("work_branch").cloned();
        let pr_url = params_map.get("pr_url").cloned();

        // stack is stored as JSON in params_map
        let stack: Vec<StackEntry> = params_map
            .get("stack")
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        // state is stored as labels
        let state = Self::labels_to_state(&issue.labels);

        // signal is stored as a label
        let signal = issue
            .labels
            .iter()
            .find_map(|l| Self::label_to_signal(&l.name));

        let pause = issue
            .labels
            .iter()
            .any(|l| Self::label_to_flag(&l.name) == Some("pause"));

        let confirm = issue
            .labels
            .iter()
            .any(|l| Self::label_to_flag(&l.name) == Some("confirm"));

        Task {
            id: issue.number,
            title: issue.title,
            description,
            state,
            destination_repository,
            destination_branch,
            work_branch,
            pr_url,
            checklist,
            signal,
            stack,
            pause,
            confirm,
            pipeline_run_id: params_map
                .get("pipeline_run_id")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0),
            stage_count: params_map
                .get("stage_count")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0),
            etag: Some(body),
        }
    }

    /// Build the string parameters map for serialization, including promoted fields.
    fn task_to_string_params(task: &Task) -> HashMap<String, String> {
        let mut params: HashMap<String, String> = HashMap::new();
        if let Some(ref v) = task.pr_url {
            params.insert("pr_url".to_string(), v.clone());
        }
        if let Some(ref v) = task.destination_repository {
            params.insert("destination_repository".to_string(), v.clone());
        }
        if let Some(ref v) = task.destination_branch {
            params.insert("destination_branch".to_string(), v.clone());
        }
        if let Some(ref v) = task.work_branch {
            params.insert("work_branch".to_string(), v.clone());
        }
        if !task.stack.is_empty() {
            if let Ok(json) = serde_json::to_string(&task.stack) {
                params.insert("stack".to_string(), json);
            }
        }
        if task.pipeline_run_id > 0 {
            params.insert(
                "pipeline_run_id".to_string(),
                task.pipeline_run_id.to_string(),
            );
        }
        if task.stage_count > 0 {
            params.insert(
                "stage_count".to_string(),
                task.stage_count.to_string(),
            );
        }
        params
    }

    fn record_cooling(&self, id: u64) {
        let deadline = tokio::time::Instant::now() + COOLING_DURATION;
        let mut map = self.cooling_deadlines.lock().unwrap();
        map.insert(id, deadline);
    }

    /// Wait until the cooling period expires for a specific issue.
    async fn await_cooling_for(&self, id: u64) {
        let deadline = {
            let mut map = self.cooling_deadlines.lock().unwrap();
            match map.get(&id).copied() {
                Some(d) if d > tokio::time::Instant::now() => Some(d),
                Some(_) => {
                    map.remove(&id);
                    None
                }
                None => None,
            }
        };
        if let Some(deadline) = deadline {
            tracing::debug!("Issue #{id}: waiting for cooling period to expire");
            tokio::time::sleep_until(deadline).await;
            let mut map = self.cooling_deadlines.lock().unwrap();
            map.remove(&id);
        }
    }

    /// Wait until all cooling periods expire (used before list queries).
    async fn await_all_cooling(&self) {
        let latest_deadline = {
            let map = self.cooling_deadlines.lock().unwrap();
            map.values().max().copied()
        };
        if let Some(deadline) = latest_deadline {
            if deadline > tokio::time::Instant::now() {
                tracing::debug!("Waiting for cooling period to expire before listing issues");
                tokio::time::sleep_until(deadline).await;
            }
            let mut map = self.cooling_deadlines.lock().unwrap();
            let now = tokio::time::Instant::now();
            map.retain(|_, d| *d > now);
        }
    }

    /// Internal: fetch task from GitHub without cooling check.
    async fn fetch_task(&self, id: u64) -> anyhow::Result<Task> {
        let (owner, repo) = self.parse_repo()?;
        let issue: IssueResponse = retry_github("get issue", || {
            self.octocrab
                .get(format!("/repos/{owner}/{repo}/issues/{id}"), None::<&()>)
        })
        .await?;
        Ok(Self::issue_to_task(issue))
    }

    /// Internal: read task with cooling check.
    async fn read_task(&self, id: u64) -> anyhow::Result<Task> {
        self.await_cooling_for(id).await;
        self.fetch_task(id).await
    }

    /// Get or create a per-task lock.
    fn task_lock(&self, id: u64) -> Arc<tokio::sync::Mutex<()>> {
        let mut locks = self.task_locks.lock().unwrap();
        locks
            .entry(id)
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    }

    /// Internal: close a task (issue).
    async fn close_task_internal(&self, id: u64) -> anyhow::Result<()> {
        let (owner, repo) = self.parse_repo()?;
        let url = format!("/repos/{owner}/{repo}/issues/{id}");
        let body = serde_json::json!({ "state": "closed" });
        retry_github("close issue", || {
            self.octocrab.patch(url.clone(), Some(&body))
        })
        .await
        .map(|_: serde_json::Value| ())?;
        Ok(())
    }

    /// Internal: read-modify-write a task atomically.
    async fn modify_task_internal(
        &self,
        id: u64,
        mutate: Box<dyn FnOnce(Task) -> Task + Send>,
    ) -> anyhow::Result<Task> {
        let task = self.fetch_task(id).await?;
        let original_state = task.state.clone();
        let original_signal = task.signal.clone();
        let original_pause = task.pause;
        let original_confirm = task.confirm;
        let expected_description = task.etag.clone().unwrap_or_else(|| {
            let string_params = Self::task_to_string_params(&task);
            serialize_description_full(&task.description, &string_params, &task.checklist)
        });

        let task = mutate(task);

        let string_params = Self::task_to_string_params(&task);
        let new_description =
            serialize_description_full(&task.description, &string_params, &task.checklist);

        // Write description with retry and conflict detection
        const MAX_RETRIES: u32 = 3;
        let mut new_desc = new_description;
        let mut expected_desc = expected_description;
        for attempt in 1..=MAX_RETRIES {
            match self.update_task_description(id, &new_desc).await {
                Ok(()) => break,
                Err(e) if attempt >= MAX_RETRIES => return Err(e),
                Err(_) => {}
            }
            let current_task = self.fetch_task(id).await?;
            let current_body = match current_task.etag {
                Some(etag) => etag,
                None => {
                    let sp = Self::task_to_string_params(&current_task);
                    serialize_description_full(
                        &current_task.description,
                        &sp,
                        &current_task.checklist,
                    )
                }
            };
            if current_body != expected_desc {
                new_desc =
                    merge_concurrent_description_updates(&expected_desc, &current_body, &new_desc);
                expected_desc = current_body;
            }
        }

        if task.state != original_state {
            self.apply_state_change(id, &task.state).await?;
        }
        if task.signal != original_signal {
            self.apply_signal_change(id, task.signal.as_ref()).await?;
        }
        if task.pause != original_pause || task.confirm != original_confirm {
            self.apply_flag_change(id, task.pause, task.confirm).await?;
        }

        self.record_cooling(id);
        let mut saved_task = task;
        saved_task.etag = Some(new_desc);
        Ok(saved_task)
    }

    /// Internal: get task comments.
    async fn get_task_comments_internal(&self, id: u64) -> anyhow::Result<Vec<Comment>> {
        let (owner, repo) = self.parse_repo()?;
        let comments: Vec<CommentResponse> = retry_github("list issue comments", || {
            self.octocrab.get(
                format!("/repos/{owner}/{repo}/issues/{id}/comments"),
                None::<&()>,
            )
        })
        .await?;

        Ok(comments
            .into_iter()
            .map(|c| {
                let body = c.body.unwrap_or_default();
                let timestamp = c.created_at.unwrap_or_default();

                let mut parts = body.splitn(2, '\n');
                let tag_line = parts.next().unwrap_or("");
                let rest = parts.next();

                let (tag, raw_text) = match tag_line.parse::<CommentTag>() {
                    Ok(t) => {
                        let body_text = rest.unwrap_or("").trim_start().to_string();
                        (t, body_text)
                    }
                    Err(_) => (
                        CommentTag::new(String::new(), 0, String::new(), String::new(), None, None),
                        body.clone(),
                    ),
                };

                let (text, report_name) = extract_report_link(&raw_text);

                Comment {
                    timestamp,
                    stage: tag.stage,
                    hostname: tag.hostname,
                    tool: tag.tool,
                    model: tag.model,
                    text,
                    pipeline: tag.pipeline,
                    pipeline_run_id: tag.pipeline_run_id,
                    caller_pipeline: tag.caller_pipeline,
                    caller_pipeline_run_id: tag.caller_pipeline_run_id,
                    report_name,
                    prompt_name: None,
                }
            })
            .collect())
    }

    /// Internal: post a task comment.
    async fn post_task_comment_internal(
        &self,
        id: u64,
        stage: &str,
        hostname: &str,
        tool: Option<Tool>,
        model: Option<Model>,
        body: &str,
        pipeline: &str,
        pipeline_run_id: u64,
        caller_pipeline: Option<&str>,
        caller_pipeline_run_id: Option<u64>,
        report_name: Option<&str>,
        prompt_name: Option<&str>,
    ) -> anyhow::Result<()> {
        let (owner, repo) = self.parse_repo()?;

        let mut tag = CommentTag::new(
            pipeline.to_string(),
            pipeline_run_id,
            stage.to_string(),
            hostname.to_string(),
            tool,
            model,
        );
        if let (Some(cp), Some(cr)) = (caller_pipeline, caller_pipeline_run_id) {
            tag = tag.with_caller(cp.to_string(), cr);
        }

        let reports_branch = self.reports_branch().unwrap_or("main");
        let reports_path = self.reports_path();

        let mut body_extended = body.to_string();
        if let Some(rn) = report_name {
            body_extended = format!(
                "{body_extended}{}",
                format_report_link(owner, repo, reports_branch, reports_path, id, rn)
            );
        }
        if let Some(pn) = prompt_name {
            body_extended = format!(
                "{body_extended}{}",
                format_report_link(owner, repo, reports_branch, reports_path, id, pn)
            );
        }

        let formatted_body = format!("{}\n\n{}", tag, body_extended);

        retry_github("create issue comment", || async {
            self.octocrab
                .issues(owner, repo)
                .create_comment(id, &formatted_body)
                .await
        })
        .await?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Report file storage (GitHub Contents API)
// ---------------------------------------------------------------------------

impl ZbobrTaskBackendGithubImpl {
    /// Store a report file in the GitHub repo, deduplicating with `_N` suffix if needed.
    /// Returns the actual filename (without directory prefix).
    async fn store_report(
        &self,
        task_id: u64,
        base_name: &str,
        content: &str,
    ) -> anyhow::Result<String> {
        let (owner, repo) = self.parse_repo()?;
        let reports_path = self.reports_path();
        let reports_branch = self.reports_branch();
        let dir = format!("{reports_path}/task_{task_id}");

        // When checking existence on a non-default branch, pass ?ref=
        let ref_query: Option<Vec<(&str, &str)>> =
            reports_branch.map(|b| vec![("ref", b)]);

        let mut n = 0u32;
        let filename = loop {
            let candidate = if n == 0 {
                format!("{base_name}.md")
            } else {
                format!("{base_name}_{n}.md")
            };
            let path = format!("{dir}/{candidate}");

            // 404 → file does not exist → is_ok() == false; no retry for 404
            let exists = self
                .octocrab
                .get::<serde_json::Value, _, _>(
                    format!("/repos/{owner}/{repo}/contents/{path}"),
                    ref_query.as_ref(),
                )
                .await
                .is_ok();

            if !exists {
                break candidate;
            }
            n += 1;
        };

        let path = format!("{dir}/{filename}");
        let message = format!("zbobr: store report {filename} for task #{task_id}");
        let encoded = BASE64.encode(content.as_bytes());

        let mut body = serde_json::json!({
            "message": message,
            "content": encoded,
        });
        if let Some(branch) = reports_branch {
            body["branch"] = serde_json::Value::String(branch.to_string());
        }

        retry_github("create report file", || async {
            self.octocrab
                .put::<serde_json::Value, _, _>(
                    format!("/repos/{owner}/{repo}/contents/{path}"),
                    Some(&body),
                )
                .await
        })
        .await?;

        tracing::debug!("Stored report for task {task_id}: {filename}");
        Ok(filename)
    }

    /// Read a report file from the GitHub repo by exact name.
    async fn read_report_internal(&self, task_id: u64, name: &str) -> anyhow::Result<String> {
        anyhow::ensure!(!name.contains(".."), "Invalid report name: {name}");
        let (owner, repo) = self.parse_repo()?;
        let reports_path = self.reports_path();
        let path = format!("{reports_path}/task_{task_id}/{name}");
        let ref_query: Option<Vec<(&str, &str)>> =
            self.reports_branch().map(|b| vec![("ref", b)]);

        let resp: ContentResponse = retry_github("read report file", || {
            self.octocrab.get(
                format!("/repos/{owner}/{repo}/contents/{path}"),
                ref_query.as_ref(),
            )
        })
        .await?;

        let encoded = resp
            .content
            .ok_or_else(|| anyhow::anyhow!("Report file has no content: {name}"))?;
        // GitHub returns base64 with embedded newlines
        let clean: String = encoded.chars().filter(|c| !c.is_whitespace()).collect();
        let bytes = BASE64
            .decode(&clean)
            .map_err(|e| anyhow::anyhow!("Failed to decode report content: {e}"))?;
        String::from_utf8(bytes)
            .map_err(|e| anyhow::anyhow!("Report content is not valid UTF-8: {e}"))
    }
}

// ---------------------------------------------------------------------------
// GithubTaskWeak / GithubTaskMut
// ---------------------------------------------------------------------------

use tokio::sync::OwnedMutexGuard;
use zbobr_api::backend::{TaskMut, TaskWeak};

struct GithubTaskWeak {
    id: u64,
    backend: Arc<ZbobrTaskBackendGithubImpl>,
    saved_snapshot: Arc<std::sync::Mutex<Option<Task>>>,
}

#[async_trait]
impl TaskWeak for GithubTaskWeak {
    fn task_id(&self) -> u64 {
        self.id
    }

    async fn snapshot(&self, refresh: bool) -> anyhow::Result<Task> {
        if !refresh && let Some(task) = self.saved_snapshot.lock().unwrap().clone() {
            return Ok(task);
        }

        let task = self.backend.read_task(self.id).await?;
        *self.saved_snapshot.lock().unwrap() = Some(task.clone());
        Ok(task)
    }

    async fn upgrade(&self) -> anyhow::Result<Box<dyn TaskMut>> {
        let lock = self.backend.task_lock(self.id);
        let guard = lock.lock_owned().await;
        Ok(Box::new(GithubTaskMut {
            id: self.id,
            backend: self.backend.clone(),
            saved_snapshot: self.saved_snapshot.clone(),
            _guard: guard,
        }))
    }

    async fn get_comments(&self) -> anyhow::Result<Vec<Comment>> {
        self.backend.get_task_comments_internal(self.id).await
    }

    async fn read_report(&self, name: &str) -> anyhow::Result<String> {
        self.backend.read_report_internal(self.id, name).await
    }
}

struct GithubTaskMut {
    id: u64,
    backend: Arc<ZbobrTaskBackendGithubImpl>,
    saved_snapshot: Arc<std::sync::Mutex<Option<Task>>>,
    _guard: OwnedMutexGuard<()>,
}

#[async_trait]
impl TaskMut for GithubTaskMut {
    fn task_id(&self) -> u64 {
        self.id
    }

    async fn snapshot(&self, refresh: bool) -> anyhow::Result<Task> {
        if !refresh && let Some(task) = self.saved_snapshot.lock().unwrap().clone() {
            return Ok(task);
        }

        let task = self.backend.read_task(self.id).await?;
        *self.saved_snapshot.lock().unwrap() = Some(task.clone());
        Ok(task)
    }

    async fn modify_task(
        &self,
        mutate: Box<dyn FnOnce(Task) -> Task + Send>,
    ) -> anyhow::Result<()> {
        let task = self.backend.modify_task_internal(self.id, mutate).await?;
        *self.saved_snapshot.lock().unwrap() = Some(task);
        Ok(())
    }

    async fn close(&self) -> anyhow::Result<()> {
        self.backend.close_task_internal(self.id).await
    }

    async fn post_comment(
        &self,
        stage: &str,
        hostname: &str,
        tool: Option<Tool>,
        model: Option<Model>,
        body: &str,
        pipeline: &str,
        pipeline_run_id: u64,
        caller_pipeline: Option<&str>,
        caller_pipeline_run_id: Option<u64>,
        report_text: Option<&str>,
        prompt_text: Option<&str>,
    ) -> anyhow::Result<()> {
        let tag = comment_tag(body);
        let report_name = if let Some(text) = report_text {
            let base_name = format!("report_{pipeline}_{pipeline_run_id}_{stage}_{tag}");
            Some(self.backend.store_report(self.id, &base_name, text).await?)
        } else {
            None
        };

        let prompt_name = if let Some(text) = prompt_text {
            let base_name = format!("prompt_{pipeline}_{pipeline_run_id}_{stage}_{tag}");
            Some(self.backend.store_report(self.id, &base_name, text).await?)
        } else {
            None
        };

        self.backend
            .post_task_comment_internal(
                self.id,
                stage,
                hostname,
                tool,
                model,
                body,
                pipeline,
                pipeline_run_id,
                caller_pipeline,
                caller_pipeline_run_id,
                report_name.as_deref(),
                prompt_name.as_deref(),
            )
            .await
    }

    fn downgrade(self: Box<Self>) -> Box<dyn TaskWeak> {
        Box::new(GithubTaskWeak {
            id: self.id,
            backend: self.backend.clone(),
            saved_snapshot: self.saved_snapshot.clone(),
        })
    }
}

// ---------------------------------------------------------------------------
// ArcTaskBackendGithub — proper TaskBackend wrapper
// ---------------------------------------------------------------------------

/// Arc-wrapped GitHub backend that properly returns TaskWeak/TaskMut handles.
#[derive(Clone)]
pub struct TaskBackendGithub {
    inner: Arc<ZbobrTaskBackendGithubImpl>,
}

impl TaskBackendGithub {
    pub fn from_config(config: ZbobrTaskBackendGithubConfig) -> anyhow::Result<Self> {
        Ok(Self {
            inner: Arc::new(ZbobrTaskBackendGithubImpl::from_config(config)?),
        })
    }

    /// Create from config and validate connectivity to GitHub.
    pub async fn new(config: ZbobrTaskBackendGithubConfig) -> anyhow::Result<Self> {
        let backend = Self::from_config(config)?;
        backend.validate_connectivity().await?;
        Ok(backend)
    }
}

#[async_trait]
impl TaskBackend for TaskBackendGithub {
    async fn get_task(&self, id: u64) -> anyhow::Result<Box<dyn TaskWeak>> {
        // Verify the task exists
        let task = self.inner.read_task(id).await?;
        Ok(Box::new(GithubTaskWeak {
            id,
            backend: self.inner.clone(),
            saved_snapshot: Arc::new(std::sync::Mutex::new(Some(task))),
        }))
    }

    async fn list_tasks(&self) -> anyhow::Result<Vec<Box<dyn TaskWeak>>> {
        self.inner.await_all_cooling().await;

        let (owner, repo) = self.inner.parse_repo()?;
        let params = vec![
            ("state", "open".to_string()),
            ("per_page", "100".to_string()),
        ];

        let issues: Vec<IssueResponse> = retry_github("list issues", || {
            self.inner
                .octocrab
                .get(format!("/repos/{owner}/{repo}/issues"), Some(&params))
        })
        .await?;

        let mut result: Vec<Box<dyn TaskWeak>> = Vec::new();
        for issue in issues {
            let id = issue.number;
            // Reuse list payload as the saved snapshot until a caller asks for refresh.
            let task = ZbobrTaskBackendGithubImpl::issue_to_task(issue);
            result.push(Box::new(GithubTaskWeak {
                id,
                backend: self.inner.clone(),
                saved_snapshot: Arc::new(std::sync::Mutex::new(Some(task))),
            }));
        }
        Ok(result)
    }

    async fn create_task(
        &self,
        title: &str,
        description: &str,
        state: State,
    ) -> anyhow::Result<u64> {
        let (owner, repo) = self.inner.parse_repo()?;
        let body = serialize_description_full(description, &HashMap::new(), &[]);

        let issue = retry_github("create issue", || async {
            let issues = self.inner.octocrab.issues(owner, repo);
            let builder = issues.create(title).body(body.clone());
            builder.send().await
        })
        .await?;

        let issue_id = issue.number;

        // Apply the initial state as a label
        if !state.is_empty() {
            self.inner.apply_state_change(issue_id, &state).await?;
        }

        Ok(issue_id)
    }

    async fn setup(&self, force: bool, signal_labels: &[String]) -> anyhow::Result<()> {
        self.inner.setup(force, signal_labels).await
    }

    async fn validate_connectivity(&self) -> anyhow::Result<()> {
        let (owner, repo) = self.inner.parse_repo()?;
        let task_repo_exists = retry_github("check task repo", || {
            self.inner
                .octocrab
                .get::<RepoResponse, _, _>(format!("/repos/{owner}/{repo}"), None::<&()>)
        })
        .await
        .is_ok();
        if !task_repo_exists {
            anyhow::bail!(
                "github_repo '{owner}/{repo}' is not accessible on GitHub.\n  \
                 Check your github_repo setting and ensure the repository exists \
                 and your token has access to it."
            );
        }

        Ok(())
    }

    fn debug_state(&self) -> String {
        format!(
            "GitHubTaskBackend({})",
            self.inner.backend_config.github_repo
        )
    }
}

/*
#[cfg(test)]
mod tests {
    use super::*;
    use zbobr_api::{Model, Parameter, Signal, Stage, Tool};

    #[test]
    fn issue_to_task_includes_confirm_flag() {
        let issue = IssueResponse {
            number: 10,
            title: "foo".to_string(),
            body: Some("".to_string()),
            state: "open".to_string(),
            labels: vec![IssueLabel {
                name: format!("{FLAG_PREFIX}confirm"),
            }],
        };

        let task = ZbobrTaskBackendGithub::issue_to_task(issue);
        assert!(task.confirm, "confirm flag should be parsed from labels");
    }

    #[tokio::test]
    async fn apply_flag_change_adds_and_removes_confirm_label() {
        // Install TLS provider required by octocrab.
        let _ = rustls::crypto::ring::default_provider().install_default();
        // This test just exercises the label loop; we don't hit GitHub.
        let config = crate::config::ZbobrTaskBackendGithubConfig {
            github_repo: "dummy/repo".to_string(),
            github_token: "dummy-token".to_string(),
        };
        let backend = ZbobrTaskBackendGithub::from_config(config).expect("backend init");

        // the method returns Result<(), _>; call with dummy values to ensure no panics
        // since actual network calls are inside retry_github we simply drop the future.
        // We cannot easily verify labels without mocking; ensure the code compiles and runs
        // the loop by invoking with both true/false combinations.
        let _ = backend.apply_flag_change(1, true, false, true).await;
        let _ = backend.apply_flag_change(1, false, true, false).await;
    }
}
*/

#[cfg(test)]
mod parse_tests {
    use zbobr_api::task::CommentTag;

    fn split_tag_body(input: &str) -> (CommentTag, String) {
        let mut parts = input.splitn(2, '\n');
        let tag_line = parts.next().unwrap_or("");
        let rest = parts.next();

        match tag_line.parse::<CommentTag>() {
            Ok(tag) => {
                let body = rest.unwrap_or("").trim_start().to_string();
                (tag, body)
            }
            Err(_) => (
                CommentTag::new(String::new(), 0, String::new(), String::new(), None, None),
                input.to_string(),
            ),
        }
    }

    #[test]
    fn test_parse_comment_tag_simple() {
        let input = "// main:1:planning by localhost\n\nThis is the body";
        let (tag, body) = split_tag_body(input);
        assert_eq!(tag.pipeline, "main");
        assert_eq!(tag.pipeline_run_id, 1);
        assert_eq!(tag.stage, "planning");
        assert_eq!(tag.hostname, "localhost");
        assert_eq!(body, "This is the body");
    }

    #[test]
    fn test_parse_comment_tag_new_format() {
        let input = "// main:3:reviewing by skynet\n\nRejected.";
        let (tag, body) = split_tag_body(input);
        assert_eq!(tag.pipeline, "main");
        assert_eq!(tag.pipeline_run_id, 3);
        assert_eq!(tag.stage, "reviewing");
        assert_eq!(tag.hostname, "skynet");
        assert_eq!(body, "Rejected.");
    }

    #[test]
    fn test_parse_comment_tag_no_tag_treated_as_empty() {
        let input = "This is just text without a tag";
        let (tag, body) = split_tag_body(input);
        assert_eq!(tag.stage, "");
        assert_eq!(tag.hostname, "");
        assert_eq!(body, "This is just text without a tag");
    }

    #[test]
    fn test_comment_tag_roundtrip() {
        let tag = CommentTag::new(
            "main".into(),
            5,
            "working".into(),
            "host".into(),
            None,
            None,
        );
        let s = tag.to_string();
        let parsed: CommentTag = s.parse().unwrap();
        assert_eq!(parsed, tag);

        let linked = CommentTag::new("sub".into(), 2, "done".into(), "host".into(), None, None)
            .with_caller("main".into(), 1);
        let linked_s = linked.to_string();
        assert_eq!(linked_s, "// sub:2:done by host for main:1");
        let linked_parsed: CommentTag = linked_s.parse().unwrap();
        assert_eq!(linked_parsed, linked);
    }
}

#[cfg(test)]
mod report_link_tests {
    use super::*;

    #[test]
    fn extract_no_link() {
        let (text, name) = extract_report_link("just some text");
        assert_eq!(text, "just some text");
        assert_eq!(name, None);
    }

    #[test]
    fn extract_with_link() {
        let body = "[report_success] Brief\n\n[report_main_1_working_success.md](https://github.com/org/repo/blob/main/reports/task_5/report_main_1_working_success.md)";
        let (text, name) = extract_report_link(body);
        assert_eq!(text, "[report_success] Brief");
        assert_eq!(name.as_deref(), Some("report_main_1_working_success.md"));
    }

    #[test]
    fn extract_non_report_link() {
        let body = "text\n\n[something](https://example.com/other)";
        let (text, name) = extract_report_link(body);
        assert_eq!(text, "text\n\n[something](https://example.com/other)");
        assert_eq!(name, None);
    }

    #[test]
    fn roundtrip() {
        let original = "[report_success] Brief summary";
        let filename = "report_main_1_working_success.md";
        let with_link = format!(
            "{}{}",
            original,
            format_report_link("org", "repo", "main", "reports", 5, filename)
        );
        let (text, name) = extract_report_link(&with_link);
        assert_eq!(text, original);
        assert_eq!(name.as_deref(), Some(filename));
    }

    #[test]
    fn link_only_body() {
        let body = "[report.md](https://github.com/o/r/blob/main/reports/task_1/report.md)";
        let (text, name) = extract_report_link(body);
        assert_eq!(text, "");
        assert_eq!(name.as_deref(), Some("report.md"));
    }
}
