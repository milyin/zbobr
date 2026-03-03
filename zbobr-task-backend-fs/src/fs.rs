use std::{collections::HashMap, path::PathBuf};

use anyhow::Context;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::fs;
use zbobr_api::{ChecklistItem, Comment, CommentAuthor, CommentType, Model, Parameter, Role, Stage, Task, Tool, backend::TaskBackend};

use crate::config::ZbobrTaskBackendFsConfig;

/// Serializable task structure for YAML storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskFile {
    id: u64,
    title: String,
    description: String,
    plan: String,
    stage: String,
    tool: Option<String>,
    model: Option<String>,
    parameters: HashMap<String, String>,
    #[serde(default)]
    conflict: bool,
    #[serde(default)]
    pause: bool,
    #[serde(default)]
    confirm: bool,
    checklist: Vec<ChecklistItem>,
    signal: Option<String>,
    closed: bool,
}

impl TaskFile {
    fn to_task(&self) -> anyhow::Result<Task> {
        let stage = Stage::from_milestone_name(&self.stage)
            .ok_or_else(|| anyhow::anyhow!("Invalid stage: {}", self.stage))?;

        let tool = self.tool.as_ref().map(|s| s.parse()).transpose()?;

        let model = self.model.as_ref().map(|s| s.parse()).transpose()?;

        let signal = self.signal.as_ref().map(|s| s.parse()).transpose()?;

        let parameters: Result<HashMap<Parameter, String>, String> = self
            .parameters
            .iter()
            .map(|(k, v)| {
                let param = match k.as_str() {
                    "destination_repository" => Ok(Parameter::DestinationRepository),
                    "destination_branch" => Ok(Parameter::DestinationBranch),
                    "work_branch" => Ok(Parameter::WorkBranch),
                    "pr_url" => Ok(Parameter::PrUrl),
                    _ => Err(format!("Unknown parameter: {}", k)),
                }?;
                Ok((param, v.clone()))
            })
            .collect();
        let parameters = parameters.map_err(|e| anyhow::anyhow!(e))?;

        Ok(Task {
            id: self.id,
            title: self.title.clone(),
            description: self.description.clone(),
            plan: self.plan.clone(),
            stage,
            tool,
            model,
            parameters,
            checklist: self.checklist.clone(),
            signal,
            conflict: self.conflict,
            pause: self.pause,
            confirm: self.confirm,
            etag: None,
        })
    }

    fn from_task(task: &Task, closed: bool) -> Self {
        Self {
            id: task.id,
            title: task.title.clone(),
            description: task.description.clone(),
            plan: task.plan.clone(),
            stage: task.stage.milestone_name().to_string(),
            tool: task.tool.map(|t| t.to_string()),
            model: task.model.as_ref().map(|m| m.to_string()),
            parameters: task
                .parameters
                .iter()
                .map(|(k, v)| (k.name().to_string(), v.clone()))
                .collect(),
            conflict: task.conflict,
            pause: task.pause,
            confirm: task.confirm,
            checklist: task.checklist.clone(),
            signal: task.signal.map(|s| s.name().to_string()),
            closed,
        }
    }
}

/// Parse tag from comment start: `// REPORT role:host:model` or `// ERROR role:host[:<model>]` or `// REQUEST`
/// Returns (CommentType, role_opt, host, model_opt, remaining_text)
fn parse_comment_tag(text: &str) -> (CommentType, Option<String>, String, Option<String>, String) {
    let trimmed = text.trim_start();

    // Check for tags like "// REPORT role:host:model" or "// REQUEST"
    if let Some(rest) = trimmed.strip_prefix("// ") {
        // Find the end of the tag line
        let tag_line_end = rest.find('\n').unwrap_or(rest.len());
        let tag_line = &rest[..tag_line_end];

        // Extract the tag type (first word before space or end of line)
        let tag_parts: Vec<&str> = tag_line.splitn(2, ' ').collect();
        if let Some(tag_str) = tag_parts.get(0) {
            if let Some(comment_type) = CommentType::from_str(&tag_str.to_lowercase()) {
                // For REPORT and ERROR, parse role:host:model format
                if comment_type != CommentType::Request {
                    if let Some(meta_part) = tag_parts.get(1) {
                        let meta_parts: Vec<&str> = meta_part.split(':').collect();
                        let role = meta_parts.get(0).map(|s| s.to_string());
                        let host = meta_parts.get(1).map(|s| s.to_string()).unwrap_or_default();
                        let model = meta_parts.get(2).map(|s| s.to_string());

                        // Extract body: skip tag line and the blank line that follows
                        let body_start = tag_line_end + 1;
                        let body = if body_start < rest.len() {
                            rest[body_start..].trim_start().to_string()
                        } else {
                            String::new()
                        };

                        return (comment_type, role, host, model, body);
                    }
                }

                // For REQUEST, extract text after the tag type
                let remaining = if let Some(body_part) = tag_parts.get(1) {
                    body_part.to_string()
                } else {
                    // If REQUEST tag has no inline text, check for text after tag line
                    let body_start = tag_line_end + 1;
                    if body_start < rest.len() {
                        rest[body_start..].trim_start().to_string()
                    } else {
                        String::new()
                    }
                };
                return (CommentType::Request, None, String::new(), None, remaining);
            }
        }
    }

    // No tag found, treat as request
    (CommentType::Request, None, String::new(), None, text.to_string())
}

