# zbobr-task-backend-fs

Filesystem-based backend for zbobr task storage. This backend stores tasks as YAML files in a configurable directory on the local filesystem.

## Features

- **Simple file-based storage**: Each task is stored as a YAML file with a name like `{task_id}.yaml`
- **Human-readable format**: Tasks are stored in YAML format that can be easily edited manually if needed
- **Separate comment storage**: Comments are stored in separate files (`{task_id}.comments.yaml`)
- **Auto-incrementing IDs**: Task IDs are managed using a simple counter file (`next_id.txt`)
- **Open/closed state tracking**: Tasks track their closed state in the YAML file

## Configuration

Configure the filesystem backend in your `zbobr.toml`:

```toml
[tasks.fs]
tasks_dir = "./tasks"  # Optional: defaults to "./tasks"
```

Or set via environment variable:

```bash
export ZBOBR_TASKS_DIR=/path/to/tasks
```

Or pass as CLI argument when supported by the zbobr coordinator.

## File Structure

The tasks directory contains:

- `{id}.yaml` - Task data files
- `{id}.comments.yaml` - Comments for each task
- `next_id.txt` - Counter for generating new task IDs

### Task YAML Format

Each task file contains:

- `id`: Task ID (u64)
- `title`: Task title
- `description`: Detailed description
- `plan`: Planning notes
- `stage`: Current stage (e.g., "PENDING", "PLANNING", "WORKING")
- `tool`: Optional tool name (e.g., "copilot", "claude")
- `model`: Optional model name (e.g., "gpt-5-mini", "claude-sonnet-4.5")
- `parameters`: Key-value map of task parameters
- `done`: Boolean flag
- `checklist`: Array of checklist items with id, checked status, and text
- `signal`: Optional control signal (e.g., "stop", "done", "go_plan")
- `closed`: Boolean indicating if task is closed

## Usage

```rust
use zbobr_task_backend_fs::{FilesystemTaskBackend, ZbobrTaskBackendFsToml};

// Create backend instance
let toml_config = ZbobrTaskBackendFsToml {
    tasks_dir: Some("./my_tasks".to_string()),
};
let backend = FilesystemTaskBackend::new(Some(&toml_config), None)?;

// Use with zbobr dispatcher
// The backend implements the TaskBackend trait
```

## Differences from GitHub Backend

Unlike the GitHub backend which uses:

- Issues for tasks
- Milestones for stages
- Labels for signals/tools/models
- Issue comments for discussion

The filesystem backend:

- Uses plain YAML files for all task data
- Stores everything locally
- Has no network dependencies
- Suitable for testing, local development, or offline operation

## Thread Safety

Note: This backend does not implement file locking. For concurrent access from multiple processes, consider using the GitHub backend or implementing additional locking mechanisms.
