# Review Report: Context Structure Implementation

## Status: Incomplete / Failure ❌

The implementation of `TaskContext` structure is started but significant portions of the task are missing.

## Findings

### 1. Data Structures and Parsing (✅ Partially Complete)
- `TaskContext`, `StageContext`, `ContextRecord` etc. are correctly defined in `zbobr-api/src/task.rs`.
- `context_format.rs` correctly implements markdown serialization/parsing, including interspersing user comments.
- `separator.rs` is updated to handle `TaskContext` in the task description.
- **Issue**: `Task` struct in `zbobr-api/src/task.rs` still contains the `checklist` field, which should be removed according to requirements.

### 2. Backend Integration (❌ Missing)
- `zbobr-task-backend-github/src/github.rs` does not use `TaskContext`. It likely still relies on `checklist`.
- `zbobr-task-backend-fs` is not updated.
- The `checklist` field needs to be removed and replaced by `context` in backend mappings.

### 3. MCP Implementation (❌ Missing)
- `GetHistory`, `GetChecklist` tools are not removed.
- `DeleteCtxRec` tool is not added.
- `AddChecklistItem` is not updated.
- MCP implementations in `zbobr-dispatcher` need to be updated to operate on `TaskContext`.

### 4. Prompts and Templating (❌ Missing)
- Prompt templates need to be updated to use `{context}` instead of checklist/history variables.
- Logic to inject `TaskContext` into prompts is missing.

### 5. CLI Integration (❌ Missing)
- Stage creation in CLI needs to initialize `StageContext`.

## Recommendations

1.  **Remove `checklist`**: Remove the `checklist` field from `Task` struct and all references to it.
2.  **Update Backends**:
    - Update `issue_to_task` in `github.rs` to map parsed context to `task.context`.
    - Update `modify_task`/`create_task` to serialize `task.context` correctly.
    - Update FS backend similarly.
3.  **Update MCP**:
    - Remove obsolete tools (`GetHistory`, `GetChecklist`, etc.).
    - Implement `DeleteCtxRec`.
    - Update `RoleSession` to handle context records instead of checklist items.
4.  **Update Prompts**:
    - Switch prompt templates to use `{context}`.
5.  **Clean up**:
    - Ensure all tests pass.
    - Remove `ChecklistItem` struct if unused.

Please proceed with the remaining items in the checklist.
