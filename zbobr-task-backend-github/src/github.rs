use std::{collections::HashMap, time::Duration};

use crate::separator::{
    merge_concurrent_description_updates, parse_description_full, serialize_description_full,
};

use async_trait::async_trait;

use zbobr_dispatcher::backend::TaskBackend;
use zbobr_dispatcher::{Model, Parameter, Signal, Stage, Task, Tool, ZbobrError};

use crate::config::ZbobrTaskBackendGithubConfig;

/// Convert an octocrab error into a ZbobrError with detailed information.
fn octocrab_to_zbobr_error(e: octocrab::Error) -> ZbobrError {
    let error_msg = match e {
        octocrab::Error::GitHub { source, .. } => {
            format!(
                "GitHub API error: {} (status: {}) -- details: {:?}",
                source.message, source.status_code, source
            )
        }
        other => format!("GitHub API error: {:?}", other),
    };
    ZbobrError::GitHub(error_msg)
}

fn is_transient_octocrab_error(error: &octocrab::Error) -> bool {
    match error {
        octocrab::Error::GitHub { source, .. } => source.status_code.is_server_error(),
        _ => true,
    }
}

/// Generates a `retry` method on a struct that has an `octocrab` field.
/// The method retries transient GitHub API errors up to 3 times.
/// Closures capture `self.octocrab` from the surrounding scope (zero-arg).
macro_rules! impl_retry {
    ($type:ty) => {
        impl $type {
            async fn retry<T, F, Fut>(&self, op_name: &str, mut f: F) -> Result<T, ZbobrError>
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
                                    "Transient GitHub error during {op_name} (attempt {attempt}/3): {e}"
                                );
                                tokio::time::sleep(Duration::from_millis(250 * attempt)).await;
                                continue;
                            }
                            return Err(octocrab_to_zbobr_error(e));
                        }
                    }
                }
            }
        }
    };
}

