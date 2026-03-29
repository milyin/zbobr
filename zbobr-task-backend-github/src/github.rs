use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use zbobr_api::{
    Comment, Signal, Task,
    backend::TaskBackend,
    task::{Pipeline, StackEntry, Stage, State, TaskContext},
};

// -- Parameter name constants (GitHub issue body parameter keys) --

const PARAM_DESTINATION_REPOSITORY: &str = "destination_repository";
const PARAM_DESTINATION_BRANCH: &str = "destination_branch";
const PARAM_WORK_BRANCH: &str = "work_branch";
const PARAM_PR_URL: &str = "pr_url";
const PARAM_STACK: &str = "stack";
const PARAM_PIPELINE: &str = "pipeline";
const PARAM_STAGE: &str = "stage";
const PARAM_SIGNAL: &str = "signal";
const PARAM_PIPELINE_RUN_ID: &str = "pipeline_run_id";
const PARAM_STAGE_COUNT: &str = "stage_count";
const PARAM_MAX_STAGE_COUNT: &str = "max_stage_count";
const PARAM_FLAG_PAUSE: &str = "pause";
const PARAM_FLAG_CONFIRM: &str = "confirm";
const PARAM_FLAG_VALUE_TRUE: &str = "true";

// -- Label prefix constants (GitHub-backend-specific) --

const STATE_PREFIX: &str = "state:";
const FLAG_LABEL_PREFIX: &str = "flag:";

// -- State label name constants --

const STATE_LABEL_DONE: &str = "done";
const STATE_LABEL_PAUSE: &str = "pause";
const STATE_LABEL_READY: &str = "ready";
const STATE_LABEL_PENDING: &str = "pending";
const STATE_LABEL_RUNNING: &str = "running";

const ALL_STATE_LABEL_NAMES: &[&str] = &[
    STATE_LABEL_DONE,
    STATE_LABEL_PAUSE,
    STATE_LABEL_READY,
    STATE_LABEL_PENDING,
    STATE_LABEL_RUNNING,
];

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
    state: String,
    labels: Vec<IssueLabel>,
    #[serde(default)]
    user: Option<IssueUser>,
}

#[derive(Debug, serde::Deserialize)]
struct IssueUser {
    login: String,
}

#[derive(Debug, serde::Deserialize)]
struct IssueLabel {
    name: String,
}

#[derive(Debug, serde::Deserialize)]
struct CommentResponse {
    body: Option<String>,
    created_at: Option<String>,
    html_url: Option<String>,
    user: Option<IssueUser>,
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

    /// Convert a state to its GitHub label representations (state:* label only).
    fn state_to_labels(state: &State) -> Vec<String> {
        let state_label = |name: &str| format!("{}{name}", STATE_PREFIX);

        match state {
            State::Empty => vec![],
            State::Done => vec![state_label(STATE_LABEL_DONE)],
            State::Pause => vec![state_label(STATE_LABEL_PAUSE)],
            State::Ready => vec![state_label(STATE_LABEL_READY)],
            State::Pending(_) => vec![state_label(STATE_LABEL_PENDING)],
            State::Running(_, _) => vec![state_label(STATE_LABEL_RUNNING)],
            State::Unknown(raw) => vec![state_label(raw)],
        }
    }

    /// Parse a State from GitHub issue labels and params.
    /// Pipeline and stage are passed as params (no longer stored in labels).
    fn labels_to_state(
        labels: &[IssueLabel],
        pipeline_param: Option<&str>,
        stage_param: Option<&str>,
    ) -> State {
        let mut state_value: Option<&str> = None;

        for label in labels {
            if let Some(v) = label.name.strip_prefix(STATE_PREFIX) {
                state_value = Some(v);
            }
        }

        match state_value {
            None => State::Empty,
            Some(v) if v == STATE_LABEL_DONE => State::Done,
            Some(v) if v == STATE_LABEL_PAUSE => State::Pause,
            Some(v) if v == STATE_LABEL_READY => State::Ready,
            Some(v) if v == STATE_LABEL_PENDING => match pipeline_param {
                Some(p) => State::Pending(Pipeline::from(p)),
                None => State::Unknown(format!("{}{}", STATE_PREFIX, STATE_LABEL_PENDING)),
            },
            Some(v) if v == STATE_LABEL_RUNNING => match (pipeline_param, stage_param) {
                (Some(p), Some(s)) => State::Running(Pipeline::from(p), Stage::from(s)),
                (None, Some(s)) => State::Unknown(format!(
                    "{}{}; stage:{s}",
                    STATE_PREFIX, STATE_LABEL_RUNNING
                )),
                (Some(p), None) => State::Unknown(format!(
                    "{}{}; pipeline:{p}",
                    STATE_PREFIX, STATE_LABEL_RUNNING
                )),
                (None, None) => State::Unknown(format!("{}{}", STATE_PREFIX, STATE_LABEL_RUNNING)),
            },
            Some(other) => State::Unknown(format!("{}{other}", STATE_PREFIX)),
        }
    }

