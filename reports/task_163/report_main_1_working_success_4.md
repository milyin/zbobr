# Review Fixes Implementation Report

## Summary
All 5 checklist items completed across 4 commits. The workspace builds cleanly with no warnings and all 109 tests pass.

## Changes by Checklist Item

### 1. Remove obsolete `checklist` field from Task (commit 9191eb1)
**19 files changed, 33 insertions, 1102 deletions**

- Removed `ChecklistItem` struct from `zbobr-api/src/task.rs`
- Deleted `zbobr-api/src/checklist_format.rs` module entirely
- Removed `set_checklist` from backend trait (`zbobr-api/src/backend.rs`)
- Removed `checklist` field from `Task` struct
- Removed `ChecklistItem` exports from `zbobr-api/src/lib.rs` and `zbobr-dispatcher/src/lib.rs`
- Removed checklist methods (`get_checklist`, `add_checklist_item`, `check_checklist_item`, `delete_checklist_item`, `checklist_scope_prefix`, `strip_checklist_scope`) from `RoleSession` and `TaskSession` in `zbobr-dispatcher/src/task.rs`
- Removed checklist display from `zbobr-dispatcher/src/cli.rs`
- Removed `VAR_CHECKLIST`, `pipeline_scope` parameter, and checklist building from `zbobr-dispatcher/src/prompts.rs`
- Removed checklist MCP tool definitions from `zbobr-dispatcher/src/mcp/unified.rs`
- Removed checklist impl methods from `zbobr-dispatcher/src/mcp/traits.rs`
- Removed checklist param structs from `zbobr-dispatcher/src/mcp/common.rs`
- Removed `checklist: vec![]` from all Task constructions in fs.rs, github.rs, workflow.rs, commands.rs
- Removed `{checklist}` template placeholder from init.rs task prompt
- Removed checklist tests from prompts.rs, dispatcher task.rs, and fs backend
- Updated mcp_integration test_helpers.rs to remove checklist assertions

### 2. Remove unused `user_comment` from StageContext (commit aa81c78)
**3 files changed, 14 insertions, 17 deletions**

- Removed `user_comment: Option<String>` field from `StageContext` in `zbobr-api/src/task.rs`
- Removed all `user_comment: None` initializations in `context_format.rs` (4 places), `task.rs` tests (4 places), and `separator.rs` (4 places)

### 3. Fix Pipeline::from() to .parse() (commit 36a0dae)
**1 file changed, 1 insertion, 1 deletion**

- Changed `Pipeline::from(pipeline_str)` to `pipeline_str.parse().unwrap()` in `context_format.rs:220` for consistency with how `tool` and `model` are parsed nearby

### 4. Error on unrecognized lines + MCP cleanup (commit 0a4643d)
**4 files changed, 7 insertions, 33 deletions**

- Added `bail!("Unrecognized line in context: {}", trimmed)` after HTML comment skip in `parse_context()`
- Removed checklist-related `McpTool` enum variants (`GetChecklist`, `AddChecklistItem`, `CheckChecklistItem`, `DeleteChecklistItem`) from `config_tools.rs` to match actual registered tools
- Updated init.rs role definitions to remove checklist tool references
- Fixed unused import warnings

### 5. Build/Test Verification
- `cargo build --workspace`: clean, no warnings
- `cargo test --workspace`: 109 tests pass, 0 failures
