use std::{collections::HashMap, path::PathBuf, sync::Arc};

use anyhow::Context;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio::sync::{Mutex, OwnedMutexGuard};
use zbobr_api::{
    ChecklistItem, Comment, Model, StackEntry, Task, Tool,
    backend::{TaskBackend, TaskMut, TaskWeak},
};

use crate::config::ZbobrTaskBackendFsConfig;

/// Serializable task structure for YAML storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskFile {
    id: u64,
    title: String,
    description: String,
    /// Task state string (e.g. "READY", "main_PENDING", "main_working", "DONE")
    #[serde(default)]
    state: String,
    /// Legacy field — migrated to `state` on read.
    #[serde(default)]
    stage: Option<String>,

    // First-class routing fields (promoted from parameters)
    #[serde(default)]
    destination_repository: Option<String>,
    #[serde(default)]
    destination_branch: Option<String>,
    #[serde(default)]
    work_branch: Option<String>,

    parameters: HashMap<String, String>,
    #[serde(default)]
    pause: bool,
    #[serde(default)]
    confirm: bool,
    checklist: Vec<ChecklistItem>,
    signal: Option<String>,
    #[serde(default)]
    stack: Vec<StackEntry>,
    #[serde(default)]
    worktree_retries: u32,
    closed: bool,
}

impl TaskFile {
    fn to_task(&self) -> anyhow::Result<Task> {
        // Migrate legacy `stage` field to `state` if present
        let state = if self.state.is_empty() {
            if let Some(ref stage) = self.stage {
                stage.clone()
            } else {
                String::new()
            }
        } else {
            self.state.clone()
        };

        let pr_url = self.parameters.get("pr_url").cloned();

        Ok(Task {
            id: self.id,
            title: self.title.clone(),
            description: self.description.clone(),
            state,
            destination_repository: self.destination_repository.clone(),
            destination_branch: self.destination_branch.clone(),
            work_branch: self.work_branch.clone(),
            pr_url,
            checklist: self.checklist.clone(),
            signal: self.signal.clone(),
            stack: self.stack.clone(),
            pause: self.pause,
            confirm: self.confirm,
            worktree_retries: self.worktree_retries,
            etag: None,
        })
    }

    fn from_task(task: &Task, closed: bool) -> Self {
        Self {
            id: task.id,
            title: task.title.clone(),
            description: task.description.clone(),
            state: task.state.clone(),
            stage: None,
            destination_repository: task.destination_repository.clone(),
            destination_branch: task.destination_branch.clone(),
            work_branch: task.work_branch.clone(),
            parameters: {
                let mut p = HashMap::new();
                if let Some(ref url) = task.pr_url {
                    p.insert("pr_url".to_string(), url.clone());
                }
                p
            },
            pause: task.pause,
            confirm: task.confirm,
            checklist: task.checklist.clone(),
            signal: task.signal.clone(),
            stack: task.stack.clone(),
            worktree_retries: task.worktree_retries,
            closed,
        }
    }
}

/// Comments storage structure - stores structured comments as YAML.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct CommentsFile {
    comments: Vec<Comment>,
}

/// Filesystem-based task backend.
pub struct ZbobrTaskBackendFs {
    config: ZbobrTaskBackendFsConfig,
    /// Per-task locks for exclusive access.
    locks: Mutex<HashMap<u64, Arc<Mutex<()>>>>,
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
        Ok(Self {
            config,
            locks: Mutex::new(HashMap::new()),
        })
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

    /// Get or create a per-task lock.
    async fn task_lock(&self, id: u64) -> Arc<Mutex<()>> {
        let mut locks = self.locks.lock().await;
        locks
            .entry(id)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    /// Read and increment the next task ID counter.
    async fn get_next_id(&self) -> anyhow::Result<u64> {
        let path = self.next_id_path();

        fs::create_dir_all(&self.config.tasks_dir)
            .await
            .context("Failed to create tasks directory")?;

        let current_id = match fs::read_to_string(&path).await {
            Ok(content) => content.trim().parse::<u64>().unwrap_or(1),
            Err(_) => 1,
        };

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

        fs::create_dir_all(&self.config.tasks_dir)
            .await
            .context("Failed to create tasks directory")?;

        let yaml = serde_yaml::to_string(task_file).context("Failed to serialize task")?;

        fs::write(&path, yaml)
            .await
            .context("Failed to write task file")
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
            Err(_) => Ok(vec![]),
        }
    }

    /// Write structured comments to disk.
    async fn write_comments_structured(
        &self,
        id: u64,
        comments: Vec<Comment>,
    ) -> anyhow::Result<()> {
        let path = self.comments_path(id);

        let comments_file = CommentsFile { comments };
        let yaml = serde_yaml::to_string(&comments_file).context("Failed to serialize comments")?;

        fs::write(&path, yaml)
            .await
            .context("Failed to write comments file")
    }

