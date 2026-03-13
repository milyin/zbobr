use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use async_trait::async_trait;
use zbobr_api::{
    Comment, CommentTag, CommentType, Model, Parameter, Role, Signal, Stage, Task, Tool,
    backend::TaskBackend,
};

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
    milestone: Option<IssueMilestone>,
    labels: Vec<IssueLabel>,
}

#[derive(Debug, serde::Deserialize)]
struct IssueMilestone {
    title: String,
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
struct MilestoneResponse {
    number: u64,
    title: String,
}

// ============================================================================
// GitHubTaskBackend
// ============================================================================

/// Duration to wait after writing to a GitHub issue before allowing reads.
/// This handles GitHub API eventual consistency for list/filter queries.
const COOLING_DURATION: Duration = Duration::from_secs(3);

pub struct ZbobrTaskBackendGithub {
    backend_config: ZbobrTaskBackendGithubConfig,
    octocrab: octocrab::Octocrab,
    cooling_deadlines: Mutex<HashMap<u64, tokio::time::Instant>>,
    /// Per-task mutexes to serialize concurrent read-modify-write cycles
    /// for the same task within this process.
    task_locks: std::sync::Mutex<HashMap<u64, Arc<tokio::sync::Mutex<()>>>>,
}

impl ZbobrTaskBackendGithub {
    pub fn new(
        toml: Option<crate::config::ZbobrTaskBackendGithubToml>,
        args: crate::config::ZbobrTaskBackendGithubArgs,
    ) -> anyhow::Result<Self> {
        let backend_config = <ZbobrTaskBackendGithubConfig as zbobr_api::config::Config>::build(
            toml,
            args,
            std::path::Path::new("."),
        );
        Self::from_config(backend_config)
    }

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
    fn signal_to_label(signal: Signal) -> String {
        format!("signal:{}", signal.name())
    }

    /// Parse a GitHub label string back to a Signal.
    fn label_to_signal(label: &str) -> Option<Signal> {
        label
            .strip_prefix("signal:")
            .and_then(|name| name.parse().ok())
    }

    /// Convert a flag name to its GitHub label representation.
    fn flag_to_label(name: &str) -> String {
        format!("flag:{}", name)
    }

    /// Parse a GitHub label string back to a flag name.
    fn label_to_flag(label: &str) -> Option<&str> {
        label.strip_prefix("flag:")
    }

    fn parse_repo(&self) -> anyhow::Result<(&str, &str)> {
        self.backend_config.parse_repo()
    }

