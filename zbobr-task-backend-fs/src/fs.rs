use std::collections::HashMap;
use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::config::ZbobrTaskBackendFsConfig;
use zbobr_dispatcher::backend::TaskBackend;
use zbobr_dispatcher::{ChecklistItem, Model, Parameter, Stage, Task, Tool, ZbobrError};

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
    done: bool,
    checklist: Vec<ChecklistItem>,
    signal: Option<String>,
    closed: bool,
}

impl TaskFile {
    fn to_task(&self) -> Result<Task, ZbobrError> {
        let stage = Stage::from_milestone_name(&self.stage)
            .ok_or_else(|| ZbobrError::Other(format!("Invalid stage: {}", self.stage)))?;

        let tool = self
            .tool
            .as_ref()
            .map(|s| s.parse())
            .transpose()
            .map_err(|e: String| ZbobrError::Other(e))?;

        let model = self
            .model
            .as_ref()
            .map(|s| s.parse())
            .transpose()
            .map_err(|e: String| ZbobrError::Other(e))?;

        let signal = self
            .signal
            .as_ref()
            .map(|s| s.parse())
            .transpose()
            .map_err(|e: String| ZbobrError::Other(e))?;

        let parameters: Result<HashMap<Parameter, String>, String> = self
            .parameters
            .into_iter()
            .map(|(k, v)| {
                let param = match k.as_str() {
                    "destination_repository" => Ok(Parameter::DestinationRepository),
                    "destination_branch" => Ok(Parameter::DestinationBranch),
                    "work_branch" => Ok(Parameter::WorkBranch),
                    "pr_url" => Ok(Parameter::PrUrl),
                    _ => Err(format!("Unknown parameter: {}", k)),
                }?;
                Ok((param, v))
            })
            .collect();
        let parameters = parameters.map_err(ZbobrError::Other)?;

        Ok(Task {
            id: self.id,
            title: self.title,
            description: self.description,
            plan: self.plan,
            discussion: vec![], // Will be loaded separately
            stage,
            tool,
            model,
            parameters,
            done: self.done,
            checklist: self.checklist,
            signal,
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
            done: task.done,
            checklist: task.checklist.clone(),
            signal: task.signal.map(|s| s.name().to_string()),
            closed,
        }
    }
}

/// Comments storage structure.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct CommentsFile {
    comments: Vec<String>,
}

/// Filesystem-based task backend.
pub struct FilesystemTaskBackend {
    config: ZbobrTaskBackendFsConfig,
}

