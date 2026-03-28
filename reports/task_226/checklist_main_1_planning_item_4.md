
## What to change

In `zbobr-dispatcher/src/mcp/traits.rs`:

### `stop_with_error_impl`
Replace the two separate calls (`set_error` + `set_pause`) with a single `pause_with_status(STATUS_ICON_ERROR, message)` call. No context record, no comment.

### `stop_with_question_impl`
Replace the current comment-posting approach entirely:
1. Call `pause_with_status(STATUS_ICON_QUESTION, message)` — sets status field + pause, no comment
2. Additionally add a context record (like `report_impl` does): store a report file with the question text and add a context record so the question appears in the task context alongside other stage records. Use `ContextRecordType::Comment` for the record type.

### Common helper (optional, if it reduces duplication)
Consider extracting `stop_with_status_impl(icon: char, record_type: Option<ContextRecordType>, brief: &str)` that both call, branching only on whether to add a context record. This eliminates duplication and makes the "only difference is question goes to context" rule explicit in code.

## Why

The task says: "The question and error procedures should reuse the same code. The only difference between them is that question is placed to context, the error is not."

Currently `stop_with_question_impl` posts a comment (comment-based approach), but the requirement is to place it in the context (context-record approach like `report_*`). And `stop_with_error_impl` already doesn't post to context.

## Analog

The `report_impl` method in `traits.rs` already shows the pattern for storing a report file and adding a context record. Follow the same flow for the question path.