/// Comments storage structure - stores structured comments as YAML.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct CommentsFile {
    comments: Vec<Comment>,
}

/// Filesystem-based task backend.
pub struct ZbobrTaskBackendFs {
    config: ZbobrTaskBackendFsConfig,
}

impl ZbobrTaskBackendFs {
    pub fn new(
        toml: Option<crate::config::ZbobrTaskBackendFsToml>,
        args: crate::config::ZbobrTaskBackendFsArgs,
        config_dir: &std::path::Path,
    ) -> anyhow::Result<Self> {
        let config =
            <ZbobrTaskBackendFsConfig as zbobr_api::config::Config>::build(toml, args, config_dir);
        Self::from_config(config)
    }

    pub fn from_config(config: ZbobrTaskBackendFsConfig) -> anyhow::Result<Self> {
        config.validate()?;
        Ok(Self { config })
    }

    /// Get the path to a task file.
    fn task_path(&self, id: u64) -> PathBuf {
        self.config.tasks_dir.join(format!("{}.yaml", id))
    }

    /// Get the path to a task's comments file.
    fn comments_path(&self, id: u64) -> PathBuf {
        self.config.tasks_dir.join(format!("{}.comments.yaml", id))
    }

    /// Get the path to the next ID counter file.
    fn next_id_path(&self) -> PathBuf {
        self.config.tasks_dir.join("next_id.txt")
    }

    /// Read and increment the next task ID counter.
    async fn get_next_id(&self) -> anyhow::Result<u64> {
        let path = self.next_id_path();

        // Ensure the tasks directory exists
        fs::create_dir_all(&self.config.tasks_dir)
            .await
            .context("Failed to create tasks directory")?;

        // Read current ID or start at 1
        let current_id = match fs::read_to_string(&path).await {
            Ok(content) => content.trim().parse::<u64>().unwrap_or(1),
            Err(_) => 1,
        };

        // Write next ID
        let next_id = current_id + 1;
        fs::write(&path, next_id.to_string())
            .await
            .context("Failed to write next ID")?;

        Ok(current_id)
    }

    /// Read a task file from disk.
    async fn read_task_file(&self, id: u64) -> anyhow::Result<TaskFile> {
        let path = self.task_path(id);
        let content = fs::read_to_string(&path)
            .await
            .with_context(|| format!("Failed to read task file {}", id))?;

        serde_yaml::from_str(&content).with_context(|| format!("Failed to parse task file {}", id))
    }

    /// Write a task file to disk.
    async fn write_task_file(&self, task_file: &TaskFile) -> anyhow::Result<()> {
        let path = self.task_path(task_file.id);

        // Ensure directory exists
        fs::create_dir_all(&self.config.tasks_dir)
            .await
            .context("Failed to create tasks directory")?;

        let yaml = serde_yaml::to_string(task_file).context("Failed to serialize task")?;

        fs::write(&path, yaml)
            .await
            .context("Failed to write task file")
    }

