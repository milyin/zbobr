# Implementation Plan: TaskContext Structure

## Context
The current system splits work tracking between checklist items (in task description) and comments feed. This makes it hard to analyze tasks, observe progress, and control context size. The new TaskContext structure unifies these into a single timeline-aware structure stored in the task description.

## Architecture

### New Types (zbobr-api/src/task.rs)
- `ContextRecordType` enum: Checkbox(bool), Success, Failure, Comment, Question
- `ContextRecord`: id (u64), record_type, brief, report_link
- `StageInfo`: pipeline, stage, tool, model, prompt_link, timestamp
- `StageContext`: info, records, user_comment
- `TaskContext`: stages vec with methods (next_id, find_record, delete_record)

### Key Design Decisions
1. **No pipeline scoping**: Unlike old checklist with scoped IDs (pipeline__run_id__item_id), context uses global numeric IDs. Stage headers carry pipeline/run metadata.
2. **Error propagation**: parse_context returns Result — no silent data loss on parse failures.
3. **Dual storage for reports**: report_success/failure still posts comments (for dispatcher workflow/transitions) AND adds ContextRecord to TaskContext.
4. **User comments interspersed**: MD output includes user comments by timestamp for agent context, but parse ignores them (authoritative source is comments feed).
5. **for_prompt flag**: MD generator omits prompt links when rendering for prompt templates.

### MCP Changes
- Removed: GetHistory, GetChecklist, GetFullReport, DeleteChecklistItem
- Added: DeleteCtxRec (accepts "42" or "ctx_rec_42")
- Modified: AddChecklistItem gains optional long_description (stored as file)
- Multiple report_success/failure calls now allowed per stage

### Files Modified (14 files)
- zbobr-api/src/task.rs — core types
- zbobr-api/src/context_format.rs — new MD serialize/parse
- zbobr-api/src/checklist_format.rs — deleted
- zbobr-api/src/lib.rs — exports
- zbobr-task-backend-github/src/separator.rs — CONTEXT section
- zbobr-task-backend-github/src/github.rs — context field
- zbobr-task-backend-fs/src/fs.rs — TaskFile.context
- zbobr-api/src/config_tools.rs — McpTool enum
- zbobr-dispatcher/src/mcp/traits.rs — tool implementations
- zbobr-dispatcher/src/mcp/unified.rs — tool routing
- zbobr-dispatcher/src/mcp/common.rs — param structs
- zbobr-dispatcher/src/task.rs — RoleSession methods
- zbobr-dispatcher/src/prompts.rs — {context} variable
- zbobr-dispatcher/src/cli.rs — StageContext creation