impl FilesystemTaskBackend {
    pub fn new(
        toml: Option<&crate::config::ZbobrTaskBackendFsToml>,
        tasks_dir_override: Option<&str>,
    ) -> Result<Self, ZbobrError> {
        let config = ZbobrTaskBackendFsConfig::build(toml, tasks_dir_override);
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
    async fn get_next_id(&self) -> Result<u64, ZbobrError> {
        let path = self.next_id_path();

        // Ensure the tasks directory exists
        fs::create_dir_all(&self.config.tasks_dir)
            .await
            .map_err(|e| ZbobrError::Other(format!("Failed to create tasks directory: {}", e)))?;

        // Read current ID or start at 1
        let current_id = match fs::read_to_string(&path).await {
            Ok(content) => content.trim().parse::<u64>().unwrap_or(1),
            Err(_) => 1,
        };

        // Write next ID
        let next_id = current_id + 1;
        fs::write(&path, next_id.to_string())
            .await
            .map_err(|e| ZbobrError::Other(format!("Failed to write next ID: {}", e)))?;

        Ok(current_id)
    }

    /// Read a task file from disk.
    async fn read_task_file(&self, id: u64) -> Result<TaskFile, ZbobrError> {
        let path = self.task_path(id);
        let content = fs::read_to_string(&path)
            .await
            .map_err(|e| ZbobrError::Other(format!("Failed to read task file {}: {}", id, e)))?;

        serde_yaml::from_str(&content)
            .map_err(|e| ZbobrError::Other(format!("Failed to parse task file {}: {}", id, e)))
    }

    /// Write a task file to disk.
    async fn write_task_file(&self, task_file: &TaskFile) -> Result<(), ZbobrError> {
        let path = self.task_path(task_file.id);

        // Ensure directory exists
        fs::create_dir_all(&self.config.tasks_dir)
            .await
            .map_err(|e| ZbobrError::Other(format!("Failed to create tasks directory: {}", e)))?;

        let yaml = serde_yaml::to_string(task_file)
            .map_err(|e| ZbobrError::Other(format!("Failed to serialize task: {}", e)))?;

        fs::write(&path, yaml)
            .await
            .map_err(|e| ZbobrError::Other(format!("Failed to write task file: {}", e)))
    }

    /// Read comments from disk.
    async fn read_comments(&self, id: u64) -> Result<Vec<String>, ZbobrError> {
        let path = self.comments_path(id);
        match fs::read_to_string(&path).await {
            Ok(content) => {
                let comments_file: CommentsFile = serde_yaml::from_str(&content).map_err(|e| {
                    ZbobrError::Other(format!("Failed to parse comments file: {}", e))
                })?;
                Ok(comments_file.comments)
            }
            Err(_) => Ok(vec![]), // No comments file yet
        }
    }

    /// Write comments to disk.
    async fn write_comments(&self, id: u64, comments: Vec<String>) -> Result<(), ZbobrError> {
        let path = self.comments_path(id);

        let comments_file = CommentsFile { comments };
        let yaml = serde_yaml::to_string(&comments_file)
            .map_err(|e| ZbobrError::Other(format!("Failed to serialize comments: {}", e)))?;

        fs::write(&path, yaml)
            .await
            .map_err(|e| ZbobrError::Other(format!("Failed to write comments file: {}", e)))
    }

    /// List all task files in the directory.
    async fn list_task_files(&self) -> Result<Vec<u64>, ZbobrError> {
        let mut task_ids = Vec::new();

        // Check if directory exists
        if !self.config.tasks_dir.exists() {
            return Ok(task_ids);
        }

        let mut entries = fs::read_dir(&self.config.tasks_dir)
            .await
            .map_err(|e| ZbobrError::Other(format!("Failed to read tasks directory: {}", e)))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| ZbobrError::Other(format!("Failed to read directory entry: {}", e)))?
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
impl TaskBackend for FilesystemTaskBackend {
    async fn get_task(&self, id: u64) -> Result<Task, ZbobrError> {
        let task_file = self.read_task_file(id).await?;
        let mut task = task_file.to_task()?;

        // Load comments
        task.discussion = self.read_comments(id).await?;

        Ok(task)
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
        let id = self.get_next_id().await?;

        let task = Task {
            id,
            title: title.to_string(),
            description: description.to_string(),
            plan: String::new(),
            discussion: vec![],
            stage,
            tool,
            model,
            parameters,
            done: false,
            checklist: vec![],
            signal: None,
            etag: None,
        };

        let task_file = TaskFile::from_task(&task, false);
        self.write_task_file(&task_file).await?;

        tracing::info!("Created task {} in {}", id, self.config.tasks_dir.display());
        Ok(id)
    }

    async fn close_task(&self, id: u64) -> Result<(), ZbobrError> {
        let mut task_file = self.read_task_file(id).await?;
        task_file.closed = true;
        self.write_task_file(&task_file).await?;

        tracing::info!("Closed task {}", id);
        Ok(())
    }

    async fn is_task_closed(&self, id: u64) -> Result<bool, ZbobrError> {
        let task_file = self.read_task_file(id).await?;
        Ok(task_file.closed)
    }

    async fn modify_task(
        &self,
        id: u64,
        mutate: Box<dyn FnOnce(Task) -> Task + Send>,
    ) -> Result<(), ZbobrError> {
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
    ) -> Result<Vec<Task>, ZbobrError> {
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

    async fn get_task_comments(&self, id: u64) -> Result<Vec<String>, ZbobrError> {
        self.read_comments(id).await
    }

    async fn post_task_comment(
        &self,
        id: u64,
        body: &str,
        role: &str,
        hostname: &str,
    ) -> Result<(), ZbobrError> {
        let mut comments = self.read_comments(id).await?;
        let formatted_comment = format!("[{}@{}] {}", role, hostname, body);
        comments.push(formatted_comment);
        self.write_comments(id, comments).await?;

        tracing::debug!("Posted comment to task {}", id);
        Ok(())
    }

    async fn setup(&self, _force: bool) -> Result<(), ZbobrError> {
        // Create the tasks directory if it doesn't exist
        fs::create_dir_all(&self.config.tasks_dir)
            .await
            .map_err(|e| ZbobrError::Other(format!("Failed to create tasks directory: {}", e)))?;

        tracing::info!(
            "Filesystem backend setup complete: {}",
            self.config.tasks_dir.display()
        );
        Ok(())
    }

    async fn validate_connectivity(&self) -> Result<(), ZbobrError> {
        // Check if we can write to the tasks directory
        fs::create_dir_all(&self.config.tasks_dir)
            .await
            .map_err(|e| {
                ZbobrError::Config(format!(
                    "Cannot access tasks directory '{}': {}",
                    self.config.tasks_dir.display(),
                    e
                ))
            })?;

        // Try to write a test file
        let test_path = self.config.tasks_dir.join(".test");
        fs::write(&test_path, "test").await.map_err(|e| {
            ZbobrError::Config(format!(
                "Cannot write to tasks directory '{}': {}",
                self.config.tasks_dir.display(),
                e
            ))
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