    /// Read comments from disk.
    /// Read raw comment strings from disk for backward compatibility.
    async fn read_comments_raw(&self, id: u64) -> anyhow::Result<Vec<String>> {
        let comments = self.read_comments_structured(id).await?;
        Ok(comments
            .into_iter()
            .map(|c| {
                // Convert structured comment back to string format for backward compatibility
                let mut result = String::new();
                match c.comment_type {
                    CommentType::Error => result.push_str("// ERROR"),
                    CommentType::Report => result.push_str("// REPORT"),
                    CommentType::Plan => result.push_str("// PLAN"),
                    CommentType::Request => result.push_str("// REQUEST"),
                }
                
                if let CommentAuthor::Role(role) = c.author {
                    result.push_str(&format!(" {}:{}", role, c.hostname));
                    if let Some(model) = c.model {
                        result.push_str(&format!(":{}", model));
                    }
                }
                
                result.push_str(&format!("\n\n{}", c.text));
                result
            })
            .collect())
    }

    /// Read structured comments from disk.
    async fn read_comments_structured(&self, id: u64) -> anyhow::Result<Vec<Comment>> {
        let path = self.comments_path(id);
        match fs::read_to_string(&path).await {
            Ok(content) => {
                let comments_file: CommentsFile =
                    serde_yaml::from_str(&content).context("Failed to parse comments file")?;
                Ok(comments_file.comments)
            }
            Err(_) => Ok(vec![]), // No comments file yet
        }
    }

    /// Read comments from disk (for backward compatibility with string format).
    async fn read_comments(&self, id: u64) -> anyhow::Result<Vec<String>> {
        self.read_comments_raw(id).await
    }

    /// Write structured comments to disk.
    async fn write_comments_structured(&self, id: u64, comments: Vec<Comment>) -> anyhow::Result<()> {
        let path = self.comments_path(id);

        let comments_file = CommentsFile { comments };
        let yaml = serde_yaml::to_string(&comments_file).context("Failed to serialize comments")?;

        fs::write(&path, yaml)
            .await
            .context("Failed to write comments file")
    }

    /// Write comments to disk (for backward compatibility).
    async fn write_comments(&self, id: u64, comments: Vec<String>) -> anyhow::Result<()> {
        // Convert string comments back to structured format (best effort)
        let structured_comments = comments
            .into_iter()
            .map(|text| {
                // Try to parse tag, fall back to User author
                let (comment_type, role_opt, host, model_str_opt, body_text) = parse_comment_tag(&text);
                let author = if let Some(role_str) = role_opt {
                    match role_str.parse::<Role>() {
                        Ok(role) => CommentAuthor::Role(role),
                        Err(_) => CommentAuthor::User,
                    }
                } else {
                    CommentAuthor::User
                };
                
                // Convert model string to Model enum if present
                let model = model_str_opt.and_then(|s| s.parse::<Model>().ok());
                
                Comment {
                    comment_type,
                    timestamp: format!("{:?}", std::time::SystemTime::now()),
                    author,
                    hostname: host,
                    model,
                    text: body_text,
                }
            })
            .collect();

        self.write_comments_structured(id, structured_comments).await
    }

    /// List all task files in the directory.
    async fn list_task_files(&self) -> anyhow::Result<Vec<u64>> {
        let mut task_ids = Vec::new();

        // Check if directory exists
        if !self.config.tasks_dir.exists() {
            return Ok(task_ids);
        }

        let mut entries = fs::read_dir(&self.config.tasks_dir)
            .await
            .context("Failed to read tasks directory")?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .context("Failed to read directory entry")?
        {
            let path = entry.path();
            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                // Match files like "123.yaml" but not "123.comments.yaml"
                if filename.ends_with(".yaml")
                    && !filename.contains(".comments.")
                    && let Some(id_str) = filename.strip_suffix(".yaml")
                    && let Ok(id) = id_str.parse::<u64>()
                {
                    task_ids.push(id);
                }
            }
        }

        Ok(task_ids)
    }
}

#[async_trait]
impl TaskBackend for ZbobrTaskBackendFs {
    async fn get_task(&self, id: u64) -> anyhow::Result<Task> {
        let task_file = self.read_task_file(id).await?;
        task_file.to_task()
    }

    async fn create_task(
        &self,
        title: &str,
        description: &str,
        stage: Stage,
        tool: Option<Tool>,
        model: Option<Model>,
        parameters: HashMap<Parameter, String>,
    ) -> anyhow::Result<u64> {
        let id = self.get_next_id().await?;

        let task = Task {
            id,
            title: title.to_string(),
            description: description.to_string(),
            plan: String::new(),
            stage,
            tool,
            model,
            parameters,
            checklist: vec![],
            signal: None,
            conflict: false,
            pause: false,
            confirm: false,
            etag: None,
        };

        let task_file = TaskFile::from_task(&task, false);
        self.write_task_file(&task_file).await?;

        tracing::info!("Created task {} in {}", id, self.config.tasks_dir.display());
        Ok(id)
    }

