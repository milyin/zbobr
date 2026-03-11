use std::path::{Path, PathBuf};

/// Represents the directory for a task in the workspaces directory.
/// Task directories follow the naming convention: `task-{task_id}`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskDir {
    path: PathBuf,
    task_id: u64,
}

impl TaskDir {
    /// Construct a TaskDir from a workspaces path and task ID.
    pub fn new(workspaces: &Path, task_id: u64) -> Self {
        let path = workspaces.join(format!("task-{task_id}"));
        Self { path, task_id }
    }

    /// Parse a TaskDir from an existing directory path.
    /// Returns an error if the directory name doesn't match the task- pattern.
    pub fn from_path(workspaces: &Path, path: &Path) -> anyhow::Result<Self> {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid path: cannot extract file name"))?;

        if !name.starts_with("task-") {
            return Err(anyhow::anyhow!(
                "Invalid task directory name: {} (must start with 'task-')",
                name
            ));
        }

        let task_id: u64 = name
            .strip_prefix("task-")
            .ok_or_else(|| anyhow::anyhow!("Failed to strip prefix from {}", name))?
            .parse()
            .map_err(|_| {
                anyhow::anyhow!(
                    "Invalid task directory name: {} (task ID must be a number)",
                    name
                )
            })?;

        Ok(Self::new(workspaces, task_id))
    }

    /// Get the task ID.
    pub fn task_id(&self) -> u64 {
        self.task_id
    }

    /// Get the path to the task directory.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get the name of the task directory (e.g., "task-123").
    pub fn dir_name(&self) -> String {
        format!("task-{}", self.task_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let workspaces = Path::new("/workspaces");
        let task_dir = TaskDir::new(workspaces, 123);

        assert_eq!(task_dir.task_id(), 123);
        assert_eq!(task_dir.path(), Path::new("/workspaces/task-123"));
        assert_eq!(task_dir.dir_name(), "task-123");
    }

    #[test]
    fn test_from_path_valid() {
        let workspaces = Path::new("/workspaces");
        let path = Path::new("/workspaces/task-456");

        let task_dir = TaskDir::from_path(workspaces, path).unwrap();
        assert_eq!(task_dir.task_id(), 456);
        assert_eq!(task_dir.path(), path);
    }

    #[test]
    fn test_from_path_invalid_prefix() {
        let workspaces = Path::new("/workspaces");
        let path = Path::new("/workspaces/invalid#456");

        assert!(TaskDir::from_path(workspaces, path).is_err());
    }

    #[test]
    fn test_from_path_invalid_id() {
        let workspaces = Path::new("/workspaces");
        let path = Path::new("/workspaces/task-abc");

        assert!(TaskDir::from_path(workspaces, path).is_err());
    }
}