    /// List all task files in the directory.
    async fn list_task_files(&self) -> anyhow::Result<Vec<u64>> {
        let mut task_ids = Vec::new();

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

    /// Read a task from disk (internal, returns Task directly).
    async fn read_task(&self, id: u64) -> anyhow::Result<Task> {
        let task_file = self.read_task_file(id).await?;
        task_file.to_task()
    }
}

// ---------------------------------------------------------------------------
// FsTaskWeak — read-only handle
// ---------------------------------------------------------------------------

struct FsTaskWeak {
    id: u64,
    backend: Arc<ZbobrTaskBackendFs>,
}

#[async_trait]
impl TaskWeak for FsTaskWeak {
    fn task_id(&self) -> u64 {
        self.id
    }

    async fn snapshot(&self) -> anyhow::Result<Task> {
        self.backend.read_task(self.id).await
    }

    async fn upgrade(&self) -> anyhow::Result<Box<dyn TaskMut>> {
        let lock = self.backend.task_lock(self.id).await;
        let guard = lock.try_lock_owned().map_err(|_| {
            anyhow::anyhow!(
                "Task {} is already exclusively locked by another TaskMut",
                self.id
            )
        })?;
        Ok(Box::new(FsTaskMut {
            id: self.id,
            backend: self.backend.clone(),
            _guard: guard,
        }))
    }

    async fn get_comments(&self) -> anyhow::Result<Vec<Comment>> {
        self.backend.read_comments_structured(self.id).await
    }
}

// ---------------------------------------------------------------------------
// FsTaskMut — exclusive mutable handle
// ---------------------------------------------------------------------------

struct FsTaskMut {
    id: u64,
    backend: Arc<ZbobrTaskBackendFs>,
    _guard: OwnedMutexGuard<()>,
}

#[async_trait]
impl TaskMut for FsTaskMut {
    fn task_id(&self) -> u64 {
        self.id
    }

    async fn snapshot(&self) -> anyhow::Result<Task> {
        self.backend.read_task(self.id).await
    }

    async fn modify_task(
        &self,
        mutate: Box<dyn FnOnce(Task) -> Task + Send>,
    ) -> anyhow::Result<()> {
        let task_file = self.backend.read_task_file(self.id).await?;
        let was_closed = task_file.closed;

        let task = self.backend.read_task(self.id).await?;
        let task = mutate(task);

        let task_file = TaskFile::from_task(&task, was_closed);
        self.backend.write_task_file(&task_file).await?;

        tracing::debug!("Modified task {}", self.id);
        Ok(())
    }

    async fn close(&self) -> anyhow::Result<()> {
        let mut task_file = self.backend.read_task_file(self.id).await?;
        task_file.closed = true;
        self.backend.write_task_file(&task_file).await?;

        tracing::info!("Closed task {}", self.id);
        Ok(())
    }

    async fn post_comment(
        &self,
        stage: &str,
        hostname: &str,
        tool: Option<Tool>,
        model: Option<Model>,
        body: &str,
        boundary: bool,
        hidden: bool,
    ) -> anyhow::Result<()> {
        let mut comments = self.backend.read_comments_structured(self.id).await?;

        let new_comment = Comment {
            timestamp: format!("{:?}", std::time::SystemTime::now()),
            stage: stage.to_string(),
            hostname: hostname.to_string(),
            tool,
            model,
            text: body.to_string(),
            boundary,
            hidden,
        };

        comments.push(new_comment);
        self.backend
            .write_comments_structured(self.id, comments)
            .await?;

        tracing::debug!("Posted structured comment to task {}", self.id);
        Ok(())
    }

    fn downgrade(self: Box<Self>) -> Box<dyn TaskWeak> {
        Box::new(FsTaskWeak {
            id: self.id,
            backend: self.backend.clone(),
        })
    }
}

#[async_trait]
impl TaskBackend for ZbobrTaskBackendFs {
    async fn get_task(&self, id: u64) -> anyhow::Result<Box<dyn TaskWeak>> {
        // Verify the task exists by reading it
        let _task = self.read_task(id).await?;
        // We need self to be wrapped in Arc for the handles.
        // This is handled by the caller wrapping ZbobrTaskBackendFs in Arc.
        // For now, we'll create a simple approach using a trick.
        anyhow::bail!("ZbobrTaskBackendFs must be wrapped in Arc and accessed via ArcTaskBackendFs")
    }

    async fn list_tasks(&self) -> anyhow::Result<Vec<Box<dyn TaskWeak>>> {
        anyhow::bail!("ZbobrTaskBackendFs must be wrapped in Arc and accessed via ArcTaskBackendFs")
    }

    async fn create_task(
        &self,
        title: &str,
        description: &str,
        state: &str,
    ) -> anyhow::Result<u64> {
        let id = self.get_next_id().await?;

        let task = Task {
            id,
            title: title.to_string(),
            description: description.to_string(),
            state: state.to_string(),
            destination_repository: None,
            destination_branch: None,
            work_branch: None,
            pr_url: None,
            checklist: vec![],
            signal: None,
            stack: vec![],
            pause: false,
            confirm: false,
            worktree_retries: 0,
            etag: None,
        };

        let task_file = TaskFile::from_task(&task, false);
        self.write_task_file(&task_file).await?;

        tracing::info!("Created task {} in {}", id, self.config.tasks_dir.display());
        Ok(id)
    }