    /// Return the GitHub label color for a state label.
    fn state_label_color(label: &str) -> &'static str {
        if let Some(state_name) = label.strip_prefix(STATE_PREFIX) {
            match state_name {
                v if v == STATE_LABEL_DONE => "0e8a16",    // green
                v if v == STATE_LABEL_READY => "0075ca",   // blue
                v if v == STATE_LABEL_PAUSE => "e4e669",   // yellow
                v if v == STATE_LABEL_PENDING => "d3d3d3", // gray
                v if v == STATE_LABEL_RUNNING => "c2e0c6", // light green
                _ => "ededed",
            }
        } else {
            "ededed" // fallback light gray
        }
    }

    fn parse_repo(&self) -> anyhow::Result<(&str, &str)> {
        self.backend_config.parse_repo()
    }

    /// Build a URL prefix for report files of the given task, usable as a closure
    /// for `serialize_context` / `serialize_description_full`.
    fn report_url_prefix(&self, task_id: u64) -> Option<String> {
        let (owner, repo) = self.parse_repo().ok()?;
        let branch = self.reports_branch().unwrap_or("main");
        let reports_path = self.reports_path();
        Some(format!(
            "https://github.com/{owner}/{repo}/blob/{branch}/{reports_path}/task_{task_id}/"
        ))
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

        // Fetch current labels and remove all existing state: and legacy flag: labels
        let issue: IssueResponse = retry_github("get issue labels", || {
            self.octocrab
                .get(format!("/repos/{owner}/{repo}/issues/{id}"), None::<&()>)
        })
        .await?;
        for label in &issue.labels {
            if label.name.starts_with(STATE_PREFIX) || label.name.starts_with(FLAG_LABEL_PREFIX) {
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
            Err(octocrab::Error::GitHub { source, .. }) if source.status_code.as_u16() == 422 => {
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

    async fn setup(&self, force: bool) -> anyhow::Result<()> {
        tracing::info!(
            "Setting up GitHub repo: {} (force: {})",
            self.backend_config.github_repo,
            force
        );

        // Ensure the task repo exists
        self.ensure_task_repo_exists().await?;

        let existing_labels = self.list_labels().await?;

        // Create state labels programmatically from type constants
        let state_labels: Vec<String> = ALL_STATE_LABEL_NAMES
            .iter()
            .map(|name| format!("{}{name}", STATE_PREFIX))
            .collect();

        for label_name in &state_labels {
            let color = Self::state_label_color(label_name);
            let desc = format!("State: {}", label_name);
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

        // Delete obsolete managed labels (state:* not in the expected set)
        let expected_labels: std::collections::HashSet<&str> =
            state_labels.iter().map(|s| s.as_str()).collect();
        for label in &existing_labels {
            if label.starts_with(STATE_PREFIX) && !expected_labels.contains(label.as_str()) {
                tracing::info!("Deleting obsolete label '{label}'");
                self.delete_label(label).await?;
            }
        }

        tracing::info!(
            "GitHub setup complete for {}",
            self.backend_config.github_repo
        );
        Ok(())
    }

    /// Parse an IssueResponse into a Task.
    fn issue_to_task(issue: IssueResponse) -> anyhow::Result<Task> {
        let body = issue.body.unwrap_or_default();
        let (description, params_map, status, context) = parse_description_full(&body)?;

        // Promoted fields: read from params_map where they were stored
        let destination_repository = params_map.get(PARAM_DESTINATION_REPOSITORY).cloned();
        let destination_branch = params_map.get(PARAM_DESTINATION_BRANCH).cloned();
        let work_branch = params_map.get(PARAM_WORK_BRANCH).cloned();
        let pr_url = params_map.get(PARAM_PR_URL).cloned();

        // stack is stored as JSON in params_map
        let stack: Vec<StackEntry> = params_map
            .get(PARAM_STACK)
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        // pipeline, stage, and signal are stored as params
        let pipeline_param = params_map.get(PARAM_PIPELINE).map(|s| s.as_str());
        let stage_param = params_map.get(PARAM_STAGE).map(|s| s.as_str());
        let signal: Option<Signal> = params_map.get(PARAM_SIGNAL).and_then(|s| s.parse().ok());

        // state is stored as label; pipeline/stage come from params
        let state = Self::labels_to_state(&issue.labels, pipeline_param, stage_param);

        let pause = params_map
            .get(PARAM_FLAG_PAUSE)
            .map(|s| s == PARAM_FLAG_VALUE_TRUE)
            .unwrap_or(false);
        let confirm = params_map
            .get(PARAM_FLAG_CONFIRM)
            .map(|s| s == PARAM_FLAG_VALUE_TRUE)
            .unwrap_or(false);

        Ok(Task {
            id: issue.number,
            title: issue.title,
            description,
            state,
            destination_repository,
            destination_branch,
            work_branch,
            pr_url,
            context,
            signal,
            stack,
            status,
            pause,
            confirm,
            pipeline_run_id: params_map
                .get(PARAM_PIPELINE_RUN_ID)
                .and_then(|s| s.parse().ok())
                .unwrap_or(0),
            stage_count: params_map
                .get(PARAM_STAGE_COUNT)
                .and_then(|s| s.parse().ok())
                .unwrap_or(0),
            max_stage_count: params_map
                .get(PARAM_MAX_STAGE_COUNT)
                .and_then(|s| s.parse().ok())
                .unwrap_or(0),
            closed: issue.state == "closed",
            etag: Some(body),
        })
    }

    /// Build the string parameters map for serialization, including promoted fields.
    fn task_to_string_params(task: &Task) -> HashMap<String, String> {
        let mut params: HashMap<String, String> = HashMap::new();
        if let Some(ref v) = task.pr_url {
            params.insert(PARAM_PR_URL.to_string(), v.clone());
        }
        if let Some(ref v) = task.destination_repository {
            params.insert(PARAM_DESTINATION_REPOSITORY.to_string(), v.clone());
        }
        if let Some(ref v) = task.destination_branch {
            params.insert(PARAM_DESTINATION_BRANCH.to_string(), v.clone());
        }
        if let Some(ref v) = task.work_branch {
            params.insert(PARAM_WORK_BRANCH.to_string(), v.clone());
        }
        if !task.stack.is_empty() {
            if let Ok(json) = serde_json::to_string(&task.stack) {
                params.insert(PARAM_STACK.to_string(), json);
            }
        }
        // Store pipeline and stage as params (not labels)
        match &task.state {
            State::Pending(pipeline) => {
                params.insert(PARAM_PIPELINE.to_string(), pipeline.as_str().to_string());
            }
            State::Running(pipeline, stage) => {
                params.insert(PARAM_PIPELINE.to_string(), pipeline.as_str().to_string());
                params.insert(PARAM_STAGE.to_string(), stage.as_str().to_string());
            }
            _ => {}
        }
        // Store signal as param (not label)
        if let Some(ref signal) = task.signal {
            params.insert(PARAM_SIGNAL.to_string(), signal.to_string());
        }
        if task.pipeline_run_id > 0 {
            params.insert(
                PARAM_PIPELINE_RUN_ID.to_string(),
                task.pipeline_run_id.to_string(),
            );
        }
        if task.stage_count > 0 {
            params.insert(PARAM_STAGE_COUNT.to_string(), task.stage_count.to_string());
        }
        if task.max_stage_count > 0 {
            params.insert(
                PARAM_MAX_STAGE_COUNT.to_string(),
                task.max_stage_count.to_string(),
            );
        }
        if task.pause {
            params.insert(
                PARAM_FLAG_PAUSE.to_string(),
                PARAM_FLAG_VALUE_TRUE.to_string(),
            );
        }
        if task.confirm {
            params.insert(
                PARAM_FLAG_CONFIRM.to_string(),
                PARAM_FLAG_VALUE_TRUE.to_string(),
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
        Self::issue_to_task(issue)
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
        let comments = self.get_task_comments_internal(id).await?;
        let url_prefix = self.report_url_prefix(id);
        let make_url = |filename: &str| -> String {
            match &url_prefix {
                Some(prefix) => format!("{prefix}{filename}"),
                None => filename.to_string(),
            }
        };
        let expected_description = task.etag.clone().unwrap_or_else(|| {
            let string_params = Self::task_to_string_params(&task);
            serialize_description_full(
                &task.description,
                &string_params,
                &task.status,
                &task.context,
                &comments,
                Some(&make_url),
            )
        });

        let task = mutate(task);

        let string_params = Self::task_to_string_params(&task);
        let new_description = serialize_description_full(
            &task.description,
            &string_params,
            &task.status,
            &task.context,
            &comments,
            Some(&make_url),
        );

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
                        &current_task.status,
                        &current_task.context,
                        &comments,
                        Some(&make_url),
                    )
                }
            };
            if current_body != expected_desc {
                new_desc =
                    merge_concurrent_description_updates(&expected_desc, &current_body, &new_desc)?;
                expected_desc = current_body;
            }
        }

        // Always apply state change to ensure legacy flag: labels are removed even when state is unchanged.
        self.apply_state_change(id, &task.state).await?;

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
                let timestamp: chrono::DateTime<chrono::FixedOffset> = c
                    .created_at
                    .as_deref()
                    .unwrap_or("1970-01-01T00:00:00Z")
                    .parse()
                    .unwrap_or_else(|_| {
                        chrono::DateTime::parse_from_rfc3339("1970-01-01T00:00:00Z").unwrap()
                    });

                let username = c
                    .user
                    .as_ref()
                    .map(|u| u.login.clone())
                    .unwrap_or("unknown".to_string());

                Comment {
                    timestamp,
                    username,
                    body: body.clone(),
                    url: c.html_url,
                }
            })
            .collect())
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
        let ref_query: Option<Vec<(&str, &str)>> = reports_branch.map(|b| vec![("ref", b)]);

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
        let message = format!("zbobr: store report {filename} for task # {task_id}");
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
        let ref_query: Option<Vec<(&str, &str)>> = self.reports_branch().map(|b| vec![("ref", b)]);

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

    async fn store_report(&self, base_name: &str, content: &str) -> anyhow::Result<String> {
        self.backend.store_report(self.id, base_name, content).await
    }

    fn report_url(&self, filename: &str) -> String {
        match self.backend.report_url_prefix(self.id) {
            Some(prefix) => format!("{prefix}{filename}"),
            None => filename.to_string(),
        }
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

    async fn list_tasks(&self, allowed_users: &[String]) -> anyhow::Result<Vec<Box<dyn TaskWeak>>> {
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
            // Filter by allowed_users (matched against issue author login).
            if !allowed_users.is_empty() {
                let author = issue.user.as_ref().map(|u| u.login.as_str()).unwrap_or("");
                if !allowed_users.iter().any(|u| u == author) {
                    continue;
                }
            }
            let id = issue.number;
            // Reuse list payload as the saved snapshot until a caller asks for refresh.
            match ZbobrTaskBackendGithubImpl::issue_to_task(issue) {
                Ok(task) => {
                    result.push(Box::new(GithubTaskWeak {
                        id,
                        backend: self.inner.clone(),
                        saved_snapshot: Arc::new(std::sync::Mutex::new(Some(task))),
                    }));
                }
                Err(e) => {
                    tracing::warn!("Skipping issue #{id}: failed to parse task: {e}");
                }
            }
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
        let body = serialize_description_full(
            description,
            &HashMap::new(),
            &None,
            &TaskContext::default(),
            &[],
            None,
        );

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

    async fn setup(&self, force: bool) -> anyhow::Result<()> {
        self.inner.setup(force).await
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

    fn task_repo_name(&self) -> Option<String> {
        Some(self.inner.backend_config.github_repo.clone())
    }
}

#[cfg(test)]
mod flag_tests {
    use super::*;
    use crate::separator::PARAMETERS_SEPARATOR;

    fn make_issue_with_params(key: &str, value: &str) -> IssueResponse {
        let body = format!("desc{PARAMETERS_SEPARATOR}{key}: {value}\n");
        IssueResponse {
            number: 1,
            title: "test".to_string(),
            body: Some(body),
            state: "open".to_string(),
            labels: vec![],
            user: None,
        }
    }

    #[test]
    fn issue_to_task_reads_pause_from_params() {
        let issue = make_issue_with_params(PARAM_FLAG_PAUSE, PARAM_FLAG_VALUE_TRUE);
        let task = ZbobrTaskBackendGithubImpl::issue_to_task(issue).unwrap();
        assert!(task.pause);
        assert!(!task.confirm);
    }

    #[test]
    fn issue_to_task_reads_confirm_from_params() {
        let issue = make_issue_with_params(PARAM_FLAG_CONFIRM, PARAM_FLAG_VALUE_TRUE);
        let task = ZbobrTaskBackendGithubImpl::issue_to_task(issue).unwrap();
        assert!(!task.pause);
        assert!(task.confirm);
    }

    #[test]
    fn task_to_string_params_includes_flags_when_set() {
        use zbobr_api::task::State;
        use zbobr_api::task::TaskContext;
        let task = Task {
            id: 1,
            title: "t".to_string(),
            description: "d".to_string(),
            state: State::Done,
            destination_repository: None,
            destination_branch: None,
            work_branch: None,
            pr_url: None,
            context: TaskContext::default(),
            signal: None,
            stack: vec![],
            status: None,
            pause: true,
            confirm: true,
            pipeline_run_id: 0,
            stage_count: 0,
            max_stage_count: 0,
            closed: false,
            etag: None,
        };
        let params = ZbobrTaskBackendGithubImpl::task_to_string_params(&task);
        assert_eq!(
            params.get(PARAM_FLAG_PAUSE).map(|s| s.as_str()),
            Some(PARAM_FLAG_VALUE_TRUE)
        );
        assert_eq!(
            params.get(PARAM_FLAG_CONFIRM).map(|s| s.as_str()),
            Some(PARAM_FLAG_VALUE_TRUE)
        );
    }
}
