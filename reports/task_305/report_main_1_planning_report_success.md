# Zbobr Directory Configuration and Instance Analysis

## Project Overview
**Zbobr** is an AI-powered task dispatcher that manages GitHub issues through automated stages using pluggable AI tools (Claude, GitHub Copilot). The system processes issues through a workflow pipeline while maintaining separate working directories for different tasks.

**Key Files:**
- `/data/home/skynet/zdam/zbobr-dev/workspaces/task-305/zbobr/README.md` - Project documentation
- `/data/home/skynet/zdam/zbobr-dev/workspaces/task-305/zbobr/zbobr-api/src/config.rs` - Configuration structures
- `/data/home/skynet/zdam/zbobr-dev/workspaces/task-305/zbobr/zbobr-dispatcher/src/` - Main dispatcher code

## 1. Instance Configuration

### Where Instance is Defined

**File:** `/data/home/skynet/zdam/zbobr-dev/workspaces/task-305/zbobr/zbobr-api/src/config.rs` (lines 690-760)

**Struct Definition:**
```rust
#[derive(Clone)]
#[config_struct]
pub struct ZbobrDispatcherConfig {
    /// Name of this zbobr instance. Used to label tasks (zbobr:<instance>) so that
    /// multiple instances can run against the same repository without interfering.
    pub instance: String,
    /// Workspaces directory; each task gets a separate subdirectory.
    #[config(path)]
    pub workspaces: std::path::PathBuf,
    // ... other fields
}
```

**Default Values:**
```rust
impl Default for ZbobrDispatcherConfig {
    fn default() -> Self {
        Self {
            instance: "default".to_string(),
            workspaces: std::path::PathBuf::from("./workspaces"),
            // ...
        }
    }
}
```

### Where Instance is Used

1. **CLI Config Parsing**
   - File: `/data/home/skynet/zdam/zbobr-dev/workspaces/task-305/zbobr/zbobr-dispatcher/src/cli.rs` (line 601)
   - Usage: Retrieved from `self.zbobr.config().instance.clone()` and added to `StageContext.info`

2. **Task Context**
   - File: `/data/home/skynet/zdam/zbobr-dev/workspaces/task-305/zbobr/zbobr-dispatcher/src/cli.rs` (lines 601-623)
   - Purpose: Used in `StageContext` to label stages with instance info for task context tracking

3. **GitHub Labels (Instance Labeling)**
   - File: `/data/home/skynet/zdam/zbobr-dev/workspaces/task-305/zbobr/zbobr-task-backend-github/src/github.rs` (lines 557-559, 1260)
   - Usage: Creates GitHub labels like `zbobr:instance_name` to track which instance is handling a task

### Instance Configuration via TOML

**File:** `/data/home/skynet/zdam/zbobr-dev/workspaces/task-305/zbobr/zbobr/src/init.rs` (line 228)

Default initialization:
```rust
let config = RootConfigToml {
    dispatcher: Some(ZbobrDispatcherToml {
        instance: Some("default".into()),
        workspaces: Some(PathBuf::from("./workspaces")),
        // ...
    }),
    // ...
}
```

The `#[config_struct]` macro (defined in `/data/home/skynet/zdam/zbobr-dev/workspaces/task-305/zbobr/zbobr-macros/src/lib.rs`) auto-generates `ZbobrDispatcherArgs` and `ZbobrDispatcherToml` types that support:
- TOML file parsing
- CLI argument overrides via `--instance` flag
- Configuration merging

## 2. Directory Structure

### WorkspaceDirectory
**Base Path:** Configured in `ZbobrDispatcherConfig.workspaces` (default: `./workspaces`)

**Current Structure:**
```
workspaces/
├── task-1/
│   ├── repo/  (symlink or clone of target repository)
│   └── ... (worktree contents)
├── task-2/
│   ├── repo/  (symlink or clone of target repository)
│   └── ... (worktree contents)
├── task-N/
│   ├── repo/  (symlink or clone of target repository)
│   └── ... (worktree contents)
```

### TaskDir Structure

**File:** `/data/home/skynet/zdam/zbobr-dev/workspaces/task-305/zbobr/zbobr-dispatcher/src/task_dir.rs`

**Struct:**
```rust
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
}
```

**Key Methods:**
- `TaskDir::new(workspaces, task_id)` - Creates path: `{workspaces}/task-{task_id}`
- `TaskDir::from_path()` - Parses existing directory
- `task_id()` - Get the task ID
- `path()` - Get the full path
- `dir_name()` - Get directory name (e.g., "task-123")

### Workspace Path Construction

**Key Usage Locations:**

1. **In `/data/home/skynet/zdam/zbobr-dev/workspaces/task-305/zbobr/zbobr-dispatcher/src/lib.rs` (lines 385-391):**
```rust
pub async fn update_worktree(
    &self,
    identity: &zbobr_api::TaskIdentity,
) -> anyhow::Result<bool> {
    let repo_name = self.repo_backend.repo_name();
    let task_dir = TaskDir::new(&self.config.workspaces, identity.task_id);
    let workspace_path = task_dir.path().join(repo_name);  // e.g., workspaces/task-123/repo
    self.repo_backend
        .update_worktree(
            identity,
            &workspace_path,
            &self.config.git_user_name,
            &self.config.git_user_email,
        )
        .await
}
```

