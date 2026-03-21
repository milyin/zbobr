/// All possible static MCP tool names across all roles.
/// Dynamic `call_*` tools are not included here.
pub const ALL_TOOL_NAMES: &[&str] = &[
    "get_history",
    "report_success",
    "report_failure",
    "stop_with_error",
    "stop_with_question",
    "configure_worktree",
    "get_checklist",
    "add_checklist_item",
    "check_checklist_item",
    "delete_checklist_item",
];
