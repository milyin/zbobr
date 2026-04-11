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
    task::{StackEntry, State, TaskContext},
};

// -- Parameter name constants (GitHub issue body parameter keys) --

const PARAM_WORK_BRANCH: &str = "work_branch";
const PARAM_PR_URL: &str = "pr_url";
const PARAM_STACK: &str = "stack";
const PARAM_STATE: &str = "state";
const PARAM_SIGNAL: &str = "signal";
const PARAM_PIPELINE_RUN_ID: &str = "pipeline_run_id";
const PARAM_STAGE_COUNT: &str = "stage_count";
const PARAM_MAX_STAGE_COUNT: &str = "max_stage_count";
const PARAM_FLAG_PAUSE: &str = "pause";
const PARAM_FLAG_CONFIRM: &str = "confirm";
const PARAM_FLAG_VALUE_TRUE: &str = "true";
const DEFAULT_REPORTS_PATH: &str = "reports";

// -- Label prefix constants (GitHub-backend-specific) --

const INSTANCE_LABEL_PREFIX: &str = "zbobr:";
const PIPELINE_LABEL_PREFIX: &str = "pipeline:";
const PAUSE_LABEL: &str = "pause";

const MAX_GITHUB_RETRY_ATTEMPTS: u64 = 5;

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

fn format_report_filename_timestamp() -> String {
    chrono::Local::now().format("%Y-%m-%d_%H-%M-%S_%z").to_string()
}

fn is_transient_octocrab_error(error: &octocrab::Error) -> bool {
    match error {
        octocrab::Error::GitHub { source, .. } => source.status_code.is_server_error(),
        _ => true,
    }
}

fn is_conflict_octocrab_error(error: &octocrab::Error) -> bool {
    matches!(
        error,
        octocrab::Error::GitHub { source, .. }
        if source.status_code.as_u16() == 409
    )
}

