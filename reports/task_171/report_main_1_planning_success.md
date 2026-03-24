## Design Rationale

**Chosen analog:** The PARAMETERS section in `separator.rs` is the closest pattern. The ERROR section follows the same parse/serialize/merge approach with one key difference: error is `Option<String>` (single value that replaces) rather than `HashMap` (multiple key-value pairs).

**Section order:** description → PARAMETERS → ERROR → CHECKLIST. This keeps CHECKLIST last (it's the most frequently changing section) and places ERROR after PARAMETERS for logical grouping.

**Key decisions:**
1. Error replaces (not appends) — achieved naturally by storing as `Option<String>` on the Task struct
2. `HistoryRecordType::Error` is removed — since errors no longer appear as comments, the classification variant is unnecessary. Legacy `[stop_with_error]` comments in existing data will classify as `Other`.
3. Both FS and GitHub backends updated — FS backend uses `#[serde(default)]` for backward compatibility with existing YAML files.
4. The `stop_with_error_impl` is simplified — instead of posting a comment, it sets the error field on the task body via `modify_task`, then sets pause. This is atomic per-field (two separate `modify_task` calls) matching the existing pattern.

**Files affected (8):**
- `zbobr-task-backend-github/src/separator.rs` — core parsing/serialization
- `zbobr-api/src/task.rs` — Task struct + HistoryRecordType cleanup
- `zbobr-api/src/backend.rs` — TaskMut trait
- `zbobr-task-backend-github/src/github.rs` — GitHub backend
- `zbobr-task-backend-fs/src/fs.rs` — FS backend
- `zbobr-dispatcher/src/task.rs` — RoleSession
- `zbobr-dispatcher/src/mcp/traits.rs` — stop_with_error_impl
- `zbobr-dispatcher/tests/mcp_integration/test_helpers.rs` — test assertions