2. **In `/data/home/skynet/zdam/zbobr-dev/workspaces/task-305/zbobr/zbobr-dispatcher/src/cli.rs` (lines 1498-1501):**
```rust
let repo_name = zbobr.repo_backend().repo_name();
let work_dir = TaskDir::new(zbobr.config().workspaces.as_path(), task_id)
    .path()
    .join(repo_name);  // e.g., workspaces/task-123/repo
```

## 3. Repository Configuration

### WorktreeBackend Trait
**File:** `/data/home/skynet/zdam/zbobr-dev/workspaces/task-305/zbobr/zbobr-api/src/backend.rs` (lines 237-275)

**Key Methods:**
```rust
pub trait WorktreeBackend: Send + Sync {
    /// Return the full configured repository path (e.g. "owner/repo" or local path).
    fn repository(&self) -> &str;

    /// Return the configured base branch (e.g. "main").
    fn branch(&self) -> &str;

    /// Return the short name of the configured repository (last path component).
    /// Used to compute the workspace subdirectory name.
    fn repo_name(&self) -> &str;

    /// Prepare worktree for the task.
    async fn update_worktree(
        &self,
        identity: &TaskIdentity,
        workspace_path: &std::path::Path,
        git_user_name: &str,
        git_user_email: &str,
    ) -> anyhow::Result<bool>;
    
    // ... other methods
}
```

**Default Implementation:** 
In `/data/home/skynet/zdam/zbobr-dev/workspaces/task-305/zbobr/zbobr-dispatcher/src/lib.rs` and `/data/home/skynet/zdam/zbobr-dev/workspaces/task-305/zbobr/zbobr-dispatcher/src/task.rs`, `repo_name()` returns hardcoded `"repo"`.

### Repository Backends

Several backends support different repository sources:

1. **GitHub Backend:** 
   - File: `/data/home/skynet/zdam/zbobr-dev/workspaces/task-305/zbobr/zbobr-repo-backend-github/`
   - Provides: GitHub repository checkout/worktree management

2. **Filesystem Backend:**
   - File: `/data/home/skynet/zdam/zbobr-dev/workspaces/task-305/zbobr/zbobr-repo-backend-fs/`
   - Provides: Local filesystem repository management

## 4. Current Directory Path Pattern

**Current Pattern:**
```
{workspaces}/task-{task_id}/{repo_name}
```

**Example:**
```
./workspaces/task-123/repo
./workspaces/task-456/repo
```

## 5. Collision Risk for Multiple Instances

**Problem:** If multiple zbobr instances run with the same `workspaces` directory, they can collide:
- Both instances create `task-123` directory for different GitHub issues
- Both instances try to work in the same `workspaces/task-123/repo` path
- This leads to conflicts and race conditions

**Current Safeguard:** 
Only instance labeling in GitHub (via labels like `zbobr:instance_name`) but NO directory-level separation.

## 6. Proposed Solution: Instance-Based Directory Naming

### Implementation Points

**Option 1: Append instance to task directory name**
```
{workspaces}/task-{task_id}-{instance}/{repo_name}

Example:
./workspaces/task-123-default/repo
./workspaces/task-123-staging/repo  (different instance, same task ID)
./workspaces/task-456-default/repo
```

**Files to Modify:**
1. `/data/home/skynet/zdam/zbobr-dev/workspaces/task-305/zbobr/zbobr-dispatcher/src/task_dir.rs` - Update `TaskDir::new()` to include instance in directory name
2. `/data/home/skynet/zdam/zbobr-dev/workspaces/task-305/zbobr/zbobr-dispatcher/src/task_dir.rs` - Update `TaskDir::from_path()` to parse instance from directory name
3. `/data/home/skynet/zdam/zbobr-dev/workspaces/task-305/zbobr/zbobr-dispatcher/src/cleanup.rs` - Update cleanup logic to handle new naming
4. Update callers to pass instance when creating TaskDir (in lib.rs, cli.rs, task.rs)

**Option 2: Separate workspaces per instance**
```
{base_dir}/{instance}/workspaces/task-{task_id}/{repo_name}

Example:
./zbobr-data/default/workspaces/task-123/repo
./zbobr-data/staging/workspaces/task-123/repo
```

Would require modifying config to support instance-aware workspaces paths.

### Benefits
- **Isolation:** Complete separation between instances
- **Safety:** No file system collisions even with same task IDs
- **Clarity:** Directory structure shows which instance owns each task
- **Cleanup:** Instance-specific cleanup can work independently

## Summary

The zbobr project uses:
- **Instance Field:** `ZbobrDispatcherConfig.instance` (default: "default") - for labeling and identification
- **Workspace Root:** `ZbobrDispatcherConfig.workspaces` (default: "./workspaces") - base directory for all tasks
- **Task Directory Pattern:** `task-{task_id}` containing `{repo_name}/` subdirectory
- **Current Issue:** No instance-based separation in filesystem paths, only in GitHub labels

To prevent collisions with multiple instances, directory paths should incorporate the `instance` name, either as a suffix to task directories or as a prefix in the workspaces path configuration.