    async fn close_task(&self, id: u64) -> anyhow::Result<()> {
        let mut task_file = self.read_task_file(id).await?;
        task_file.closed = true;
        self.write_task_file(&task_file).await?;

        tracing::info!("Closed task {}", id);
        Ok(())
    }

    async fn is_task_closed(&self, id: u64) -> anyhow::Result<bool> {
        let task_file = self.read_task_file(id).await?;
        Ok(task_file.closed)
    }

    async fn modify_task(
        &self,
        id: u64,
        mutate: Box<dyn FnOnce(Task) -> Task + Send>,
    ) -> anyhow::Result<()> {
        // Read current task
        let task = self.get_task(id).await?;
        let was_closed = self.is_task_closed(id).await?;

        // Apply mutation
        let modified_task = mutate(task);

        // Write back
        let task_file = TaskFile::from_task(&modified_task, was_closed);
        self.write_task_file(&task_file).await?;

        tracing::debug!("Modified task {}", id);
        Ok(())
    }

    async fn list_tasks_by_stage(
        &self,
        stage: Stage,
        tool: Option<Tool>,
    ) -> anyhow::Result<Vec<Task>> {
        let task_ids = self.list_task_files().await?;
        let mut matching_tasks = Vec::new();

        for id in task_ids {
            // Skip if task file indicates closed
            let task_file = match self.read_task_file(id).await {
                Ok(tf) => tf,
                Err(_) => continue, // Skip files we can't read
            };

            if task_file.closed {
                continue;
            }

            let task = match task_file.to_task() {
                Ok(t) => t,
                Err(_) => continue,
            };

            // Filter by stage
            if task.stage != stage {
                continue;
            }

            // Filter by tool if specified
            if let Some(filter_tool) = tool {
                if let Some(task_tool) = task.tool {
                    if task_tool != filter_tool {
                        continue;
                    }
                } else {
                    // Task has no tool label, include it (can be taken by anyone)
                }
            }

            matching_tasks.push(task);
        }

        Ok(matching_tasks)
    }

    async fn get_task_comments(&self, id: u64) -> anyhow::Result<Vec<String>> {
        self.read_comments(id).await
    }

    async fn post_task_comment(
        &self,
        id: u64,
        body: &str,
        role: &str,
        hostname: &str,
    ) -> anyhow::Result<()> {
        let mut comments = self.read_comments(id).await?;
        let formatted_comment = format!("[{}@{}] {}", role, hostname, body);
        comments.push(formatted_comment);
        self.write_comments(id, comments).await?;

        tracing::debug!("Posted comment to task {}", id);
        Ok(())
    }

    async fn get_task_comments_structured(&self, id: u64) -> anyhow::Result<Vec<Comment>> {
        // Now that we store structured comments, return them directly
        self.read_comments_structured(id).await
    }

    async fn post_task_comment_structured(
        &self,
        id: u64,
        comment_type: CommentType,
        role: Option<Role>,
        hostname: &str,
        model: Option<Model>,
        body: &str,
    ) -> anyhow::Result<()> {
        let mut comments = self.read_comments_structured(id).await?;
        
        let new_comment = Comment {
            comment_type,
            timestamp: format!("{:?}", std::time::SystemTime::now()),
            author: match role {
                Some(r) => CommentAuthor::Role(r),
                None => CommentAuthor::User,
            },
            hostname: hostname.to_string(),
            model,
            text: body.to_string(),
        };
        
        comments.push(new_comment);
        self.write_comments_structured(id, comments).await?;
        
        tracing::debug!("Posted structured comment to task {}", id);
        Ok(())
    }

    async fn setup(&self, _force: bool) -> anyhow::Result<()> {
        // Create the tasks directory if it doesn't exist
        fs::create_dir_all(&self.config.tasks_dir)
            .await
            .context("Failed to create tasks directory")?;

        tracing::info!(
            "Filesystem backend setup complete: {}",
            self.config.tasks_dir.display()
        );
        Ok(())
    }