    async fn find_stage_number(&self, stage: Stage) -> anyhow::Result<Option<u64>> {
        let title = stage.milestone_name();
        let stages = self.list_stages().await?;
        Ok(stages.into_iter().find(|(_, t)| t == title).map(|(n, _)| n))
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

    /// Apply a stage change on a GitHub issue (update milestone).
    async fn apply_stage_change(&self, id: u64, stage: Stage) -> anyhow::Result<()> {
        let stage_number = self
            .find_stage_number(stage)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Milestone '{}' not found", stage))?;

        let (owner, repo) = self.parse_repo()?;
        let url = format!("/repos/{owner}/{repo}/issues/{id}");
        let body = serde_json::json!({ "milestone": stage_number });
        retry_github("set issue milestone", || {
            self.octocrab.patch(url.clone(), Some(&body))
        })
        .await
        .map(|_: serde_json::Value| ())?;
        Ok(())
    }

    /// Apply a signal change on a GitHub issue (remove old signal labels, add new one).
    async fn apply_signal_change(&self, id: u64, signal: Option<Signal>) -> anyhow::Result<()> {
        let (owner, repo) = self.parse_repo()?;

        // Remove all existing signal labels
        for sig in Signal::all() {
            let label = Self::signal_to_label(*sig);
            let _ = retry_github("remove signal label", || async {
                self.octocrab
                    .issues(owner, repo)
                    .remove_label(id, &label)
                    .await
            })
            .await;
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

    /// Apply flag changes on a GitHub issue (sync conflict and pause labels).
    async fn apply_flag_change(
        &self,
        id: u64,
        conflict: bool,
        pause: bool,
        confirm: bool,
    ) -> anyhow::Result<()> {
        let (owner, repo) = self.parse_repo()?;

        for (flag_name, desired) in [
            ("conflict", conflict),
            ("pause", pause),
            ("confirm", confirm),
        ] {
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

    async fn list_stages(&self) -> anyhow::Result<Vec<(u64, String)>> {
        let (owner, repo) = self.parse_repo()?;
        let milestones: Vec<MilestoneResponse> = retry_github("list milestones", || {
            self.octocrab
                .get(format!("/repos/{owner}/{repo}/milestones"), None::<&()>)
        })
        .await?;
        Ok(milestones
            .into_iter()
            .map(|m| (m.number, m.title))
            .collect())
    }

    async fn create_stage(&self, stage: Stage) -> anyhow::Result<()> {
        let (owner, repo) = self.parse_repo()?;
        let url = format!("/repos/{owner}/{repo}/milestones");
        let title = stage.milestone_name();
        let description = stage_description(stage);
        let body = serde_json::json!({
            "title": title,
            "description": description,
            "state": "open"
        });
        retry_github("create milestone", || {
            self.octocrab.post(url.clone(), Some(&body))
        })
        .await
        .map(|_: serde_json::Value| ())?;
        Ok(())
    }

    async fn delete_stage(&self, number: u64) -> anyhow::Result<()> {
        let (owner, repo) = self.parse_repo()?;
        let url = format!("/repos/{owner}/{repo}/milestones/{number}");
        let _response = retry_github("delete milestone", || {
            self.octocrab._delete(url.clone(), None::<&()>)
        })
        .await?;
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

        // Create stages
        let desired_stages = [
            Stage::Pending,
            Stage::Preparing,
            Stage::Planning,
            Stage::Working,
            Stage::Reviewing,
            Stage::Testing,
            Stage::Merging,
            Stage::Done,
        ];
        let existing = self.list_stages().await?;
        let existing_titles: Vec<&str> = existing.iter().map(|(_, t)| t.as_str()).collect();

        for stage in &desired_stages {
            let title = stage.milestone_name();
            if existing_titles.contains(&title) {
                tracing::info!("Stage '{title}' already exists");
            } else {
                tracing::info!("Creating stage '{title}'");
                self.create_stage(*stage).await?;
            }
        }

        // Delete extra stages
        let desired_titles: Vec<&str> = desired_stages.iter().map(|s| s.milestone_name()).collect();
        for (number, title) in &existing {
            if !desired_titles.contains(&title.as_str()) {
                tracing::info!("Deleting stage '{title}'");
                self.delete_stage(*number).await?;
            }
        }

        // Create labels
        let existing_labels = self.list_labels().await?;

        const SIGNAL_LABEL_COLOR: &str = "5319e7";
        const FLAG_LABEL_COLOR: &str = "f9d0c4";

        for signal in Signal::all() {
            let signal_label = Self::signal_to_label(*signal);
            let signal_desc = format!("Signal: {}", signal.name());
            if !existing_labels.contains(&signal_label) {
                tracing::info!("Creating label '{signal_label}'");
                self.create_label(&signal_label, SIGNAL_LABEL_COLOR, &signal_desc)
                    .await?;
            } else if force {
                tracing::info!("Updating label '{signal_label}' (force)");
                self.update_label(&signal_label, SIGNAL_LABEL_COLOR, &signal_desc)
                    .await?;
            } else {
                tracing::info!("Label '{signal_label}' already exists");
            }
        }

        for flag_name in ["conflict", "pause", "confirm"] {
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

        tracing::info!(
            "GitHub setup complete for {}",
            self.backend_config.github_repo
        );
        Ok(())
    }

    /// Parse an IssueResponse into a Task.
    fn issue_to_task(issue: IssueResponse) -> Task {
        let stage = match issue.milestone.as_ref().map(|m| m.title.as_str()) {
            Some(t) => Stage::from_milestone_name(t).unwrap_or(Stage::Planning),
            _ => Stage::Planning,
        };

        let body = issue.body.unwrap_or_default();
        let (description, params_map, checklist) = parse_description_full(&body);

        let mut parameters = HashMap::new();
        if let Some(repo) = params_map.get(Parameter::DestinationRepository.name()) {
            parameters.insert(Parameter::DestinationRepository, repo.clone());
        }
        if let Some(branch) = params_map.get(Parameter::DestinationBranch.name()) {
            parameters.insert(Parameter::DestinationBranch, branch.clone());
        }
        if let Some(branch) = params_map.get(Parameter::WorkBranch.name()) {
            parameters.insert(Parameter::WorkBranch, branch.clone());
        }
        if let Some(url) = params_map.get(Parameter::PrUrl.name()) {
            parameters.insert(Parameter::PrUrl, url.clone());
        }

        let signal = issue
            .labels
            .iter()
            .filter_map(|l| Self::label_to_signal(&l.name))
            .min();

        let conflict = issue
            .labels
            .iter()
            .any(|l| Self::label_to_flag(&l.name) == Some("conflict"));

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
            stage,
            parameters,
            checklist,
            signal,
            conflict,
            pause,
            confirm,
            etag: Some(body),
        }
    }

    /// Record that an issue was just written to and needs a cooling period.
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
}

#[async_trait]
impl TaskBackend for ZbobrTaskBackendGithub {
    async fn get_task(&self, id: u64) -> anyhow::Result<Task> {
        self.await_cooling_for(id).await;
        self.fetch_task(id).await
    }

    async fn create_task(
        &self,
        title: &str,
        description: &str,
        stage: Stage,
        parameters: HashMap<Parameter, String>,
    ) -> anyhow::Result<u64> {
        let (owner, repo) = self.parse_repo()?;
        let mut params_text: HashMap<String, String> = HashMap::new();
        if let Some(v) = parameters.get(&Parameter::DestinationRepository) {
            params_text.insert(
                Parameter::DestinationRepository.name().to_string(),
                v.clone(),
            );
        }
        if let Some(v) = parameters.get(&Parameter::DestinationBranch) {
            params_text.insert(Parameter::DestinationBranch.name().to_string(), v.clone());
        }
        if let Some(v) = parameters.get(&Parameter::WorkBranch) {
            params_text.insert(Parameter::WorkBranch.name().to_string(), v.clone());
        }
        if let Some(v) = parameters.get(&Parameter::PrUrl) {
            params_text.insert(Parameter::PrUrl.name().to_string(), v.clone());
        }
        let body = serialize_description_full(description, &params_text, &[]);

        let stage_number = self.find_stage_number(stage).await?;

        let issue = retry_github("create issue", || async {
            let issues = self.octocrab.issues(owner, repo);
            let mut builder = issues.create(title).body(body.clone());

            if let Some(n) = stage_number {
                builder = builder.milestone(n);
            }

            builder.send().await
        })
        .await?;
        Ok(issue.number)
    }

    async fn close_task(&self, id: u64) -> anyhow::Result<()> {
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

    async fn is_task_closed(&self, id: u64) -> anyhow::Result<bool> {
        let (owner, repo) = self.backend_config.parse_repo()?;
        let issue: IssueResponse = retry_github("get issue state", || {
            self.octocrab
                .get(format!("/repos/{owner}/{repo}/issues/{id}"), None::<&()>)
        })
        .await?;
        Ok(issue.state == "closed")
    }

    async fn modify_task(
        &self,
        id: u64,
        mutate: Box<dyn FnOnce(Task) -> Task + Send>,
    ) -> anyhow::Result<()> {
        let lock = {
            let mut locks = self.task_locks.lock().unwrap();
            locks
                .entry(id)
                .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
                .clone()
        };
        let _guard = lock.lock().await;

        let task = self.fetch_task(id).await?;
        let original_stage = task.stage;
        let original_signal = task.signal;
        let original_conflict = task.conflict;
        let original_pause = task.pause;
        let original_confirm = task.confirm;
        let expected_description = task.etag.clone().unwrap_or_else(|| {
            let string_params: HashMap<String, String> = task
                .parameters
                .iter()
                .map(|(k, v)| (k.name().to_string(), v.clone()))
                .collect();
            serialize_description_full(&task.description, &string_params, &task.checklist)
        });

        let task = mutate(task);

        let string_params: HashMap<String, String> = task
            .parameters
            .iter()
            .map(|(k, v)| (k.name().to_string(), v.clone()))
            .collect();
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
            // Re-read to check for concurrent modifications
            let current_task = self.fetch_task(id).await?;
            let current_body = current_task.etag.unwrap_or_else(|| {
                let sp: HashMap<String, String> = current_task
                    .parameters
                    .iter()
                    .map(|(k, v)| (k.name().to_string(), v.clone()))
                    .collect();
                serialize_description_full(&current_task.description, &sp, &current_task.checklist)
            });
            if current_body != expected_desc {
                new_desc =
                    merge_concurrent_description_updates(&expected_desc, &current_body, &new_desc);
                expected_desc = current_body;
            }
        }

        // Apply stage change if it differs
        if task.stage != original_stage {
            self.apply_stage_change(id, task.stage).await?;
        }

        // Apply signal change if it differs
        if task.signal != original_signal {
            self.apply_signal_change(id, task.signal).await?;
        }

        // Apply flag changes if they differ
        if task.conflict != original_conflict
            || task.pause != original_pause
            || task.confirm != original_confirm
        {
            self.apply_flag_change(id, task.conflict, task.pause, task.confirm)
                .await?;
        }

        self.record_cooling(id);
        Ok(())
    }

    async fn list_tasks_by_stage(&self, stage: Stage) -> anyhow::Result<Vec<Task>> {
        self.await_all_cooling().await;
        let stage_number = match self.find_stage_number(stage).await? {
            Some(n) => n,
            None => return Ok(vec![]),
        };

        let (owner, repo) = self.parse_repo()?;
        let params = vec![
            ("milestone", stage_number.to_string()),
            ("state", "open".to_string()),
        ];

        let issues: Vec<IssueResponse> = retry_github("list issues", || {
            self.octocrab
                .get(format!("/repos/{owner}/{repo}/issues"), Some(&params))
        })
        .await?;

        let mut tasks = Vec::new();
        for issue in issues {
            let task = Self::issue_to_task(issue);

            tasks.push(task);
        }
        Ok(tasks)
    }

    async fn get_task_comments(&self, id: u64) -> anyhow::Result<Vec<Comment>> {
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

                // Split first line (tag) from body text so we can parse metadata.
                // split into first line (possible tag) plus the rest of the text
                let mut parts = body.splitn(2, '\n');
                let tag_line = parts.next().unwrap_or("");
                let rest = parts.next();

                // if the first line parses as a tag we drop it and keep the trailing
                // body.  otherwise we treat the entire comment as a simple request
                // and retain the original text verbatim.
                let (tag, text) = match tag_line.parse::<CommentTag>() {
                    Ok(t) => {
                        let body_text = rest.unwrap_or("").trim_start().to_string();
                        (t, body_text)
                    }
                    Err(_) => (
                        CommentTag::new(CommentType::Request, None, String::new(), None, None),
                        body.clone(),
                    ),
                };

                Comment {
                    comment_type: tag.comment_type,
                    timestamp,
                    role: tag.role,
                    hostname: tag.hostname,
                    tool: tag.tool,
                    model: tag.model,
                    text,
                }
            })
            .collect())
    }

    async fn post_task_comment(
        &self,
        id: u64,
        comment_type: CommentType,
        role: Option<Role>,
        hostname: &str,
        tool: Option<Tool>,
        model: Option<Model>,
        body: &str,
    ) -> anyhow::Result<()> {
        let (owner, repo) = self.parse_repo()?;

        let tag = CommentTag::new(comment_type, role, hostname.to_string(), tool, model);
        let formatted_body = format!("{}\n\n{}", tag, body);

        retry_github("create issue comment", || async {
            self.octocrab
                .issues(owner, repo)
                .create_comment(id, &formatted_body)
                .await
        })
        .await?;
        Ok(())
    }

    async fn setup(&self, force: bool) -> anyhow::Result<()> {
        self.setup(force).await
    }

    async fn validate_connectivity(&self) -> anyhow::Result<()> {
        let (owner, repo) = self.parse_repo()?;
        let task_repo_exists = retry_github("check task repo", || {
            self.octocrab
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
        format!("GitHubTaskBackend({})", self.backend_config.github_repo)
    }
}

/// Stage descriptions.
fn stage_description(stage: Stage) -> &'static str {
    match stage {
        Stage::Pending => "Task is pending dispatch",
        Stage::Preparing => "Task parameters are being set",
        Stage::Planning => "Task is in planning",
        Stage::Working => "Task is in work",
        Stage::Reviewing => "Task is in review",
        Stage::Testing => "Task is undergoing comprehensive testing",
        Stage::Merging => "Task is in merge conflict resolution",
        Stage::Done => "Task is complete",
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
            milestone: None,
            labels: vec![IssueLabel {
                name: "flag:confirm".to_string(),
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
    use std::str::FromStr;

    use zbobr_api::task::CommentTag;

    use super::*;

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
                CommentTag::new(CommentType::Request, None, String::new(), None, None),
                input.to_string(),
            ),
        }
    }

    #[test]
    fn test_parse_comment_tag_report_with_body() {
        let input = "// REPORT worker:localhost:claude-opus-4.6\n\nThis is the report body\nWith multiple lines";
        let (tag, body) = split_tag_body(input);
        let comment_type = tag.comment_type;
        let role = tag.role;
        let host = tag.hostname;
        let tool = tag.tool;
        let model = tag.model;

        assert_eq!(comment_type, CommentType::Report);
        assert_eq!(role, Some(Role::Worker));
        assert_eq!(host, "localhost");
        assert_eq!(tool, None);
        assert_eq!(model, Some(Model::from_str("claude-opus-4.6").unwrap()));
        assert_eq!(body, "This is the report body\nWith multiple lines");
    }

    #[test]
    fn test_parse_comment_tag_error_with_body() {
        let input = "// ERROR planner:skynet:gpt-4o\n\nAn error occurred";
        let (tag, body) = split_tag_body(input);
        let comment_type = tag.comment_type;
        let tool = tag.tool;
        let role = tag.role;
        let host = tag.hostname;
        let model = tag.model;

        assert_eq!(comment_type, CommentType::Error);
        assert_eq!(role, Some(Role::Planner));
        assert_eq!(host, "skynet");
        assert_eq!(tool, None);
        assert_eq!(model, Some(Model::from_str("gpt-4o").unwrap()));
        assert_eq!(body, "An error occurred");
    }

    #[test]
    fn test_parse_comment_tag_request_with_body() {
        let input = "// REQUEST\n\nThis is a user request";
        let (tag, body) = split_tag_body(input);
        let comment_type = tag.comment_type;
        let tool = tag.tool;
        let role = tag.role;
        let host = tag.hostname;
        let model = tag.model;

        assert_eq!(comment_type, CommentType::Request);
        assert_eq!(role, None);
        assert_eq!(host, "");
        assert_eq!(tool, None);
        assert_eq!(model, None);
        assert_eq!(body, "This is a user request");
    }

    #[test]
    fn test_parse_comment_tag_report_no_model() {
        let input = "// REPORT reviewer:host\n\nBody text";
        let (tag, body) = split_tag_body(input);
        let comment_type = tag.comment_type;
        let tool = tag.tool;
        let role = tag.role;
        let host = tag.hostname;
        let model = tag.model;

        assert_eq!(comment_type, CommentType::Report);
        assert_eq!(role, Some(Role::Reviewer));
        assert_eq!(host, "host");
        assert_eq!(tool, None);
        assert_eq!(model, None);
        assert_eq!(body, "Body text");
    }

    #[test]
    fn test_parse_comment_tag_no_tag_treated_as_request() {
        let input = "This is just text without a tag";
        let (tag, body) = split_tag_body(input);
        assert_eq!(tag.comment_type, CommentType::Request);
        assert_eq!(tag.role, None);
        assert_eq!(tag.hostname, "");
        assert_eq!(tag.tool, None);
        assert_eq!(tag.model, None);
        assert_eq!(body, "This is just text without a tag");
    }

    #[test]
    fn test_parse_comment_tag_bogus_tag_preserves_first_line() {
        let input = "// NOTATAG\nbody text";
        let (tag, body) = split_tag_body(input);
        assert_eq!(tag.comment_type, CommentType::Request);
        assert_eq!(tag.role, None);
        assert_eq!(tag.hostname, "");
        assert_eq!(tag.tool, None);
        assert_eq!(tag.model, None);
        assert_eq!(body, "// NOTATAG\nbody text");
    }

    #[test]
    fn test_parse_comment_tag_request_with_meta() {
        let input = "// REQUEST planner:skynet:gpt-4o\n\nPlease respond";
        let (tag, body) = split_tag_body(input);
        let comment_type = tag.comment_type;
        let tool = tag.tool;
        let role = tag.role;
        let host = tag.hostname;
        let model = tag.model;

        assert_eq!(comment_type, CommentType::Request);
        assert_eq!(role, Some(Role::Planner));
        assert_eq!(host, "skynet");
        assert_eq!(tool, None);
        assert_eq!(model, Some(Model::from_str("gpt-4o").unwrap()));
        assert_eq!(body, "Please respond");
    }

    #[test]
    fn test_parse_comment_tag_plan_with_body() {
        let input =
            "// PLAN planner:localhost:claude-opus-4.6\n\nStep 1: analyse\nStep 2: implement";
        let (tag, body) = split_tag_body(input);
        let comment_type = tag.comment_type;
        let role = tag.role;
        let tool = tag.tool;
        let host = tag.hostname;
        let model = tag.model;

        assert_eq!(comment_type, CommentType::Plan);
        assert_eq!(role, Some(Role::Planner));
        assert_eq!(host, "localhost");
        assert_eq!(tool, None);
        assert_eq!(model, Some(Model::from_str("claude-opus-4.6").unwrap()));
        assert_eq!(body, "Step 1: analyse\nStep 2: implement");
    }
}
