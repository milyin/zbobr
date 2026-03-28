## Proposed Plan

### Overview

The task requires:
1. Rename `---ERROR---` section to `---STATUS---` in the GitHub task body
2. `stop_with_error`: set STATUS field with ❌ icon (as before), no comment
3. `stop_with_question`: set STATUS field with ❓ icon AND add a context record (like `report_..` tools do) — no more posting to comments
4. Shared mechanism for formatting the STATUS field content (icon + timestamp + message)

---

### Component 1: Rename ERROR separator in GitHub backend

**File:** `zbobr-task-backend-github/src/separator.rs`

- Rename `ERROR_SEPARATOR` constant → `STATUS_SEPARATOR`, value `"\n\n---STATUS---\n"`
- Update all parse/serialize/merge functions that reference it
- Update all tests (`roundtrip_preserves_error_section`, assertions on `"---ERROR---"`, etc.)

No change to the FS backend (it stores `error` as a YAML field, not as a section in the body).

---

### Component 2: Shared formatting constants and helper

**File:** `zbobr-api/src/backend.rs`

- Add `pub const QUESTION_PREFIX: char = '❓'` alongside `ERROR_PREFIX`
- Add a `set_status_entry(icon: char, msg: Option<String>)` default method on `TaskMut` that formats `"{icon} {ts} {msg}"` and stores to `task.error`. Have `set_error` delegate to it.

**File:** `zbobr-api/src/lib.rs`

- Export `QUESTION_PREFIX` from the crate root.

---

### Component 3: `set_status` method on `RoleSession`

**File:** `zbobr-dispatcher/src/task.rs`

- Add `pub async fn set_status(&self, icon: char, message: &str) -> Result<()>` that formats with the given icon + current timestamp and stores to `task.error`
- Update existing `set_error` to delegate to `set_status(zbobr_api::ERROR_PREFIX, msg)` — removes the duplicated inline format

---

### Component 4: Unified `stop_impl` in MCP traits

**File:** `zbobr-dispatcher/src/mcp/traits.rs`

Add a shared private async helper `stop_with_status_impl(icon, record_type, message)` that:
1. Calls `session().set_status(icon, message)` — writes to STATUS field
2. If `record_type` is `Some(t)`: calls `session().add_context_record(t, message, None)` — adds a context record (like `report_..` does)
3. Calls `session().set_pause(true)` — stops task processing

Refactor:
- `stop_with_error_impl` → calls `stop_with_status_impl(ERROR_PREFIX, None, message)`
- `stop_with_question_impl` → calls `stop_with_status_impl(QUESTION_PREFIX, Some(ContextRecordType::Question), message)`
  - **Removes** the current `post_comment` call

`ContextRecordType::Question` already exists in the codebase and is rendered with `❓` in context serialization.

---

### Key design decisions

- The `task.error` field (internal name stays as-is) is the backing store for the STATUS section — renaming happens only at the serialization layer (separator string) in the GitHub backend
- `ContextRecordType::Question` is already defined and handled in context rendering — no new types needed
- The shared formatting lives in `RoleSession::set_status` (dispatcher layer), mirroring how `report_impl` lives in `CommonMcpImpl`
- The FS backend YAML field name `error` is not renamed — no user-visible impact since FS backend is for testing