    async fn validate_connectivity(&self) -> anyhow::Result<()> {
        // Check if we can write to the tasks directory
        fs::create_dir_all(&self.config.tasks_dir)
            .await
            .with_context(|| {
                format!(
                    "Cannot access tasks directory '{}'",
                    self.config.tasks_dir.display()
                )
            })?;

        // Try to write a test file
        let test_path = self.config.tasks_dir.join(".test");
        fs::write(&test_path, "test").await.with_context(|| {
            format!(
                "Cannot write to tasks directory '{}'",
                self.config.tasks_dir.display()
            )
        })?;

        // Clean up test file
        let _ = fs::remove_file(&test_path).await;

        tracing::info!("Filesystem backend connectivity validated");
        Ok(())
    }

    fn debug_state(&self) -> String {
        format!(
            "FilesystemTaskBackend(tasks_dir: {})",
            self.config.tasks_dir.display()
        )
    }
}

#[cfg(test)]
mod parse_tests {
    use super::*;

    #[test]
    fn test_parse_comment_tag_report_with_body() {
        let input = "// REPORT worker:localhost:claude-opus\n\nThis is the report body\nWith multiple lines";
        let (comment_type, role, host, model, body) = parse_comment_tag(input);
        
        assert_eq!(comment_type, CommentType::Report);
        assert_eq!(role, Some("worker".to_string()));
        assert_eq!(host, "localhost");
        assert_eq!(model, Some("claude-opus".to_string()));
        assert_eq!(body, "This is the report body\nWith multiple lines");
    }

    #[test]
    fn test_parse_comment_tag_error_with_body() {
        let input = "// ERROR planner:skynet:gpt-4\n\nAn error occurred";
        let (comment_type, role, host, model, body) = parse_comment_tag(input);
        
        assert_eq!(comment_type, CommentType::Error);
        assert_eq!(role, Some("planner".to_string()));
        assert_eq!(host, "skynet");
        assert_eq!(model, Some("gpt-4".to_string()));
        assert_eq!(body, "An error occurred");
    }

    #[test]
    fn test_parse_comment_tag_request_with_body() {
        let input = "// REQUEST\n\nThis is a user request";
        let (comment_type, role, host, model, body) = parse_comment_tag(input);

        assert_eq!(comment_type, CommentType::Request);
        assert_eq!(role, None);
        assert_eq!(host, "");
        assert_eq!(model, None);
        assert_eq!(body, "This is a user request");
    }

    #[test]
    fn test_parse_comment_tag_reply_backward_compat() {
        // Old "// REPLY" tag should still parse as Request for backward compatibility
        let input = "// REPLY\n\nThis is a legacy reply";
        let (comment_type, role, host, model, body) = parse_comment_tag(input);

        assert_eq!(comment_type, CommentType::Request);
        assert_eq!(role, None);
        assert_eq!(host, "");
        assert_eq!(model, None);
        assert_eq!(body, "This is a legacy reply");
    }

    #[test]
    fn test_parse_comment_tag_report_no_model() {
        let input = "// REPORT reviewer:host\n\nBody text";
        let (comment_type, role, host, model, body) = parse_comment_tag(input);
        
        assert_eq!(comment_type, CommentType::Report);
        assert_eq!(role, Some("reviewer".to_string()));
        assert_eq!(host, "host");
        assert_eq!(model, None);
        assert_eq!(body, "Body text");
    }

    #[test]
    fn test_parse_comment_tag_no_tag_treated_as_request() {
        let input = "This is just text without a tag";
        let (comment_type, role, host, model, body) = parse_comment_tag(input);

        assert_eq!(comment_type, CommentType::Request);
        assert_eq!(role, None);
        assert_eq!(host, "");
        assert_eq!(model, None);
        assert_eq!(body, "This is just text without a tag");
    }

    #[test]
    fn test_parse_comment_tag_plan_with_body() {
        let input = "// PLAN planner:localhost:claude-opus\n\nStep 1: analyse\nStep 2: implement";
        let (comment_type, role, host, model, body) = parse_comment_tag(input);

        assert_eq!(comment_type, CommentType::Plan);
        assert_eq!(role, Some("planner".to_string()));
        assert_eq!(host, "localhost");
        assert_eq!(model, Some("claude-opus".to_string()));
        assert_eq!(body, "Step 1: analyse\nStep 2: implement");
    }
}
