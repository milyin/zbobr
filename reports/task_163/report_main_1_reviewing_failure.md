# Review Report: Task 163 - Context Structure

**Status**: Failure ❌

## Summary
The implementation is currently incomplete. The worker has successfully implemented the core data structures (`TaskContext`, `StageContext`, `ContextRecord`) and the serialization/parsing logic in `zbobr-api`. However, the integration of these structures into the rest of the system (backends, dispatcher, MCP tools, prompts) has not been started. The `checklist` field still exists in the `Task` struct and is presumably still being used by the rest of the application.

## Detailed Findings

### 1. Scope of Changes
- **Implemented**:
    - `zbobr-api/src/context_format.rs`: Serialization and parsing logic for the new context format.
    - `zbobr-api/src/task.rs`: Added `TaskContext` struct and included it in the `Task` struct.
    - `zbobr-api/src/lib.rs`: Exposed the new module.
- **Missing**:
    - **Backends**: `zbobr-task-backend-github` and `zbobr-task-backend-fs` need to be updated to store/retrieve the context in the task description/file.
    - **Dispatcher**: Logic in `zbobr-dispatcher` needs to be updated to manipulate the context instead of the checklist.
    - **MCP Tools**: Old checklist tools need to be removed/deprecated, and new context tools (`DeleteCtxRec`, etc.) need to be added.
    - **Prompts**: Prompts need to be updated to use the new context variable.
    - **Cleanup**: The `checklist` field should be removed from `Task` once the migration is complete (or marked as deprecated if a transition period is needed).

### 2. Code Quality Review (zbobr-api)
The code that *has* been written looks high-quality and follows the plan:
- **Data Structures**: `TaskContext`, `StageContext`, and `ContextRecord` are correctly defined with Serde support.
- **Serialization**: `serialize_context` correctly formats the context as Markdown, including:
    - Stage headers with metadata.
    - Context records with appropriate icons/formatting.
    - Interspersed user comments (sorted by timestamp).
    - Optional suppression of prompt links.
- **Parsing**: `parse_context` correctly reconstructs the `TaskContext` from Markdown, ignoring user comments (blockquotes).
- **Testing**: The tests in `context_format.rs` provide good coverage of the serialization/parsing logic, including round-trip tests.

### 3. Recommendations for Next Steps
The next worker should proceed with the existing plan, focusing on:
1.  **Update Backends**: Modify `issue_to_task` / `task_to_issue` (GitHub) and `from_task` / `to_task` (FS) to handle the `context` field using the new serialization logic.
2.  **Update Dispatcher**: Refactor `RoleSession` to use `TaskContext` methods for managing items.
3.  **Update MCP**: Implement the new MCP tools and remove the old ones.
4.  **Update Prompts**: Change the prompt templates to inject the serialized context.
5.  **Cleanup**: Remove `checklist` usage and the field itself.

## Analog Consistency
The new `TaskContext` structure follows the pattern of the existing `Task` structure and uses `serde` for serialization, which is consistent with the rest of the codebase. The decision to put the formatting logic in a separate module (`context_format.rs`) matches the existing `checklist_format.rs` pattern.

## Conclusion
The foundation is laid, but the feature is not yet functional. The task cannot be marked as complete until the context is actually persisted and used by the agent.
