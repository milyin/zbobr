## Fix: Incorrect Timezone of Interspersed Comments

### Problem
Comment timestamps from GitHub API arrive as UTC (`+0000`). The `get_task_comments_internal()` method parses them as-is, so when displayed in context they always show `+0000` instead of the configured timezone. The FS backend similarly reads timestamps without conversion.

### Approach
Follow the existing `instance` injection pattern: add a `timezone` field to backend configs, inject it from dispatcher config at construction time, and apply `.with_timezone()` when building `Comment` objects.

### Changes

**1. Add `timezone` field to GitHub backend config**
- `zbobr-task-backend-github/src/config.rs` — add `pub timezone: Option<chrono::FixedOffset>` with `#[config(skip_args)]` (same as `instance`), default `None`

**2. Add `timezone` field to FS backend config**
- `zbobr-task-backend-fs/src/config.rs` — same field and annotation

**3. Apply timezone conversion in GitHub backend**
- `zbobr-task-backend-github/src/github.rs`, `get_task_comments_internal()` (~line 997) — after parsing UTC timestamp, convert with `.with_timezone()` using `self.backend_config.timezone` (fall back to local offset if `None`)

**4. Apply timezone conversion in FS backend**
- `zbobr-task-backend-fs/src/fs.rs`, `read_comments_structured()` (~line 222) — same conversion after deserializing

**5. Inject timezone at construction site**
- `zbobr/src/commands.rs` (~line 202) — after `tasks_config.instance = ...`, add `tasks_config.timezone = Some(dispatcher_config.fixed_offset())`, mirrors existing pattern exactly

**6. Update test construction sites**
- `zbobr-dispatcher/tests/mcp_integration/env.rs` — set `timezone` field in backend configs (lines ~137, ~173, ~237, ~278)
