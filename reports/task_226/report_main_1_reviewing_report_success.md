# Review: Rename ERROR section to STATUS + Unify pause-with-status API

## Status: ✅ SUCCESS

All checklist items completed and verified correct.

### Implementation Review

**Task Requirements Met:**
1. ✅ Renamed ERROR section to STATUS section
2. ✅ Place last error/question in STATUS with icon (❌ for error, ❓ for question) and timestamp
3. ✅ Questions placed in TWO locations:
   - Agent report (via add_context_record with ContextRecordType::Question)
   - STATUS section (via pause_with_status)
4. ✅ Error only in STATUS section (no comment posting)
5. ✅ Unified mechanism: shared pause_with_status_impl for both error and question
6. ✅ Compile-time enforcement: pause cannot be set without status explanation

### Completed Checklist Items

**1. Rename error→status in Task data model** ✅
- Field renamed at zbobr-api/src/task.rs:1212
- Documentation updated
- All 10+ task constructors updated across codebase

**2. Rename separator ERROR→STATUS in backends** ✅
- separator.rs: constant renamed, all parse/serialize logic updated
- github.rs: uses new separator constant
- fs.rs: field renamed in TaskFile struct
- Tests updated to check for ---STATUS--- separator

**3. Introduce shared status-formatting + enforce API** ✅
- format_status(icon, timestamp, message) function in backend.rs
- ERROR_PREFIX (❌) and QUESTION_PREFIX (❓) constants defined
- Exports via zbobr-api/src/lib.rs
- set_pause() method completely removed
- Only set_pause_with_status/set_pause_with_status_and_signal exist

**4. Update RoleSession/TaskSession dispatcher** ✅
- RoleSession.set_pause_with_status (status required)
- RoleSession.set_pause_with_status_and_signal (status + signal required)
- TaskSession has matching methods
- Old set_pause(bool) removed
- Old set_error() removed (replaced with set_status)
- Status properly clears on running state transition

**5. Refactor stop_with_error/question to use shared mechanism** ✅
- New pause_with_status_impl unifies both
- Parameters: tool, icon, message, add_context_record
- Errors: add_context_record=false → status field only
- Questions: add_context_record=true → status field + context record
- Old stop_with_question_impl behavior (post_comment) removed

**6. Update cli.rs pause callers** ✅
- 8 pause sites identified and updated:
  - Stage count limit checks
  - Pipeline failure handling
  - Merge conflict recursion guard
  - Stash/push failures
  - Tool execution failures
  - SequentialSignal::PauseThenSignal
  - Worktree configuration errors
  - ensure_pr_url errors
- New format_error_status() helper for consistent formatting
- set_task_status_with_log() (renamed from set_task_error_with_log)

### Code Quality Verification

**API Design:**
- Pause-with-status coupling enforced at compile time
- No way to set pause=true without providing status explanation
- Type-safe: status is String type, always pre-formatted with icon/timestamp

**Consistency:**
- Format: "{icon} {timestamp} {message}"
- Used in pause_with_status_impl, configure_worktree_error, cli format_error_status
- Consistent naming: ERROR_PREFIX, QUESTION_PREFIX, format_status

**Test Updates:**
- comment_model_tests updated: checks status field, verifies icon/timestamp presence
- abstract_test_helpers: checks task.status contains expected message
- test_helpers: verifies status appears correctly
- All test constructors use status: None

**Unused Code:**
- get_hostname() function in mcp/common.rs unused (import removed from traits.rs)
  - Conservative approach: function definition remains for future use
  - No cleanup necessary

### Verification

✅ No remaining calls to set_pause(true) without status
✅ No remaining use of ERROR_SEPARATOR constant
✅ No ERROR_PREFIX in old error field context
✅ All task constructors updated
✅ All pause sites properly formatted with status
✅ Separator parsing/serialization complete
✅ Tests properly validate new behavior
✅ Compile-time API enforcement in place

### Summary

The implementation is complete, well-structured, and correctly enforces the new requirements:
- ERROR section renamed to STATUS
- Shared mechanism for error/question pause handling
- Questions properly added to both context records and STATUS field
- Errors only in STATUS field
- Pause-with-status coupling enforced at API level
- All existing patterns and conventions maintained

No issues found. Implementation is ready for testing and deployment.