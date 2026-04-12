use std::{collections::HashMap, path::PathBuf, sync::Arc};

use anyhow::Context;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::{
    fs,
    sync::{Mutex, OwnedMutexGuard},
};
use zbobr_api::{
    Comment, Signal, StackEntry, State, StateOverrideRequest, Task,
    backend::{TaskBackend, TaskMut, TaskWeak},
    task::TaskContext,
};

use crate::config::ZbobrTaskBackendFsConfig;

/// Serializable task structure for YAML storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskFile {
    id: u64,
    title: String,
    description: String,
    #[serde(default)]
    state: State,
    /// Legacy field — migrated to `state` on read.
    #[serde(default)]
    stage: Option<String>,

    // First-class routing fields (promoted from parameters)
    #[serde(default)]
    work_branch: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pr_url: Option<String>,
    #[serde(default)]
    pause: bool,
    #[serde(default)]
    confirm: bool,
    #[serde(default)]
    context: TaskContext,
    signal: Option<Signal>,
    #[serde(default)]
    stack: Vec<StackEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    status: Option<String>,
    #[serde(default)]
    pipeline_run_id: u64,
    #[serde(default)]
    stage_count: u64,
    #[serde(default)]
    max_stage_count: u64,
    closed: bool,
}

impl TaskFile {
    fn to_task(&self) -> anyhow::Result<Task> {
        // Migrate legacy `stage` field to `state` if present
        let state = if self.state.is_empty() {
            if let Some(ref stage) = self.stage {
                State::from(stage.as_str())
            } else {
                State::Empty
            }
        } else {
            self.state.clone()
        };

        Ok(Task {
            id: self.id,
            title: self.title.clone(),
            description: self.description.clone(),
            state,
            work_branch: self.work_branch.clone(),
            pr_url: self.pr_url.clone(),

            context: self.context.clone(),
            signal: self.signal.clone(),
            stack: self.stack.clone(),
            status: self.status.clone(),
            go_pause: self.pause,
            confirm: self.confirm,
            pipeline_run_id: self.pipeline_run_id,
            stage_count: self.stage_count,
            max_stage_count: self.max_stage_count,
            closed: self.closed,
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
            work_branch: task.work_branch.clone(),
            pr_url: task.pr_url.clone(),
            pause: task.go_pause,
            confirm: task.confirm,
            context: task.context.clone(),
            signal: task.signal.clone(),
            stack: task.stack.clone(),
            status: task.status.clone(),
            pipeline_run_id: task.pipeline_run_id,
            stage_count: task.stage_count,
            max_stage_count: task.max_stage_count,
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
                let comments = if let Some(tz) = self.config.timezone {
                    comments_file
                        .comments
                        .into_iter()
                        .map(|mut c| {
                            c.timestamp = c.timestamp.with_timezone(&*tz);
                            c
                        })
                        .collect()
                } else {
                    comments_file.comments
                };
                Ok(comments)
            }
            Err(_) => Ok(vec![]),
        }
    }

    /// Get the reports directory for a task.
    fn reports_dir(&self, task_id: u64) -> PathBuf {
        self.config
            .tasks_dir
            .join("reports")
            .join(format!("task_{task_id}"))
    }

    /// Store a report file, deduplicating with `_N` suffix if needed.
    /// Returns the actual filename (without directory prefix).
    async fn store_report(
        &self,
        task_id: u64,
        base_name: &str,
        content: &str,
    ) -> anyhow::Result<String> {
        let dir = self.reports_dir(task_id);
        fs::create_dir_all(&dir)
            .await
            .context("Failed to create reports directory")?;

        let mut n = 0u32;
        let filename = loop {
            let candidate = if n == 0 {
                format!("{base_name}.md")
            } else {
                format!("{base_name}_{n}.md")
            };
            if !dir.join(&candidate).exists() {
                break candidate;
            }
            n += 1;
        };

        fs::write(dir.join(&filename), content)
            .await
            .context("Failed to write report file")?;

        tracing::debug!("Stored report for task {task_id}: {filename}");
        Ok(filename)
    }

    /// Read a report file by exact name.
    async fn read_report(&self, task_id: u64, name: &str) -> anyhow::Result<String> {
        anyhow::ensure!(!name.contains(".."), "Invalid report name: {name}");
        let path = self.reports_dir(task_id).join(name);
        fs::read_to_string(&path)
            .await
            .with_context(|| format!("Failed to read report file '{name}' for task {task_id}"))
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
            if let Some(filename) = path.file_name().and_then(|n| n.to_str())
                && filename.ends_with(".yaml")
                && !filename.contains(".comments.")
                && let Some(id_str) = filename.strip_suffix(".yaml")
                && let Ok(id) = id_str.parse::<u64>()
            {
                task_ids.push(id);
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
    saved_snapshot: Arc<std::sync::Mutex<Option<Task>>>,
}

#[async_trait]
impl TaskWeak for FsTaskWeak {
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
        let lock = self.backend.task_lock(self.id).await;
        let guard = lock.lock_owned().await;
        Ok(Box::new(FsTaskMut {
            id: self.id,
            backend: self.backend.clone(),
            saved_snapshot: self.saved_snapshot.clone(),
            _guard: guard,
        }))
    }

    async fn get_comments(&self) -> anyhow::Result<Vec<Comment>> {
        self.backend.read_comments_structured(self.id).await
    }

    async fn read_report(&self, name: &str) -> anyhow::Result<String> {
        self.backend.read_report(self.id, name).await
    }

    async fn pending_override(&self) -> anyhow::Result<Option<StateOverrideRequest>> {
        let task = self.snapshot(false).await?;
        if matches!(task.state, State::Pause) && !task.stack.is_empty() {
            return Ok(Some(StateOverrideRequest::Resume(Signal::Return)));
        }
        Ok(None)
    }
}

// ---------------------------------------------------------------------------
// FsTaskMut — exclusive mutable handle
// ---------------------------------------------------------------------------

struct FsTaskMut {
    id: u64,
    backend: Arc<ZbobrTaskBackendFs>,
    saved_snapshot: Arc<std::sync::Mutex<Option<Task>>>,
    _guard: OwnedMutexGuard<()>,
}

#[async_trait]
impl TaskMut for FsTaskMut {
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
        let task_file = self.backend.read_task_file(self.id).await?;
        let was_closed = task_file.closed;

