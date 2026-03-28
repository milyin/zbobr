## Scope / diff summary
Compared `origin/main...HEAD`. Changed files:
- `zbobr-api/src/backend.rs`: added `ERROR_PREFIX`, added error formatting in `TaskMut::set_error`, and enforced error clear in default `TaskMut::set_state` on non-running→running transition.
- `zbobr-api/src/context/stage_title.rs` + `context/mod.rs`: extracted and re-exported `format_timestamp` so error timestamps can reuse the same formatter as context/stage titles.
- `zbobr-api/src/lib.rs`: re-exported `ERROR_PREFIX` and `format_timestamp` for convenient downstream use.
- `zbobr-dispatcher/src/task.rs`: `RoleSession::set_error` formats errors with ❌ + timestamp; `TaskSession::set_state` clears error on non-running→running transition; unit test updated to assert prefix + timestamp presence.

## Task requirements coverage
1) **Add red X + timestamp when setting error**
- ✅ Implemented in both primary paths:
  - `TaskMut::set_error` (backend-level generic setter)
  - `RoleSession::set_error` (dispatcher role session)
- ✅ Timestamp formatting reuses the same `format_timestamp` helper used by context/stage title rendering.

2) **Clear error when task enters Running**
- ✅ Implemented as `if !task.state.is_running() && state.is_running() { task.error = None; }`
- ✅ Enforced in the default `TaskMut::set_state`, covering all backends that rely on trait defaults.
- ✅ Also present in `TaskSession::set_state`, which bypasses the default setter via `modify_task` and additionally handles `confirm`→`pause` logic.

## Analog/pattern consistency
- The chosen analog (context/stage timestamp formatting) is appropriate.
- The implementation uses a shared `format_timestamp(DateTime<FixedOffset>) -> String` for error timestamps, matching the established `YYYY-MM-DD HH:MM:SS +HHMM` format.
- A shared `ERROR_PREFIX` constant avoids duplicated ❌ literals in formatting logic.

## Code quality & robustness checks
- **No overly aggressive clearing**: clearing happens only on non-running→running transition (not on Running→Running updates).
- **Bypass risk addressed**: formatting is applied at the backend trait default (`TaskMut::set_error`), so callers using the generic API cannot accidentally set a raw/unformatted error.
- **Type specificity**: `ERROR_PREFIX` as `char` is appropriate for `starts_with`/`trim_start_matches` and avoids string allocations.
- **Tests**: updated to assert (a) prefix present, (b) message included, (c) timestamp token follows the icon.

## Minor nits (non-blocking)
- Error formatting logic is still duplicated structurally between `TaskMut::set_error` and `RoleSession::set_error` (though it now shares `ERROR_PREFIX` + `format_timestamp`). If desired later, a small helper could further dedupe.
- `TaskMut::set_error` uses UTC fixed offset (`+0000`); dispatcher uses configured fixed offset. This seems acceptable given `TaskMut` lacks dispatcher config; format remains consistent.

## Conclusion
All checklist items are satisfied, changes are scoped to the task, and behavior matches the requirements and existing formatting conventions.