use std::{collections::HashMap, path::PathBuf, sync::Arc};

use anyhow::Context;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio::sync::{Mutex, OwnedMutexGuard};
use zbobr_api::{
    ChecklistItem, Comment, HistoryRecordType, Model, Signal, StackEntry, State, Task, Tool,
    backend::{TaskBackend, TaskMut, TaskWeak},
    classify_comment,
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
    signal: Option<Signal>,
    #[serde(default)]
    stack: Vec<StackEntry>,
    #[serde(default)]
    pipeline_run_id: u64,
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
            state: state.into(),
            destination_repository: self.destination_repository.clone(),
            destination_branch: self.destination_branch.clone(),
            work_branch: self.work_branch.clone(),
            pr_url,
            checklist: self.checklist.clone(),
            signal: self.signal.clone(),
            stack: self.stack.clone(),
            pause: self.pause,
            confirm: self.confirm,
            pipeline_run_id: self.pipeline_run_id,
            etag: None,
        })
    }

    fn from_task(task: &Task, closed: bool) -> Self {
        Self {
            id: task.id,
            title: task.title.clone(),
            description: task.description.clone(),
            state: task.state.to_string(),
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
            pipeline_run_id: task.pipeline_run_id,
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
        anyhow::ensure!(
            !name.contains(".."),
            "Invalid report name: {name}"
        );
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

    async fn read_report(&self, name: &str) -> anyhow::Result<String> {
        self.backend.read_report(self.id, name).await
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
        pipeline: &str,
        pipeline_run_id: u64,
        caller_pipeline: Option<&str>,
        caller_pipeline_run_id: Option<u64>,
        report_text: Option<&str>,
    ) -> anyhow::Result<()> {
        let report_name = if let Some(text) = report_text {
            let tag = match classify_comment(body) {
                HistoryRecordType::Success => "success",
                HistoryRecordType::Failure => "failure",
                _ => "report",
            };
            let base_name = format!("report_{pipeline}_{pipeline_run_id}_{stage}_{tag}");
            Some(self.backend.store_report(self.id, &base_name, text).await?)
        } else {
            None
        };

        let mut comments = self.backend.read_comments_structured(self.id).await?;

        let new_comment = Comment {
            timestamp: format!("{:?}", std::time::SystemTime::now()),
            stage: stage.to_string(),
            hostname: hostname.to_string(),
            tool,
            model,
            text: body.to_string(),
            pipeline: pipeline.to_string(),
            pipeline_run_id,
            caller_pipeline: caller_pipeline.map(str::to_string),
            caller_pipeline_run_id,
            report_name,
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
        state: State,
    ) -> anyhow::Result<u64> {
        let id = self.get_next_id().await?;

        let task = Task {
            id,
            title: title.to_string(),
            description: description.to_string(),
            state,
            destination_repository: None,
            destination_branch: None,
            work_branch: None,
            pr_url: None,
            checklist: vec![],
            signal: None,
            stack: vec![],
            pause: false,
            confirm: false,
            pipeline_run_id: 0,
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
mod parse_tests {
    use zbobr_api::task::CommentTag;

    #[test]
    fn test_parse_comment_tag_new_format() {
        let input = "// main:3:planning by localhost";
        let tag: CommentTag = input.parse().unwrap();
        assert_eq!(tag.pipeline, "main");
        assert_eq!(tag.pipeline_run_id, 3);
        assert_eq!(tag.stage, "planning");
        assert_eq!(tag.hostname, "localhost");
    }

    #[test]
    fn test_parse_comment_tag_with_for_suffix() {
        let input = "// sub:2:done by host:copilot:gpt-5-mini for main:1";
        let tag: CommentTag = input.parse().unwrap();
        assert_eq!(tag.pipeline, "sub");
        assert_eq!(tag.pipeline_run_id, 2);
        assert_eq!(tag.stage, "done");
        assert_eq!(tag.hostname, "host");
        assert_eq!(tag.caller_pipeline.as_deref(), Some("main"));
        assert_eq!(tag.caller_pipeline_run_id, Some(1));
    }

    #[test]
    fn test_comment_tag_roundtrip() {
        let tag = CommentTag::new("main".into(), 1, "planning".into(), "localhost".into(), None, None);
        let s = tag.to_string();
        assert_eq!(s, "// main:1:planning by localhost");
        let parsed: CommentTag = s.parse().unwrap();
        assert_eq!(parsed, tag);

        let tag2 = CommentTag::new("init".into(), 2, "reviewing".into(), "host".into(), None, None);
        let s2 = tag2.to_string();
        assert_eq!(s2, "// init:2:reviewing by host");
        let parsed2: CommentTag = s2.parse().unwrap();
        assert_eq!(parsed2, tag2);

        let tag3 = CommentTag::new("sub".into(), 3, "done".into(), "worker".into(), None, None)
            .with_caller("main".into(), 1);
        let s3 = tag3.to_string();
        assert_eq!(s3, "// sub:3:done by worker for main:1");
        let parsed3: CommentTag = s3.parse().unwrap();
        assert_eq!(parsed3, tag3);
    }
}

#[cfg(test)]
mod checklist_format_tests {
    use super::*;
    use zbobr_api::checklist_format::{parse_grouped_checklist, serialize_grouped_checklist};

    #[test]
    fn fs_backend_checklist_serialize_grouped() {
        let items = vec![
            ChecklistItem {
                id: "main__1__task1".to_string(),
                checked: false,
                text: "first run work".to_string(),
            },
            ChecklistItem {
                id: "main__1__task2".to_string(),
                checked: true,
                text: "first run done".to_string(),
            },
            ChecklistItem {
                id: "main__2__task1".to_string(),
                checked: false,
                text: "second run work".to_string(),
            },
        ];

        let serialized = serialize_grouped_checklist(&items);

        // Verify grouping headers
        assert!(serialized.contains("<!-- Run: main #1 -->"));
        assert!(serialized.contains("<!-- Run: main #2 -->"));

        // Verify display IDs are clean (no scope prefix)
        assert!(serialized.contains("- [ ] task1: first run work"));
        assert!(serialized.contains("- [x] task2: first run done"));
        assert!(serialized.contains("- [ ] task1: second run work"));

        // Scoped IDs should not appear
        assert!(!serialized.contains("main__1__"));
        assert!(!serialized.contains("main__2__"));
    }

    #[test]
    fn fs_backend_checklist_parse_grouped() {
        let text = "<!-- Run: main #1 -->\n\
            - [ ] task1: first run\n\
            - [x] task2: checked\n\
            \n\
            <!-- Run: main #2 -->\n\
            - [ ] task1: second run\n";

        let items = parse_grouped_checklist(text);

        assert_eq!(items.len(), 3);

        // Verify scoped IDs reconstructed
        assert_eq!(items[0].id, "main__1__task1");
        assert_eq!(items[0].checked, false);
        assert_eq!(items[0].text, "first run");

        assert_eq!(items[1].id, "main__1__task2");
        assert_eq!(items[1].checked, true);
        assert_eq!(items[1].text, "checked");

        assert_eq!(items[2].id, "main__2__task1");
        assert_eq!(items[2].checked, false);
        assert_eq!(items[2].text, "second run");
    }

    #[test]
    fn fs_backend_checklist_roundtrip() {
        let original_items = vec![
            ChecklistItem {
                id: "main__1__a".to_string(),
                checked: false,
                text: "work a".to_string(),
            },
            ChecklistItem {
                id: "init__2__b".to_string(),
                checked: true,
                text: "work b".to_string(),
            },
        ];

        let serialized = serialize_grouped_checklist(&original_items);
        let parsed = parse_grouped_checklist(&serialized);

        assert_eq!(parsed.len(), original_items.len());
        for (orig, parsed_item) in original_items.iter().zip(parsed.iter()) {
            assert_eq!(parsed_item.id, orig.id);
            assert_eq!(parsed_item.checked, orig.checked);
            assert_eq!(parsed_item.text, orig.text);
        }
    }

    #[test]
    fn fs_backend_checklist_unscoped_not_parsed() {
        let legacy_text = "- [ ] task1: legacy\n\
            - [x] task2: old\n";

        let items = parse_grouped_checklist(legacy_text);

        assert_eq!(items.len(), 0);
    }
}
