## Plan: Append instance name for work directories

### Problem
Multiple zbobr instances sharing the same `workspaces` and `repos_dir` directories will collide — e.g., both write to `workspaces/task-123/` or `repos/owner__repo.git`. Currently, the `instance` field (default: `"default"`) is only used for GitHub labels, not filesystem isolation.

### Approach
Append `/{instance}` to the `workspaces` and `repos_dir` paths at the **wiring point** — the single place where all configs are assembled — before constructing backends or the dispatcher. This is the minimal change that achieves full isolation.

**Why this approach:** The `workspaces` path lives in `ZbobrDispatcherConfig` and `repos_dir` lives in repo backend configs. Rather than threading `instance` through every struct and method, we simply adjust the two base paths early. All downstream code (`TaskDir::new`, `ensure_bare_clone`, `cleanup_closed_tasks`, etc.) automatically operates within the instance-scoped directory because it reads the already-adjusted paths.

### Changes

1. **`zbobr/src/commands.rs` — `run()` function (~line 183)**
   Before creating backends, append instance to both directory paths:
   - `dispatcher_config.workspaces = dispatcher_config.workspaces.join(&dispatcher_config.instance);`
   - `repo_config.repos_dir = repo_config.repos_dir.join(&dispatcher_config.instance);`

2. **`zbobr-dispatcher/tests/mcp_integration/env.rs` — test environment setup**
   Apply the same pattern in test helpers that construct dispatcher configs, so tests reflect the production path layout. The `instance` field in test configs is `"default"` (from `ZbobrDispatcherConfig::default()`), so test paths become e.g. `base_path/workspaces/default/task-123/`.

3. **Update existing tests in `task_dir.rs`** if any hardcoded path expectations break (unlikely since TaskDir tests don't go through the wiring point, but verify).

### What stays unchanged
- `TaskDir` struct and methods — no changes needed
- Repo backend configs and implementations — they just use the adjusted `repos_dir`
- Cleanup logic — reads from `config.workspaces` which is already instance-scoped
- `from_path` parsing — still works because the base `workspaces` path is consistent

### Analog
The pattern of modifying config paths at the wiring point before construction already exists: `tasks_config.instance = dispatcher_config.instance.clone()` is done similarly in `commands.rs:206`.

### Verification
- Run `cargo test` to verify no regressions
- Run `cargo clippy` for lint checks
- Inspect that integration tests in `mcp_integration/` pass with the adjusted paths