impl_retry!(GitHubTaskBackend);

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
    user: Option<CommentUser>,
    body: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct CommentUser {
    login: String,
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

pub struct GitHubTaskBackend {
    backend_config: ZbobrTaskBackendGithubConfig,
    octocrab: octocrab::Octocrab,
}

impl GitHubTaskBackend {
    pub fn new(
        toml: Option<&crate::config::ZbobrTaskBackendGithubToml>,
        task_repo_override: Option<&str>,
    ) -> Result<Self, ZbobrError> {
        let backend_config = ZbobrTaskBackendGithubConfig::build(toml, task_repo_override);
        backend_config.validate()?;
        let octocrab = octocrab::Octocrab::builder()
            .personal_token(backend_config.github_token.clone())
            .build()
            .map_err(|e| ZbobrError::GitHub(format!("Failed to build octocrab client: {e}")))?;
        Ok(Self {
            backend_config,
            octocrab,
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

    fn parse_repo(&self) -> Result<(&str, &str), ZbobrError> {
        self.backend_config.parse_repo()
    }

    async fn find_stage_number(&self, stage: Stage) -> Result<Option<u64>, ZbobrError> {
        let title = stage.milestone_name();
        let stages = self.list_stages().await?;
        Ok(stages.into_iter().find(|(_, t)| t == title).map(|(n, _)| n))
    }

    /// Low-level: write the raw serialized task body.
    async fn update_task_description(&self, id: u64, description: &str) -> Result<(), ZbobrError> {
        let (owner, repo) = self.parse_repo()?;
        let url = format!("/repos/{owner}/{repo}/issues/{id}");
        let body = serde_json::json!({ "body": description });
        self.retry("update issue body", || {
            self.octocrab.patch(url.clone(), Some(&body))
        })
        .await
        .map(|_: serde_json::Value| ())?;
        Ok(())
    }

    /// Apply a stage change on a GitHub issue (update milestone).
    async fn apply_stage_change(&self, id: u64, stage: Stage) -> Result<(), ZbobrError> {
        let stage_number = self
            .find_stage_number(stage)
            .await?
            .ok_or_else(|| ZbobrError::GitHub(format!("Milestone '{}' not found", stage)))?;

        let (owner, repo) = self.parse_repo()?;
        let url = format!("/repos/{owner}/{repo}/issues/{id}");
        let body = serde_json::json!({ "milestone": stage_number });
        self.retry("set issue milestone", || {
            self.octocrab.patch(url.clone(), Some(&body))
        })
        .await
        .map(|_: serde_json::Value| ())?;
        Ok(())
    }

    /// Apply a signal change on a GitHub issue (remove old signal labels, add new one).
    async fn apply_signal_change(&self, id: u64, signal: Option<Signal>) -> Result<(), ZbobrError> {
        let (owner, repo) = self.parse_repo()?;

        // Remove all existing signal labels
        for sig in Signal::all() {
            let label = Self::signal_to_label(*sig);
            let _ = self
                .retry("remove signal label", || async {
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
            self.retry("add signal label", || async {
                self.octocrab
                    .issues(owner, repo)
                    .add_labels(id, &labels)
                    .await
            })
            .await?;
        }

        Ok(())
    }

    /// List all labels in the repository.
    async fn list_labels(&self) -> Result<Vec<String>, ZbobrError> {
        let (owner, repo) = self.parse_repo()?;
        let labels: Vec<octocrab::models::Label> = self
            .retry("list labels", || async {
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
    async fn create_label(
        &self,
        name: &str,
        color: &str,
        description: &str,
    ) -> Result<(), ZbobrError> {
        let (owner, repo) = self.parse_repo()?;
        self.retry("create label", || async {
            self.octocrab
                .issues(owner, repo)
                .create_label(name, color, description)
                .await
        })
        .await?;
        Ok(())
    }

    /// Update a label's color and description.
    async fn update_label(
        &self,
        name: &str,
        color: &str,
        description: &str,
    ) -> Result<(), ZbobrError> {
        let (owner, repo) = self.parse_repo()?;
        let url = format!("/repos/{owner}/{repo}/labels/{name}");
        let body = serde_json::json!({
            "color": color,
            "description": description,
        });
        self.retry("update label", || {
            self.octocrab.patch(url.clone(), Some(&body))
        })
        .await
        .map(|_: serde_json::Value| ())?;
        Ok(())
    }

    /// Ensure the task repo exists (used internally by setup).
    async fn ensure_task_repo_exists(&self) -> Result<(), ZbobrError> {
        let (owner, repo) = self.parse_repo()?;
        let exists = self
            .retry("check task repo exists", || {
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
            let result = self
                .retry("create org repo", || async {
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
                    self.retry("create user repo", || async {
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

    async fn list_stages(&self) -> Result<Vec<(u64, String)>, ZbobrError> {
        let (owner, repo) = self.parse_repo()?;
        let milestones: Vec<MilestoneResponse> = self
            .retry("list milestones", || {
                self.octocrab
                    .get(format!("/repos/{owner}/{repo}/milestones"), None::<&()>)
            })
            .await?;
        Ok(milestones
            .into_iter()
            .map(|m| (m.number, m.title))
            .collect())
    }

    async fn create_stage(&self, stage: Stage) -> Result<(), ZbobrError> {
        let (owner, repo) = self.parse_repo()?;
        let url = format!("/repos/{owner}/{repo}/milestones");
        let title = stage.milestone_name();
        let description = stage_description(stage);
        let body = serde_json::json!({
            "title": title,
            "description": description,
            "state": "open"
        });
        self.retry("create milestone", || {
            self.octocrab.post(url.clone(), Some(&body))
        })
        .await
        .map(|_: serde_json::Value| ())?;
        Ok(())
    }

    async fn delete_stage(&self, number: u64) -> Result<(), ZbobrError> {
        let (owner, repo) = self.parse_repo()?;
        let url = format!("/repos/{owner}/{repo}/milestones/{number}");
        let _response = self
            .retry("delete milestone", || {
                self.octocrab._delete(url.clone(), None::<&()>)
            })
            .await?;
        Ok(())
    }

    async fn setup(&self, force: bool) -> Result<(), ZbobrError> {
        tracing::info!(
            "Setting up GitHub repo: {} (force: {})",
            self.backend_config.task_repo,
            force
        );

        // Ensure the task repo exists
        self.ensure_task_repo_exists().await?;

        // Create stages
        let desired_stages = [
            Stage::Pending,
            Stage::GoPlanning,
            Stage::Planning,
            Stage::GoWorking,
            Stage::Working,
            Stage::GoReviewing,
            Stage::Reviewing,
            Stage::GoMerging,
            Stage::Merging,
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
        const TOOL_LABEL_COLOR: &str = "d4c5f9";
        const MODEL_LABEL_COLOR: &str = "bfd4f2";

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

        for tool in Tool::all() {
            let tool_label = format!("tool:{}", tool);
            let tool_desc = format!("Use {} tool", tool);
            if !existing_labels.contains(&tool_label) {
                tracing::info!("Creating label '{tool_label}'");
                self.create_label(&tool_label, TOOL_LABEL_COLOR, &tool_desc)
                    .await?;
            } else if force {
                tracing::info!("Updating label '{tool_label}' (force)");
                self.update_label(&tool_label, TOOL_LABEL_COLOR, &tool_desc)
                    .await?;
            } else {
                tracing::info!("Label '{tool_label}' already exists");
            }
        }

        for model in Model::all() {
            let model_label = format!("model:{}", model);
            let model_desc = format!("Use {} model", model);
            if !existing_labels.contains(&model_label) {
                tracing::info!("Creating label '{model_label}'");
                self.create_label(&model_label, MODEL_LABEL_COLOR, &model_desc)
                    .await?;
            } else if force {
                tracing::info!("Updating label '{model_label}' (force)");
                self.update_label(&model_label, MODEL_LABEL_COLOR, &model_desc)
                    .await?;
            } else {
                tracing::info!("Label '{model_label}' already exists");
            }
        }

        tracing::info!(
            "GitHub setup complete for {}",
            self.backend_config.task_repo
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
        let (description, params_map, plan, checklist) = parse_description_full(&body);

        let tool = issue.labels.iter().find_map(|l| {
            if let Some(name) = l.name.strip_prefix("tool:") {
                match name {
                    "copilot" => Some(Tool::Copilot),
                    "claude" => Some(Tool::Claude),
                    _ => None,
                }
            } else {
                None
            }
        });

        let model = issue.labels.iter().find_map(|l| {
            if let Some(name) = l.name.strip_prefix("model:") {
                name.parse::<Model>().ok()
            } else {
                None
            }
        });

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

        let done = issue
            .labels
            .iter()
            .any(|l| Self::label_to_signal(&l.name) == Some(Signal::Done));

        let signal = issue
            .labels
            .iter()
            .filter_map(|l| Self::label_to_signal(&l.name))
            .min();

        Task {
            id: issue.number,
            title: issue.title,
            description,
            plan,
            discussion: vec![],
            stage,
            tool,
            model,
            parameters,
            done,
            checklist,
            signal,
            etag: Some(body),
        }
    }
}

#[async_trait]
impl TaskBackend for GitHubTaskBackend {
    async fn get_task(&self, id: u64) -> Result<Task, ZbobrError> {
        let (owner, repo) = self.parse_repo()?;
        let issue: IssueResponse = self
            .retry("get issue", || {
                self.octocrab
                    .get(format!("/repos/{owner}/{repo}/issues/{id}"), None::<&()>)
            })
            .await?;

        Ok(Self::issue_to_task(issue))
    }

    async fn create_task(
        &self,
        title: &str,
        description: &str,
        stage: Stage,
        tool: Option<Tool>,
        model: Option<Model>,
        parameters: HashMap<Parameter, String>,
    ) -> Result<u64, ZbobrError> {
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
        let body = serialize_description_full(description, &params_text, "", &[]);

        let stage_number = self.find_stage_number(stage).await?;

        let mut labels = vec![];
        if let Some(t) = tool {
            labels.push(format!("tool:{}", t));
        }
        if let Some(m) = model {
            labels.push(format!("model:{}", m));
        }

        let issue = self
            .retry("create issue", || async {
                let issues = self.octocrab.issues(owner, repo);
                let mut builder = issues.create(title).body(body.clone());

                if let Some(n) = stage_number {
                    builder = builder.milestone(n);
                }

                if !labels.is_empty() {
                    builder = builder.labels(labels.clone());
                }

                builder.send().await
            })
            .await?;
        Ok(issue.number)
    }

    async fn close_task(&self, id: u64) -> Result<(), ZbobrError> {
        let (owner, repo) = self.parse_repo()?;
        let url = format!("/repos/{owner}/{repo}/issues/{id}");
        let body = serde_json::json!({ "state": "closed" });
        self.retry("close issue", || {
            self.octocrab.patch(url.clone(), Some(&body))
        })
        .await
        .map(|_: serde_json::Value| ())?;
        Ok(())
    }

    async fn is_task_closed(&self, id: u64) -> Result<bool, ZbobrError> {
        let (owner, repo) = self.backend_config.parse_repo()?;
        let issue: IssueResponse = self
            .retry("get issue state", || {
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
    ) -> Result<(), ZbobrError> {
        let task = self.get_task(id).await?;
        let original_stage = task.stage;
        let original_signal = task.signal;
        let expected_description = task.etag.clone().unwrap_or_else(|| {
            let string_params: HashMap<String, String> = task
                .parameters
                .iter()
                .map(|(k, v)| (k.name().to_string(), v.clone()))
                .collect();
            serialize_description_full(
                &task.description,
                &string_params,
                &task.plan,
                &task.checklist,
            )
        });

        let task = mutate(task);

        let string_params: HashMap<String, String> = task
            .parameters
            .iter()
            .map(|(k, v)| (k.name().to_string(), v.clone()))
            .collect();
        let new_description = serialize_description_full(
            &task.description,
            &string_params,
            &task.plan,
            &task.checklist,
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
            // Re-read to check for concurrent modifications
            let current_task = self.get_task(id).await?;
            let current_body = current_task.etag.unwrap_or_else(|| {
                let sp: HashMap<String, String> = current_task
                    .parameters
                    .iter()
                    .map(|(k, v)| (k.name().to_string(), v.clone()))
                    .collect();
                serialize_description_full(
                    &current_task.description,
                    &sp,
                    &current_task.plan,
                    &current_task.checklist,
                )
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

        Ok(())
    }

    async fn list_tasks_by_stage(
        &self,
        stage: Stage,
        tool: Option<Tool>,
    ) -> Result<Vec<Task>, ZbobrError> {
        let stage_number = match self.find_stage_number(stage).await? {
            Some(n) => n,
            None => return Ok(vec![]),
        };

        let (owner, repo) = self.parse_repo()?;
        let params = vec![
            ("milestone", stage_number.to_string()),
            ("state", "open".to_string()),
        ];

        let issues: Vec<IssueResponse> = self
            .retry("list issues", || {
                self.octocrab
                    .get(format!("/repos/{owner}/{repo}/issues"), Some(&params))
            })
            .await?;

        let mut tasks = Vec::new();
        for issue in issues {
            let task = Self::issue_to_task(issue);

            // Filter client-side: if tool filter is provided, only include tasks that:
            // - have no tool label (can be taken by anyone), OR
            // - have a matching tool label
            if let Some(filter_tool) = tool
                && let Some(t) = task.tool
                && t != filter_tool
            {
                continue;
            }

            tasks.push(task);
        }
        Ok(tasks)
    }

    async fn get_task_comments(&self, id: u64) -> Result<Vec<String>, ZbobrError> {
        let (owner, repo) = self.parse_repo()?;
        let comments: Vec<CommentResponse> = self
            .retry("list issue comments", || {
                self.octocrab.get(
                    format!("/repos/{owner}/{repo}/issues/{id}/comments"),
                    None::<&()>,
                )
            })
            .await?;

        Ok(comments
            .into_iter()
            .map(|c| {
                let user = c.user.map(|u| u.login).unwrap_or_else(|| "unknown".into());
                let body = c.body.unwrap_or_default();
                format!("{user}: {body}")
            })
            .collect())
    }

    async fn post_task_comment(
        &self,
        id: u64,
        body: &str,
        role: &str,
        hostname: &str,
    ) -> Result<(), ZbobrError> {
        let (owner, repo) = self.parse_repo()?;
        let formatted_body = format!("**[{role}@{hostname}]**\n\n{body}");
        self.retry("create issue comment", || async {
            self.octocrab
                .issues(owner, repo)
                .create_comment(id, &formatted_body)
                .await
        })
        .await?;
        Ok(())
    }

    async fn setup(&self, force: bool) -> Result<(), ZbobrError> {
        self.setup(force).await
    }

    async fn validate_connectivity(&self) -> Result<(), ZbobrError> {
        let (owner, repo) = self.parse_repo()?;
        let task_repo_exists = self
            .retry("check task repo", || {
                self.octocrab
                    .get::<RepoResponse, _, _>(format!("/repos/{owner}/{repo}"), None::<&()>)
            })
            .await
            .is_ok();
        if !task_repo_exists {
            return Err(ZbobrError::Config(format!(
                "task_repo '{owner}/{repo}' is not accessible on GitHub.\n  \
                 Check your task_repo setting and ensure the repository exists \
                 and your token has access to it."
            )));
        }

        Ok(())
    }

    fn debug_state(&self) -> String {
        format!("GitHubTaskBackend({})", self.backend_config.task_repo)
    }
}

/// Stage descriptions.
fn stage_description(stage: Stage) -> &'static str {
    match stage {
        Stage::Pending => "Task is under user's control, bot ignores it",
        Stage::GoPlanning => "Task must be taken by planner agent, any matching bot can take it",
        Stage::Planning => "Task is in planning, other bots ignore it",
        Stage::GoWorking => "Task must be taken by worker agent, any matching bot can take it",
        Stage::Working => "Task is in work, other bots ignore it",
        Stage::GoReviewing => "Task must be taken by reviewer agent, any matching bot can take it",
        Stage::Reviewing => "Task is in review, other bots ignore it",
        Stage::GoMerging => {
            "Task must be taken by merger agent to resolve conflicts, any matching bot can take it"
        }
        Stage::Merging => "Task is in merge conflict resolution, other bots ignore it",
    }
}
