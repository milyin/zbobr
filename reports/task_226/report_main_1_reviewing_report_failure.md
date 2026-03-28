# Review Report: ERROR→STATUS Rename and Unified Pause-with-Status API

## Overall Assessment
The implementation correctly handles the core requirements for renaming the ERROR section to STATUS and unifying the stop_with_error/stop_with_question mechanism. However, there is **one critical issue** that violates the API design constraint and must be fixed.

## Critical Issue: API Constraint Violation

### Problem
The dispatcher's `set_state()` method in `zbobr-dispatcher/src/task.rs` (line 495) violates the requirement: "ensure this coupling on api level, it should be impossible to set pause without explanation."

**Current Code:**
```rust
pub async fn set_state(&self, state: impl Into<State>) -> anyhow::Result<()> {
    let state = state.into();
    self.modify_task(move |mut task| {
        if task.confirm && task.state != state {
            task.pause = true;  // <-- Sets pause WITHOUT status message!
        }
        // ...
        task.state = state;
        task
    })
    .await
}
```

When `confirm` flag is true and state changes, the code directly sets `pause = true` without calling the new `set_pause_with_status` method. This bypasses the API constraint that pause must always be accompanied by a status explanation.

### Impact
- Violates the API design principle stated in the task requirements
- Creates a code path where pause can be set without explanation
- Inconsistent with all other pause-setting code that uses `set_pause_with_status`

### Required Fix
Replace the direct pause assignment with a call to the proper API:
```rust
if task.confirm && task.state != state {
    // Instead of: task.pause = true;
    // Should use the set_pause_with_status mechanism
}
```

The method should either:
1. Generate an appropriate status message (e.g., "State changed to {new_state} - dispatcher auto-paused due to confirm flag") and use `set_pause_with_status`
2. OR be restructured to separate the state-change logic from the auto-pause mechanism

This affects all callers of `set_state()` throughout `cli.rs` (at least 10+ locations).

## Positive Findings - All Correctly Implemented

✅ **Field Renaming**: All `error` fields correctly renamed to `status` across:
   - `zbobr-api/src/task.rs` (Task struct)
   - `zbobr-task-backend-fs/src/fs.rs` (TaskFile struct)
   - `zbobr-task-backend-github/src/github.rs` (parsing/serialization)
   - All test files

✅ **Separator Renaming**: `ERROR_SEPARATOR` → `STATUS_SEPARATOR`
   - Correctly defined in `separator.rs` as `"\n\n---STATUS---\n"`
   - Properly used in all parse/serialize functions
   - GitHub and FS backends correctly updated

✅ **Status Formatting Mechanism**: 
   - `format_status()` in `backend.rs` correctly formats: `"{icon} {timestamp} {message}"`
   - Uses proper Unicode symbols: `ERROR_PREFIX` (❌) and `QUESTION_PREFIX` (❓)
   - Timestamp formatting uses shared `format_timestamp()` helper

✅ **Unified Pause-with-Status API**:
   - `RoleSession::set_pause_with_status()` enforces atomic status+pause setting
   - `set_pause_with_status_and_signal()` available for pipeline signal assignment
   - Impossible to call pause-setting without providing status message (via trait design)

✅ **Shared Implementation for Error/Question**:
   - `pause_with_status_impl()` handles both error and question via `add_context_record` parameter
   - Error (false): sets STATUS section only
   - Question (true): sets STATUS section + adds Question context record
   - Both use same formatting and timestamp logic

✅ **Questions in Two Locations**:
   - STATUS field: set via `set_pause_with_status` (line 396)
   - Context record: added with `ContextRecordType::Question` (line 415)
   - Proper separation from comments (no comment posting for questions)

✅ **CLI Callers Updated**: All `cli.rs` pause-related calls correctly use:
   - `set_pause_with_status()` for simple error cases
   - `set_pause_with_status_and_signal()` for pipeline transitions
   - `format_error_status()` helper for consistent formatting

✅ **Tests Updated**: All test assertions updated to check `task.status` instead of `task.error`

✅ **Formatting**: All rustfmt issues resolved in second commit

## Architecture Assessment

The implementation correctly follows existing patterns:
- Status formatting matches context record timestamp format
- Uses existing constants (ERROR_PREFIX, QUESTION_PREFIX)
- Maintains backward compatibility in backend trait design
- Question context records properly use `ContextRecordType::Question`

## Recommendation

**DO NOT MERGE** until the `set_state()` dispatcher method is fixed to comply with the API constraint. This is a design requirement, not a minor issue.

All other aspects of the implementation are correct and ready for merge once this critical issue is resolved.
