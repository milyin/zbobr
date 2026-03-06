# MCP Response Logging Implementation

## Summary
Added comprehensive logging for all MCP request responses in the zbobr-dispatcher. The implementation includes both `log::info` (detailed key information) and `log::debug` (exact JSON) levels.

## Changes Made

### File: `zbobr-dispatcher/src/mcp/traits.rs`

#### 1. Helper Functions Added (Lines 7-117)

Three logging helper functions were added after the imports:

**`log_mcp_comments_response()`** - For `get_history` responses
- **Info Level**: Logs the number of comments returned, then for each comment:
  - Comment type (Request, Plan, Response, Report, Error, Done, Reject)
  - First line of text (stripped, truncated to 80 chars if needed)
  - Example: `[planner#123] comment type=Plan text=Add authentication to login...`
- **Debug Level**: Logs the exact JSON response
- **Error Handling**: Recognizes JSON parse errors and logs them

**`log_mcp_json_response()`** - For JSON array/object responses (e.g., checklists)
- **Info Level**: Logs item count for arrays or success status for objects
  - Example: `[worker#123] get_checklist returned 5 item(s)`
- **Debug Level**: Logs the exact JSON
- **Error Handling**: Handles parse errors gracefully

**`log_mcp_string_response()`** - For string status responses
- **Info Level**: 
  - For errors: logs the full error message
  - For success: logs truncated response (100 chars max) with "..." suffix
  - Example: `[planner#123] post_plan result: Plan posted and task ready for wor...`
- **Debug Level**: Logs the exact response string

#### 2. Methods Updated with Logging

All `*_impl()` methods in the following traits now call the appropriate logging helper:

**CommonMcpImpl trait:**
- `get_history_impl()` - Returns comments with detailed per-comment logging
- `report_error_impl()` - Status message logging
- `report_results_impl()` - Status message logging
- `ask_user_impl()` - Status message logging
- `get_checklist_impl()` - JSON array logging
- `check_checklist_item_impl()` - String status logging
- `insert_checklist_item_impl()` - String status logging with error tracking
- `update_checklist_item_impl()` - String status logging
- `delete_checklist_item_impl()` - String status logging with error tracking
- `get_param_impl()` - Parameter value logging
- `set_param_impl()` - Parameter update status logging

**PlannerMcpImpl trait:**
- `post_plan_impl()` - Status message logging

**WorkerMcpImpl trait:**
- `ask_planner_impl()` - Status message logging with error tracking

**ReviewerMcpImpl trait:**
- `review_accept_impl()` - Status message logging
- `review_reject_impl()` - Status message logging with error tracking

**TesterMcpImpl trait:**
- `test_accept_impl()` - Status message logging
- `test_reject_impl()` - Status message logging with error tracking

#### 3. Logging Pattern

Each method follows this general pattern:
```rust
// Get response from operation
let response = match operation(...).await {
    Ok(result) => serialize_or_format(result),
    Err(e) => format!("Error: {e}"),
};

// Log the response appropriately
log_mcp_*_response(self.role_name(), self.session().task_id(), "tool_name", &response);

// Return response
response
```

## Log Output Examples

### Info Level Logs

For `get_history` with 3 comments:
```
[planner#456] get_history returned 3 comment(s)
[planner#456] comment type=Request text=Fix the authentication bug in the login...
[planner#456] comment type=Plan text=I will start by checking the login module...
[planner#456] comment type=Response text=Found the issue - the token validation...
```

For `post_plan`:
```
[planner#456] post_plan result: Plan posted and task ready for worker implementation
```

For `get_checklist`:
```
[worker#456] get_checklist returned 4 item(s)
```

For errors:
```
[planner#456] report_error error: Error posting error message: Failed to connect...
```

### Debug Level Logs

All methods also log the exact JSON/string response:
```
[planner#456] response: [{"comment_type":"Request","timestamp":"2026-03-05T10:30:00Z",...}]
```

## Benefits

1. **Detailed Information**: Each MCP call is logged with meaningful context
2. **Debugging Support**: Exact JSON in debug logs for detailed investigation
3. **Performance Monitoring**: Can identify slow operations or repeated failures
4. **Audit Trail**: Complete history of all MCP interactions
5. **Error Tracking**: Clear indication of which operations failed and why
6. **Readability**: Info logs are human-readable and concise

## Testing

All existing tests pass (30 passed; 0 failed). The logging is non-blocking and doesn't affect functionality.