    async fn setup(&self, _force: bool) -> anyhow::Result<()> {
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
        fs::create_dir_all(&self.config.tasks_dir)
            .await
            .with_context(|| {
                format!(
                    "Cannot access tasks directory '{}'",
                    self.config.tasks_dir.display()
                )
            })?;

        let test_path = self.config.tasks_dir.join(".test");
        fs::write(&test_path, "test").await.with_context(|| {
            format!(
                "Cannot write to tasks directory '{}'",
                self.config.tasks_dir.display()
            )
        })?;

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

/// Arc-wrapped FS backend that properly returns TaskWeak/TaskMut handles.
/// This is the primary way to use ZbobrTaskBackendFs.
#[derive(Clone)]
pub struct ArcTaskBackendFs {
    inner: Arc<ZbobrTaskBackendFs>,
}

impl ArcTaskBackendFs {
    pub fn new(backend: ZbobrTaskBackendFs) -> Self {
        Self {
            inner: Arc::new(backend),
        }
    }
}

#[async_trait]
impl TaskBackend for ArcTaskBackendFs {
    async fn get_task(&self, id: u64) -> anyhow::Result<Box<dyn TaskWeak>> {
        // Verify the task exists
        let _task = self.inner.read_task(id).await?;
        Ok(Box::new(FsTaskWeak {
            id,
            backend: self.inner.clone(),
        }))
    }

    async fn list_tasks(&self) -> anyhow::Result<Vec<Box<dyn TaskWeak>>> {
        let task_ids = self.inner.list_task_files().await?;
        let mut result: Vec<Box<dyn TaskWeak>> = Vec::new();

        for id in task_ids {
            let task_file = match self.inner.read_task_file(id).await {
                Ok(tf) => tf,
                Err(_) => continue,
            };

            if task_file.closed {
                continue;
            }

            let task = match task_file.to_task() {
                Ok(t) => t,
                Err(_) => continue,
            };

            // Filter out completed tasks
            if task.state == "DONE" {
                continue;
            }

            result.push(Box::new(FsTaskWeak {
                id,
                backend: self.inner.clone(),
            }));
        }

        Ok(result)
    }

    async fn create_task(
        &self,
        title: &str,
        description: &str,
        state: &str,
    ) -> anyhow::Result<u64> {
        self.inner.create_task(title, description, state).await
    }

    async fn setup(&self, force: bool) -> anyhow::Result<()> {
        self.inner.setup(force).await
    }

    async fn validate_connectivity(&self) -> anyhow::Result<()> {
        self.inner.validate_connectivity().await
    }

    fn debug_state(&self) -> String {
        self.inner.debug_state()
    }
}

#[cfg(test)]
mod parse_tests {
    use zbobr_api::task::CommentTag;

    #[test]
    fn test_parse_comment_tag_simple() {
        let input = "// planning:localhost";
        let tag: CommentTag = input.parse().unwrap();
        assert_eq!(tag.stage, "planning");
        assert_eq!(tag.hostname, "localhost");
        assert!(!tag.boundary);
        assert!(!tag.hidden);
    }

    #[test]
    fn test_parse_comment_tag_boundary() {
        let input = "// reviewing:skynet:boundary";
        let tag: CommentTag = input.parse().unwrap();
        assert_eq!(tag.stage, "reviewing");
        assert_eq!(tag.hostname, "skynet");
        assert!(tag.boundary);
        assert!(!tag.hidden);
    }

    #[test]
    fn test_parse_comment_tag_hidden() {
        let input = "// working:host:hidden";
        let tag: CommentTag = input.parse().unwrap();
        assert_eq!(tag.stage, "working");
        assert_eq!(tag.hostname, "host");
        assert!(!tag.boundary);
        assert!(tag.hidden);
    }

    #[test]
    fn test_parse_comment_tag_boundary_hidden() {
        let input = "// done:host:boundary:hidden";
        let tag: CommentTag = input.parse().unwrap();
        assert_eq!(tag.stage, "done");
        assert!(tag.boundary);
        assert!(tag.hidden);
    }

    #[test]
    fn test_comment_tag_roundtrip() {
        let tag = CommentTag::new("planning".into(), "localhost".into(), None, None, false, false);
        let s = tag.to_string();
        assert_eq!(s, "// planning:localhost");
        let parsed: CommentTag = s.parse().unwrap();
        assert_eq!(parsed, tag);

        let tag2 = CommentTag::new("reviewing".into(), "host".into(), None, None, true, true);
        let s2 = tag2.to_string();
        assert_eq!(s2, "// reviewing:host:boundary:hidden");
        let parsed2: CommentTag = s2.parse().unwrap();
        assert_eq!(parsed2, tag2);
    }
}