/// Retry a GitHub API operation up to MAX_GITHUB_RETRY_ATTEMPTS times on transient errors.
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
                if attempt < MAX_GITHUB_RETRY_ATTEMPTS && is_transient_octocrab_error(&e) {
                    tracing::warn!(
                        "Transient GitHub error during {op_name} (attempt {attempt}/{max}): {}",
                        format_octocrab_error(&e),
                        max = MAX_GITHUB_RETRY_ATTEMPTS
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
}

#[derive(Debug, serde::Deserialize)]
struct IssueUser {
    login: String,
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
    default_branch: String,
}

#[derive(Debug, serde::Deserialize)]
struct ContentResponse {
    content: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct GitRefObject {
    sha: String,
}

#[derive(Debug, serde::Deserialize)]
struct GitRefResponse {
    object: GitRefObject,
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
    pub fn from_config(mut backend_config: ZbobrTaskBackendGithubConfig) -> anyhow::Result<Self> {
        backend_config.validate()?;
        let token = backend_config.github_token.as_ref().to_owned();
        let octocrab = octocrab::Octocrab::builder()
            .personal_token(token)
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build octocrab client: {e}"))?;
        Ok(Self {
            backend_config,
            octocrab,
            cooling_deadlines: Mutex::new(HashMap::new()),
            task_locks: std::sync::Mutex::new(HashMap::new()),
        })
    }

    fn default_max_stage_count(&self) -> u64 {
        self.backend_config.default_max_stage_count
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
            .unwrap_or(DEFAULT_REPORTS_PATH)
    }

    async fn repo_info(&self, owner: &str, repo: &str) -> anyhow::Result<RepoResponse> {
        retry_github("read repository metadata", || {
            self.octocrab
                .get::<RepoResponse, _, _>(format!("/repos/{owner}/{repo}"), None::<&()>)
        })
        .await
    }

    async fn branch_exists(&self, owner: &str, repo: &str, branch: &str) -> anyhow::Result<bool> {
        match self
            .octocrab
            .get::<serde_json::Value, _, _>(
                format!("/repos/{owner}/{repo}/git/ref/heads/{branch}"),
                None::<&()>,
            )
            .await
        {
            Ok(_) => Ok(true),
            Err(octocrab::Error::GitHub { source, .. }) if source.status_code.as_u16() == 404 => {
                Ok(false)
            }
            Err(error) => Err(octocrab_to_anyhow(error)),
        }
    }

    async fn ensure_reports_branch_exists(&self) -> anyhow::Result<()> {
        let Some(reports_branch) = self.reports_branch() else {
            return Ok(());
        };

        let (owner, repo) = self.parse_repo()?;
        let repo_info = self.repo_info(owner, repo).await?;

        if reports_branch == repo_info.default_branch {
            return Ok(());
        }

        if self.branch_exists(owner, repo, reports_branch).await? {
            return Ok(());
        }

        let default_ref: GitRefResponse = retry_github("read default branch ref", || {
            self.octocrab.get(
                format!(
                    "/repos/{owner}/{repo}/git/ref/heads/{}",
                    repo_info.default_branch
                ),
                None::<&()>,
            )
        })
        .await?;

        let body = serde_json::json!({
            "ref": format!("refs/heads/{reports_branch}"),
            "sha": default_ref.object.sha,
        });

        let create_result: Result<serde_json::Value, octocrab::Error> = self
            .octocrab
            .post(format!("/repos/{owner}/{repo}/git/refs"), Some(&body))
            .await;

        match create_result {
            Ok(_) => {
                tracing::info!(
                    "Created reports branch '{}' from '{}' in {}/{}",
                    reports_branch,
                    repo_info.default_branch,
                    owner,
                    repo
                );
                Ok(())
            }
            Err(octocrab::Error::GitHub { source, .. }) if source.status_code.as_u16() == 422 => {
                tracing::info!(
                    "Reports branch '{}' already exists in {}/{}",
                    reports_branch,
                    owner,
                    repo
                );
                Ok(())
            }
            Err(error) => Err(octocrab_to_anyhow(error)),
        }
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

    /// Get labels currently applied to a specific issue.
    async fn get_issue_labels(&self, id: u64) -> anyhow::Result<Vec<String>> {
        let (owner, repo) = self.parse_repo()?;
        let labels: Vec<octocrab::models::Label> = retry_github("get issue labels", || {
            self.octocrab.get(
                format!("/repos/{owner}/{repo}/issues/{id}/labels"),
                None::<&()>,
            )
        })
        .await?;
        Ok(labels.into_iter().map(|l| l.name).collect())
    }

    /// Add labels to a specific issue.
    async fn add_issue_labels(&self, id: u64, labels: &[String]) -> anyhow::Result<()> {
        if labels.is_empty() {
            return Ok(());
        }
        let (owner, repo) = self.parse_repo()?;
        let body = serde_json::json!({ "labels": labels });
        let url = format!("/repos/{owner}/{repo}/issues/{id}/labels");
        retry_github("add issue labels", || {
            self.octocrab.post(url.clone(), Some(&body))
        })
        .await
        .map(|_: serde_json::Value| ())?;
        Ok(())
    }

    /// Remove a specific label from an issue. 404 is silently ignored (label already absent).
    async fn remove_issue_label(&self, id: u64, label: &str) -> anyhow::Result<()> {
        let (owner, repo) = self.parse_repo()?;
        let url = format!("/repos/{owner}/{repo}/issues/{id}/labels/{label}");
        match self.octocrab._delete(url, None::<&()>).await {
            Ok(_) => Ok(()),
            Err(octocrab::Error::GitHub { source, .. }) if source.status_code.as_u16() == 404 => {
                Ok(())
            }
            Err(e) => Err(octocrab_to_anyhow(e)),
        }
    }

    /// Update GitHub issue labels to reflect the current task pipeline state.
    ///
    /// - Sets `pipeline:<name>` labels for all active pipelines (from state + stack).
    /// - Clears `pipeline:<name>` labels for pipelines no longer active.
    /// - Sets or clears the `pause` label based on whether state is `State::Pause`.
    async fn apply_pipeline_and_pause_labels(
        &self,
        id: u64,
        state: &State,
        stack: &[StackEntry],
    ) -> anyhow::Result<()> {
        // Collect desired pipeline labels
        let mut desired_pipeline_labels: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        if let Some(pipeline) = state.pipeline() {
            desired_pipeline_labels
                .insert(format!("{PIPELINE_LABEL_PREFIX}{}", pipeline.as_str()));
        }
        for entry in stack {
            desired_pipeline_labels
                .insert(format!("{PIPELINE_LABEL_PREFIX}{}", entry.pipeline.as_str()));
        }

        // Read current issue labels
        let current_labels = self.get_issue_labels(id).await?;
        let current_pipeline_labels: std::collections::HashSet<String> = current_labels
            .iter()
            .filter(|l| l.starts_with(PIPELINE_LABEL_PREFIX))
            .cloned()
            .collect();
        let has_pause_label = current_labels.iter().any(|l| l == PAUSE_LABEL);

        // Add missing pipeline labels (ensure label exists in repo first)
        for label in desired_pipeline_labels.difference(&current_pipeline_labels) {
            let pipeline_name = label
                .strip_prefix(PIPELINE_LABEL_PREFIX)
                .unwrap_or(label.as_str());
            self.ensure_label_exists(label, "0052cc", &format!("Pipeline: {pipeline_name}"))
                .await?;
            self.add_issue_labels(id, &[label.clone()]).await?;
        }

        // Remove stale pipeline labels
        for label in current_pipeline_labels.difference(&desired_pipeline_labels) {
            self.remove_issue_label(id, label).await?;
        }

        // Sync pause label
        let is_paused = state.is_pause();
        if is_paused && !has_pause_label {
            self.ensure_label_exists(PAUSE_LABEL, "e4e669", "Task is paused")
                .await?;
            self.add_issue_labels(id, &[PAUSE_LABEL.to_string()]).await?;
        } else if !is_paused && has_pause_label {
            self.remove_issue_label(id, PAUSE_LABEL).await?;
        }

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
        self.ensure_reports_branch_exists().await?;

        let existing_labels = self.list_labels().await?;

        // Create zbobr:<instance> label for this instance
        let instance_label = format!("{}{}", INSTANCE_LABEL_PREFIX, self.backend_config.instance);
        let instance_color = "1d76db"; // blue
        let instance_desc = format!("Zbobr instance: {}", self.backend_config.instance);
        if !existing_labels.contains(&instance_label) {
            tracing::info!("Creating instance label '{instance_label}'");
            self.create_label(&instance_label, instance_color, &instance_desc)
                .await?;
        } else if force {
            tracing::info!("Updating instance label '{instance_label}' (force)");
            self.update_label(&instance_label, instance_color, &instance_desc)
                .await?;
        } else {
            tracing::info!("Instance label '{instance_label}' already exists");
        }

        // With force: clean up zbobr:* labels belonging to other instances
        if force {
            for label in &existing_labels {
                if label.starts_with(INSTANCE_LABEL_PREFIX) && label != &instance_label {
                    tracing::info!("Deleting other-instance label '{label}' (force)");
                    self.delete_label(label).await?;
                }
            }
        }

        tracing::info!(
            "GitHub setup complete for {}",
            self.backend_config.github_repo
        );
        Ok(())
    }

    /// Parse an IssueResponse into a Task.
    fn issue_to_task(&self, issue: IssueResponse) -> anyhow::Result<Task> {
        let body = issue.body.unwrap_or_default();
        let (description, params_map, status, context) = parse_description_full(&body)?;

        // Promoted fields: read from params_map where they were stored
        let work_branch = params_map.get(PARAM_WORK_BRANCH).cloned();
        let pr_url = params_map.get(PARAM_PR_URL).cloned();

        // stack is stored as JSON in params_map
        let stack: Vec<StackEntry> = params_map
            .get(PARAM_STACK)
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        let signal: Option<Signal> = params_map.get(PARAM_SIGNAL).and_then(|s| s.parse().ok());

        // state is stored as a single param in canonical format (e.g. "running:main:working")
        let state: State = params_map
            .get(PARAM_STATE)
            .map(|s| s.parse().unwrap_or(State::Empty))
            .unwrap_or(State::Empty);

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
            work_branch,
            pr_url,
            context,
            signal,
            stack,
            status,
            go_pause: pause,
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
                .unwrap_or(self.default_max_stage_count()),
            closed: issue.state == "closed",
            etag: Some(body),
        })
    }

    fn report_url_prefix_from_config(
        config: &ZbobrTaskBackendGithubConfig,
        task_id: u64,
    ) -> Option<String> {
        let (owner, repo) = config.parse_repo().ok()?;
        let branch = config.reports_branch.as_deref().unwrap_or("main");
        let reports_path = config.reports_path.as_deref().unwrap_or("reports");
        Some(format!(
            "https://github.com/{owner}/{repo}/blob/{branch}/{reports_path}/task_{task_id}/"
        ))
    }

    fn normalize_task_report_ref_for_config(
        config: &ZbobrTaskBackendGithubConfig,
        task_id: u64,
        field_name: &str,
        link: &str,
    ) -> anyhow::Result<String> {
        let Some(prefix) = Self::report_url_prefix_from_config(config, task_id) else {
            anyhow::bail!(
                "Invalid task context link in {field_name}: cannot build GitHub blob prefix for task #{task_id}; link='{link}'"
            );
        };

        let Some(filename) = link.strip_prefix(&prefix) else {
            anyhow::bail!(
                "Invalid task context link in {field_name}: expected full GitHub blob URL with prefix '{prefix}', got '{link}'"
            );
        };

        if filename.is_empty() || filename.contains('/') || filename.contains("..") {
            anyhow::bail!(
                "Invalid task context link in {field_name}: URL tail must be a single filename, got '{link}'"
            );
        }

        Ok(filename.to_string())
    }

    fn normalize_task_report_links_for_config(
        config: &ZbobrTaskBackendGithubConfig,
        task: &mut Task,
    ) -> anyhow::Result<()> {
        for (stage_idx, stage) in task.context.stages.iter_mut().enumerate() {
            if let Some(link) = stage.info.prompt_link.as_mut() {
                *link = Self::normalize_task_report_ref_for_config(
                    config,
                    task.id,
                    &format!("stage[{stage_idx}].prompt_link"),
                    link,
                )?;
            }
            if let Some(link) = stage.info.output_link.as_mut() {
                *link = Self::normalize_task_report_ref_for_config(
                    config,
                    task.id,
                    &format!("stage[{stage_idx}].output_link"),
                    link,
                )?;
            }

            for (record_idx, record) in stage.records.iter_mut().enumerate() {
                if let Some(link) = record.report_link.as_mut() {
                    *link = Self::normalize_task_report_ref_for_config(
                        config,
                        task.id,
                        &format!("stage[{stage_idx}].records[{record_idx}].report_link"),
                        link,
                    )?;
                }
            }
        }

        Ok(())
    }

    fn hydrate_issue_to_task(&self, issue: IssueResponse) -> anyhow::Result<Task> {
        let mut task = self.issue_to_task(issue)?;
        Self::normalize_task_report_links_for_config(&self.backend_config, &mut task)?;
        Ok(task)
    }

    /// Build the string parameters map for serialization, including promoted fields.
    fn task_to_string_params(task: &Task) -> HashMap<String, String> {
        let mut params: HashMap<String, String> = HashMap::new();
        if let Some(ref v) = task.pr_url {
            params.insert(PARAM_PR_URL.to_string(), v.clone());
        }
        if let Some(ref v) = task.work_branch {
            params.insert(PARAM_WORK_BRANCH.to_string(), v.clone());
        }
        if !task.stack.is_empty()
            && let Ok(json) = serde_json::to_string(&task.stack)
        {
            params.insert(PARAM_STACK.to_string(), json);
        }
        // Store state as a single param in canonical format
        let state_str = task.state.to_string();
        if !state_str.is_empty() {
            params.insert(PARAM_STATE.to_string(), state_str);
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
        if task.go_pause {
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
        self.hydrate_issue_to_task(issue)
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

        self.record_cooling(id);
        let mut saved_task = task;
        saved_task.etag = Some(new_desc);

        // Sync pipeline/pause labels to reflect the new state
        if let Err(e) = self
            .apply_pipeline_and_pause_labels(id, &saved_task.state, &saved_task.stack)
            .await
        {
            tracing::warn!("Task #{id}: failed to sync labels: {e}");
        }

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

        let allowed_usernames = self.backend_config.allowed_usernames.as_deref();

        Ok(comments
            .into_iter()
            .filter(|c| {
                if let Some(usernames) = allowed_usernames {
                    c.user
                        .as_ref()
                        .map(|u| usernames.contains(&u.login))
                        .unwrap_or(false)
                } else {
                    true
                }
            })
            .map(|c| {
                let body = c.body.unwrap_or_default();
                let parsed: chrono::DateTime<chrono::FixedOffset> = c
                    .created_at
                    .as_deref()
                    .unwrap_or("1970-01-01T00:00:00Z")
                    .parse()
                    .unwrap_or_else(|_| {
                        chrono::DateTime::parse_from_rfc3339("1970-01-01T00:00:00Z").unwrap()
                    });
                let timestamp = match self.backend_config.timezone {
                    Some(tz) => parsed.with_timezone(&*tz),
                    None => parsed,
                };

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
        self.ensure_reports_branch_exists().await?;

        let (owner, repo) = self.parse_repo()?;
        let reports_path = self.reports_path();
        let reports_branch = self.reports_branch();
        let dir = format!("{reports_path}/task_{task_id}");

        // When checking existence on a non-default branch, pass ?ref=
        let ref_query: Option<Vec<(&str, &str)>> = reports_branch.map(|b| vec![("ref", b)]);

        let timestamp = format_report_filename_timestamp();
        let mut n = 0u64;
        let filename = loop {
            if n >= MAX_GITHUB_RETRY_ATTEMPTS {
                return Err(anyhow::anyhow!(
                    "Exceeded maximum report filename attempts ({MAX_GITHUB_RETRY_ATTEMPTS})"
                ));
            }

            let candidate = if n == 0 {
                format!("{base_name}_{timestamp}.md")
            } else {
                format!("{base_name}_{timestamp}_{n}.md")
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

            if exists {
                n += 1;
                continue;
            }

            let message = format!("zbobr: store report {candidate} for task # {task_id}");
            let encoded = BASE64.encode(content.as_bytes());

            let mut body = serde_json::json!({
                "message": message,
                "content": encoded,
            });
            if let Some(branch) = reports_branch {
                body["branch"] = serde_json::Value::String(branch.to_string());
            }

            let result = {
                let mut attempt = 0u64;
                loop {
                    attempt += 1;
                    match self
                        .octocrab
                        .put::<serde_json::Value, _, _>(
                            format!("/repos/{owner}/{repo}/contents/{path}"),
                            Some(&body),
                        )
                        .await
                    {
                        Ok(value) => break Ok(value),
                        Err(e)
                            if attempt < MAX_GITHUB_RETRY_ATTEMPTS
                                && is_transient_octocrab_error(&e) =>
                        {
                            tracing::warn!(
                                "Transient GitHub error while creating report file {candidate} (attempt {attempt}/{max}): {}",
                                format_octocrab_error(&e),
                                max = MAX_GITHUB_RETRY_ATTEMPTS
                            );
                            tokio::time::sleep(Duration::from_millis(250 * attempt)).await;
                            continue;
                        }
                        Err(e) => break Err(e),
                    }
                }
            };

            match result {
                Ok(_) => break candidate,
                Err(e) if is_conflict_octocrab_error(&e) => {
                    tracing::info!(
                        "Report filename conflict for {candidate}, retrying with a new name"
                    );
                    n += 1;
                    continue;
                }
                Err(e) => return Err(octocrab_to_anyhow(e)),
            }
        };

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

    async fn list_tasks(&self) -> anyhow::Result<Vec<Box<dyn TaskWeak>>> {
        self.inner.await_all_cooling().await;

        let (owner, repo) = self.inner.parse_repo()?;

        let instance_label = format!(
            "{}{}",
            INSTANCE_LABEL_PREFIX, self.inner.backend_config.instance
        );

        let issues: Vec<IssueResponse> =
            if let Some(usernames) = self.inner.backend_config.allowed_usernames.as_deref() {
                let mut all_issues: Vec<IssueResponse> = Vec::new();
                for username in usernames {
                    let params = vec![
                        ("state", "open".to_string()),
                        ("per_page", "100".to_string()),
                        ("labels", instance_label.clone()),
                        ("creator", username.clone()),
                    ];
                    let mut user_issues: Vec<IssueResponse> = retry_github("list issues", || {
                        self.inner
                            .octocrab
                            .get(format!("/repos/{owner}/{repo}/issues"), Some(&params))
                    })
                    .await?;
                    all_issues.append(&mut user_issues);
                }
                all_issues
            } else {
                let params = vec![
                    ("state", "open".to_string()),
                    ("per_page", "100".to_string()),
                    ("labels", instance_label),
                ];
                retry_github("list issues", || {
                    self.inner
                        .octocrab
                        .get(format!("/repos/{owner}/{repo}/issues"), Some(&params))
                })
                .await?
            };

        let mut result: Vec<Box<dyn TaskWeak>> = Vec::new();
        for issue in issues {
            let id = issue.number;
            // Reuse list payload as the saved snapshot until a caller asks for refresh.
            match self.inner.hydrate_issue_to_task(issue) {
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
        let mut init_params = HashMap::new();
        let state_str = state.to_string();
        if !state_str.is_empty() {
            init_params.insert(PARAM_STATE.to_string(), state_str);
        }
        let body = serialize_description_full(
            description,
            &init_params,
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

        // Sync pipeline/pause labels for the initial state
        if let Err(e) = self
            .inner
            .apply_pipeline_and_pause_labels(issue.number, &state, &[])
            .await
        {
            tracing::warn!(
                "Task #{}: failed to sync labels after create: {e}",
                issue.number
            );
        }

        Ok(issue.number)
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
    use std::sync::Once;

    use zbobr_api::{
        Secret,
        task::{ContextRecord, ContextRecordType, Pipeline, Stage, StageContext, StageInfo},
    };

    use super::*;
    use crate::separator::{PARAMETERS_SEPARATOR, serialize_description_full};

    fn init_rustls() {
        static RUSTLS_INIT: Once = Once::new();
        RUSTLS_INIT.call_once(|| {
            let _ = rustls::crypto::ring::default_provider().install_default();
        });
    }

    fn make_issue_with_params(key: &str, value: &str) -> IssueResponse {
        let body = format!("desc{PARAMETERS_SEPARATOR}{key}: {value}\n");
        IssueResponse {
            number: 1,
            title: "test".to_string(),
            body: Some(body),
            state: "open".to_string(),
        }
    }

    fn make_config() -> ZbobrTaskBackendGithubConfig {
        init_rustls();
        ZbobrTaskBackendGithubConfig {
            instance: "default".to_string(),
            timezone: None,
            github_repo: "org/repo".to_string(),
            github_token: Secret::value("test-token"),
            reports_branch: Some("reports".to_string()),
            reports_path: Some("reports".to_string()),
            allowed_usernames: None,
            default_max_stage_count: zbobr_api::task::DEFAULT_MAX_STAGE_COUNT,
        }
    }

    fn make_backend() -> ZbobrTaskBackendGithubImpl {
        init_rustls();
        let config = make_config();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _guard = rt.enter();
        ZbobrTaskBackendGithubImpl::from_config(config).unwrap()
    }

    fn issue_to_task(issue: IssueResponse) -> Task {
        let backend = make_backend();
        backend.issue_to_task(issue).unwrap()
    }

    #[test]
    fn issue_to_task_reads_pause_from_params() {
        let issue = make_issue_with_params(PARAM_FLAG_PAUSE, PARAM_FLAG_VALUE_TRUE);
        let task = issue_to_task(issue);
        assert!(task.go_pause);
        assert!(!task.confirm);
    }

    #[test]
    fn issue_to_task_reads_confirm_from_params() {
        let issue = make_issue_with_params(PARAM_FLAG_CONFIRM, PARAM_FLAG_VALUE_TRUE);
        let task = issue_to_task(issue);
        assert!(!task.go_pause);
        assert!(task.confirm);
    }

    #[test]
    fn task_to_string_params_includes_flags_when_set() {
        use zbobr_api::task::{State, TaskContext};
        let task = Task {
            id: 1,
            title: "t".to_string(),
            description: "d".to_string(),
            state: State::Done,
            work_branch: None,
            pr_url: None,
            context: TaskContext::default(),
            signal: None,
            stack: vec![],
            status: None,
            go_pause: true,
            confirm: true,
            pipeline_run_id: 0,
            stage_count: 0,
            max_stage_count: zbobr_api::task::DEFAULT_MAX_STAGE_COUNT,
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

    #[test]
    fn hydrate_issue_to_task_restores_bare_report_filenames_from_blob_urls() {
        let config = make_config();
        let context = TaskContext {
            stages: vec![StageContext {
                info: StageInfo {
                    instance: "default".to_string(),
                    pipeline: Pipeline::Main,
                    run_id: 1,
                    stage: Stage::new("working"),
                    tool: None,
                    model: None,
                    prompt_link: Some("prompt_main_1_working.md".to_string()),
                    output_link: Some("output_main_1_working.md".to_string()),
                    timestamp: "2025-01-01T00:00:00Z".parse().unwrap(),
                },
                records: vec![ContextRecord {
                    id: 6,
                    record_type: ContextRecordType::Success,
                    brief: "done".to_string(),
                    report_link: Some("report_main_1_working.md".to_string()),
                }],
            }],
        };
        let report_prefix = "https://github.com/org/repo/blob/reports/reports/task_1/".to_string();
        let body = serialize_description_full(
            "desc",
            &HashMap::new(),
            &None,
            &context,
            &[],
            Some(&|filename| format!("{report_prefix}{filename}")),
        );
        let issue = IssueResponse {
            number: 1,
            title: "test".to_string(),
            body: Some(body),
            state: "open".to_string(),
        };

        let mut task = issue_to_task(issue);
        ZbobrTaskBackendGithubImpl::normalize_task_report_links_for_config(&config, &mut task)
            .unwrap();
        let stage = &task.context.stages[0];

        assert_eq!(
            stage.info.prompt_link.as_deref(),
            Some("prompt_main_1_working.md")
        );
        assert_eq!(
            stage.info.output_link.as_deref(),
            Some("output_main_1_working.md")
        );
        assert_eq!(
            stage.records[0].report_link.as_deref(),
            Some("report_main_1_working.md")
        );
    }

    #[test]
    fn normalize_task_report_links_rejects_non_blob_report_link_with_diagnostic() {
        let config = make_config();
        let mut task = Task {
            id: 1,
            title: "t".to_string(),
            description: "d".to_string(),
            state: State::Pending(zbobr_api::task::Pipeline::Main),
            work_branch: None,
            pr_url: None,
            context: TaskContext {
                stages: vec![StageContext {
                    info: StageInfo {
                        instance: "default".to_string(),
                        pipeline: Pipeline::Main,
                        run_id: 1,
                        stage: Stage::new("working"),
                        tool: None,
                        model: None,
                        prompt_link: None,
                        output_link: None,
                        timestamp: "2025-01-01T00:00:00Z".parse().unwrap(),
                    },
                    records: vec![ContextRecord {
                        id: 1,
                        record_type: ContextRecordType::Success,
                        brief: "done".to_string(),
                        report_link: Some("report_main_1_working.md".to_string()),
                    }],
                }],
            },
            signal: None,
            stack: vec![],
            status: None,
            go_pause: false,
            confirm: false,
            pipeline_run_id: 0,
            stage_count: 0,
            max_stage_count: zbobr_api::task::DEFAULT_MAX_STAGE_COUNT,
            closed: false,
            etag: None,
        };

        let err =
            ZbobrTaskBackendGithubImpl::normalize_task_report_links_for_config(&config, &mut task)
                .unwrap_err()
                .to_string();

        assert!(err.contains("stage[0].records[0].report_link"));
        assert!(err.contains("expected full GitHub blob URL"));
        assert!(err.contains("report_main_1_working.md"));
    }

    #[test]
    fn normalize_task_report_links_rejects_wrong_blob_prefix_with_diagnostic() {
        let config = make_config();
        let mut task = Task {
            id: 1,
            title: "t".to_string(),
            description: "d".to_string(),
            state: State::Pending(zbobr_api::task::Pipeline::Main),
            work_branch: None,
            pr_url: None,
            context: TaskContext {
                stages: vec![StageContext {
                    info: StageInfo {
                        instance: "default".to_string(),
                        pipeline: Pipeline::Main,
                        run_id: 1,
                        stage: Stage::new("working"),
                        tool: None,
                        model: None,
                        prompt_link: Some(
                            "https://github.com/org/repo/blob/main/reports/task_1/prompt.md"
                                .to_string(),
                        ),
                        output_link: None,
                        timestamp: "2025-01-01T00:00:00Z".parse().unwrap(),
                    },
                    records: vec![],
                }],
            },
            signal: None,
            stack: vec![],
            status: None,
            go_pause: false,
            confirm: false,
            pipeline_run_id: 0,
            stage_count: 0,
            max_stage_count: zbobr_api::task::DEFAULT_MAX_STAGE_COUNT,
            closed: false,
            etag: None,
        };

        let err =
            ZbobrTaskBackendGithubImpl::normalize_task_report_links_for_config(&config, &mut task)
                .unwrap_err()
                .to_string();

        assert!(err.contains("stage[0].prompt_link"));
        assert!(err.contains("expected full GitHub blob URL"));
        assert!(err.contains("blob/main"));
    }

    #[test]
    fn format_report_filename_timestamp_matches_expected_pattern() {
        let timestamp = format_report_filename_timestamp();

        assert_eq!(timestamp.len(), 25, "timestamp should be exactly 25 characters");
        assert_eq!(timestamp.chars().nth(10), Some('_'));
        assert_eq!(timestamp.chars().nth(19), Some('_'));
        assert!(matches!(timestamp.chars().nth(20), Some('+') | Some('-')));
    }
}
