# Implementation Plan: `allowed_users` Configuration

## Approach

Add an `allowed_users: Vec<String>` config field to the dispatcher. If not specified in TOML, default it to `[git_user_email]` after config build. Pass the list to `TaskBackend::list_tasks` via a new parameter. GitHub backend filters by issue creator login; FS backend ignores it.

## Key Design Decisions

- **Trait signature change**: `list_tasks(&self, allowed_users: &[String])` — empty slice means "no filter", which preserves backward compatibility for admin CLI commands.
- **GitHub filtering in-memory**: The GitHub API `creator` param only supports one user; in-memory filtering handles multiple users cleanly.
- **Default from `git_user_email`**: Applied after config build (not at macro level), since it depends on another config field.
- **FS backend ignores the param**: Per task description — "fs don't care about it".

## Affected Components

1. `zbobr-api/src/config.rs` — add `allowed_users` field
2. `zbobr-api/src/backend.rs` — update trait signature
3. `zbobr-task-backend-github/src/github.rs` — add `IssueUser` struct, filter in `list_tasks`
4. `zbobr-task-backend-fs/src/fs.rs` — update signature, ignore param
5. `zbobr-dispatcher/src/backend.rs`, `task.rs` — update wrapper impls
6. `zbobr-dispatcher/src/cli.rs` — pass `allowed_users` from config
7. `zbobr/src/commands.rs` — pass `&[]` for admin commands