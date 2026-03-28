All tasks complete. Here's a summary of what was implemented:

## Implementation Summary

**Renamed ERROR → STATUS** across the entire codebase:
- `Task.error` → `Task.status` field
- `---ERROR---` section separator → `---STATUS---`

**Shared status-formatting mechanism** (`zbobr-api/src/backend.rs`):
- Added `QUESTION_PREFIX: char = '❓'` alongside `ERROR_PREFIX: char = '❌'`
- Added `format_status(icon, timestamp, message) -> String` helper

**Enforced pause-with-status API** — it's now impossible to set pause without an explanation:
- Replaced `set_pause(bool)` + `set_pause_with_signal(signal)` with:
  - `set_pause_with_status(status)` — atomic
  - `set_pause_with_status_and_signal(status, signal)` — atomic

**Unified stop_with_error/question** (`mcp/traits.rs`):
- Shared `pause_with_status_impl(tool, icon, message, add_context_record)`
- `stop_with_error`: ❌ icon → STATUS field only
- `stop_with_question`: ❓ icon → STATUS field + Question context record (no more GitHub comments)

**All tests pass** with no warnings.