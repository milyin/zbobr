# Implementation Plan: Instance Filtering

## Approach

Add a required `instance: String` field to the dispatcher config. Use it to:
1. Filter tasks by GitHub label `zbobr:<instance>` when listing tasks
2. Create/manage the `zbobr:<instance>` label during setup
3. Prefix the context stage title format with the instance name

The closest analog in the codebase is `allowed_usernames` in `ZbobrTaskBackendGithubConfig`, which filters tasks at the backend level. However, `instance` is a zbobr-core concept (not GitHub-specific), so it lives in `ZbobrDispatcherConfig` and is passed down to the backend via trait method parameters.

---

## Components

### 1. Config: Add `instance` to `ZbobrDispatcherConfig`
- Add `instance: String` as a required field to `ZbobrDispatcherConfig` in `zbobr-api/src/config.rs`
- CLI: `--instance <name>`, TOML: `instance = "main"`
- Required (no default) — zbobr must know its identity

### 2. `TaskBackend` trait: Pass instance as parameter
- Update `list_tasks()` signature to `list_tasks(&self, instance: &str)`
- Update `setup()` signature to `setup(&self, force: bool, instance: &str)`
- This keeps the instance concept at the zbobr level, not duplicated per backend config

### 3. GitHub backend: `list_tasks` — filter by instance label
- In `ZbobrTaskBackendGithub::list_tasks(instance)`, add `labels=zbobr:<instance>` as a GitHub API query parameter
- This is alongside existing filters (state, creator from allowed_usernames)
- The GitHub API supports label-based issue filtering natively

### 4. GitHub backend: `setup` — instance label lifecycle
- Create label `zbobr:<instance>` during setup
- **Without `--force`**: only create the current instance label; do not touch `zbobr:*` labels for other instances
- **With `--force`**: also delete any `zbobr:*` labels that do not match the current instance

### 5. Stage title: Add instance prefix
- Add `instance: String` field to `MdStageTitle` in `zbobr-api/src/context/stage_title.rs`
- New format: `instance:pipeline:run_id:**stage**` (e.g., `mybot:main:1:**preparation**`)
- Serialization: prepend `instance:` before the pipeline segment
- Deserialization: detect format by checking whether the 2nd colon-separated token is numeric (old format: pipeline:run_id:**stage**) or the 3rd is (new format: instance:pipeline:run_id:**stage**); default to empty string for old format
- Update `MdMdStageTitleForPrompt` (prompt variant) the same way

### 6. Wire up in dispatcher
- In `cli.rs`: pass `dispatcher_config.instance` when calling `task_backend.list_tasks(&instance)`
- In `cli.rs`: pass `instance` when constructing `MdStageTitle`
- In `commands.rs`: pass `instance` when calling `setup(force, &instance)`

---

## Key Design Decisions
- `instance` lives in `ZbobrDispatcherConfig`, not in the GitHub-specific backend config — it's a zbobr concept
- Instance is passed via trait method parameters (not stored in backend config) — consistent with how `force` is passed to `setup()`
- Backward-compatible stage title parsing — existing context entries without the instance prefix are parsed gracefully
- `--force` is the existing mechanism for destructive setup operations; extending it to instance label cleanup is consistent with existing semantics
