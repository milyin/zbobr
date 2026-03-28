
## What to change

Rename the `error: Option<String>` field to `status: Option<String>` in the `Task` struct in `zbobr-api/src/task.rs`. No backward compatibility needed — this is a clean rename throughout.

## Where it affects

- `zbobr-api/src/task.rs`: field declaration
- `zbobr-task-backend-fs/src/fs.rs`: private storage struct (also rename its `error` field to `status`) and all mapping code between storage struct ↔ `Task`
- `zbobr-dispatcher/src/task.rs`: all references to `task.error` in `set_error`, `set_state`, and tests

## Why

The field was named `error` but its purpose is to show the current status of why the task is paused (could be an error or a question). Renaming to `status` aligns it with the broader semantics introduced by this task.

## How to apply

Rename systematically. The Rust compiler will flag all missed call sites.