        let task = self.backend.read_task(self.id).await?;
        let task = mutate(task);

        let task_file = TaskFile::from_task(&task, was_closed);
        self.backend.write_task_file(&task_file).await?;
        *self.saved_snapshot.lock().unwrap() = Some(task);

        tracing::debug!("Modified task {}", self.id);
        Ok(())
    }

    async fn close(&self) -> anyhow::Result<()> {
        let mut task_file = self.backend.read_task_file(self.id).await?;
        task_file.closed = true;
        self.backend.write_task_file(&task_file).await?;
        self.saved_snapshot.lock().unwrap().take();

        tracing::info!("Closed task {}", self.id);
        Ok(())
    }

    async fn store_report(&self, base_name: &str, content: &str) -> anyhow::Result<String> {
        self.backend.store_report(self.id, base_name, content).await
    }

    fn report_url(&self, filename: &str) -> String {
        filename.to_string()
    }

    fn downgrade(self: Box<Self>) -> Box<dyn TaskWeak> {
        Box::new(FsTaskWeak {
            id: self.id,
            backend: self.backend.clone(),
            saved_snapshot: self.saved_snapshot.clone(),
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
        state: State,
    ) -> anyhow::Result<u64> {
        let id = self.get_next_id().await?;

        let task = Task {
            id,
            title: title.to_string(),
            description: description.to_string(),
            state,
            work_branch: None,
            pr_url: None,

            context: TaskContext::default(),
            signal: None,
            stack: vec![],
            status: None,
            go_pause: false,
            confirm: false,
            pipeline_run_id: 0,
            stage_count: 0,
            max_stage_count: self.config.default_max_stage_count,
            closed: false,
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
        let task = self.inner.read_task(id).await?;
        Ok(Box::new(FsTaskWeak {
            id,
            backend: self.inner.clone(),
            saved_snapshot: Arc::new(std::sync::Mutex::new(Some(task))),
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
            if task.state.is_done() {
                continue;
            }

            result.push(Box::new(FsTaskWeak {
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
mod tests {
    use std::path::Path;

    use chrono::Timelike;
    use zbobr_api::task::FixedOffsetTz;

    use super::*;

    fn make_config_with_timezone(
        tasks_dir: PathBuf,
        tz: Option<FixedOffsetTz>,
    ) -> ZbobrTaskBackendFsConfig {
        ZbobrTaskBackendFsConfig {
            tasks_dir,
            timezone: tz,
            default_max_stage_count: zbobr_api::task::DEFAULT_MAX_STAGE_COUNT,
        }
    }

    async fn write_comment_fixture(dir: &Path, task_id: u64, timestamp_str: &str) {
        let yaml = format!(
            "comments:\n  - timestamp: \"{}\"\n    username: user\n    body: hello\n",
            timestamp_str
        );
        let path = dir.join(format!("{}.comments.yaml", task_id));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, yaml).unwrap();
    }

    #[tokio::test]
    async fn read_comments_converts_to_configured_timezone() {
        let dir = tempfile::tempdir().unwrap();
        write_comment_fixture(dir.path(), 1, "2025-01-01T12:00:00+00:00").await;

        let tz: FixedOffsetTz = "+03:00".parse().unwrap();
        let config = make_config_with_timezone(dir.path().to_path_buf(), Some(tz));
        let backend = ZbobrTaskBackendFs::from_config(config).unwrap();
        let comments = backend.read_comments_structured(1).await.unwrap();

        assert_eq!(comments.len(), 1);
        // The timestamp should be at UTC+3 = 15:00
        assert_eq!(comments[0].timestamp.offset().local_minus_utc(), 3 * 3600);
        assert_eq!(comments[0].timestamp.hour(), 15);
    }

    #[tokio::test]
    async fn read_comments_unchanged_when_no_timezone() {
        let dir = tempfile::tempdir().unwrap();
        write_comment_fixture(dir.path(), 1, "2025-01-01T12:00:00+00:00").await;

        let config = make_config_with_timezone(dir.path().to_path_buf(), None);
        let backend = ZbobrTaskBackendFs::from_config(config).unwrap();
        let comments = backend.read_comments_structured(1).await.unwrap();

        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].timestamp.offset().local_minus_utc(), 0);
    }
}
