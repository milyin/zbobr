/// Tool name types and `PromptBuilder` trait for building role-specific
/// instruction prompts.
///
/// Each role gets a marker type with associated constants matching the actual
/// MCP method names. The `PromptBuilder` trait accepts references to these
/// types so that prompt implementations can reference tool names without
/// depending on the MCP modules.

// ---------------------------------------------------------------------------
// Tool name types — one per role, with associated constants
// ---------------------------------------------------------------------------

/// MCP tool names available to the Preparator role.
pub struct PreparatorToolNames;
impl PreparatorToolNames {
    pub const GET_HISTORY: &str = "get_history";
    pub const REPORT_ERROR: &str = "report_error";
    pub const SET_PARAM_DESTINATION_REPOSITORY: &str = "set_param_destination_repository";
    pub const SET_PARAM_DESTINATION_BRANCH: &str = "set_param_destination_branch";
    pub const SET_PARAM_WORK_BRANCH_POSTFIX: &str = "set_param_work_branch_postfix";
    pub const GET_PARAM_DESTINATION_REPOSITORY: &str = "get_param_destination_repository";
    pub const GET_PARAM_DESTINATION_BRANCH: &str = "get_param_destination_branch";
    pub const GET_PARAM_WORK_BRANCH: &str = "get_param_work_branch";
    pub const REPORT_RESULTS: &str = "report_results";
}

/// MCP tool names available to the Planner role.
pub struct PlannerToolNames;
impl PlannerToolNames {
    pub const GET_HISTORY: &str = "get_history";
    pub const POST_PLAN: &str = "post_plan";
    pub const GET_CHECKLIST: &str = "get_checklist";
    pub const INSERT_CHECKLIST_ITEM: &str = "insert_checklist_item";
    pub const UPDATE_CHECKLIST_ITEM: &str = "update_checklist_item";
    pub const DELETE_CHECKLIST_ITEM: &str = "delete_checklist_item";
    pub const REPORT_ERROR: &str = "report_error";
    pub const ASK_USER: &str = "ask_user";
    pub const GET_PARAM_DESTINATION_BRANCH: &str = "get_param_destination_branch";
    pub const GET_PARAM_WORK_BRANCH: &str = "get_param_work_branch";
}

/// MCP tool names available to the Worker role.
pub struct WorkerToolNames;
impl WorkerToolNames {
    pub const GET_HISTORY: &str = "get_history";
    pub const REPORT_ERROR: &str = "report_error";
    pub const ASK_USER: &str = "ask_user";
    pub const ASK_PLANNER: &str = "ask_planner";
    pub const GET_CHECKLIST: &str = "get_checklist";
    pub const INSERT_CHECKLIST_ITEM: &str = "insert_checklist_item";
    pub const UPDATE_CHECKLIST_ITEM: &str = "update_checklist_item";
    pub const CHECK_CHECKLIST_ITEM: &str = "check_checklist_item";
    pub const DELETE_CHECKLIST_ITEM: &str = "delete_checklist_item";
    pub const GET_PARAM_DESTINATION_BRANCH: &str = "get_param_destination_branch";
    pub const GET_PARAM_WORK_BRANCH: &str = "get_param_work_branch";
    pub const REPORT_RESULTS: &str = "report_results";
}

/// MCP tool names available to the Reviewer role.
pub struct ReviewerToolNames;
impl ReviewerToolNames {
    pub const GET_HISTORY: &str = "get_history";
    pub const REPORT_ERROR: &str = "report_error";
    pub const GET_PARAM_DESTINATION_BRANCH: &str = "get_param_destination_branch";
    pub const GET_PARAM_WORK_BRANCH: &str = "get_param_work_branch";
    pub const REVIEW_ACCEPT: &str = "review_accept";
    pub const REVIEW_REJECT: &str = "review_reject";
}

/// MCP tool names available to the Tester role.
pub struct TesterToolNames;
impl TesterToolNames {
    pub const GET_HISTORY: &str = "get_history";
    pub const REPORT_ERROR: &str = "report_error";
    pub const GET_PARAM_DESTINATION_BRANCH: &str = "get_param_destination_branch";
    pub const GET_PARAM_WORK_BRANCH: &str = "get_param_work_branch";
    pub const TEST_ACCEPT: &str = "test_accept";
    pub const TEST_REJECT: &str = "test_reject";
}

/// MCP tool names available to the Merger role.
pub struct MergerToolNames;
impl MergerToolNames {
    pub const GET_HISTORY: &str = "get_history";
    pub const REPORT_ERROR: &str = "report_error";
    pub const ASK_USER: &str = "ask_user";
    pub const GET_PARAM_DESTINATION_BRANCH: &str = "get_param_destination_branch";
    pub const GET_PARAM_WORK_BRANCH: &str = "get_param_work_branch";
    pub const REPORT_RESULTS: &str = "report_results";
}

// ---------------------------------------------------------------------------
// PromptBuilder trait
// ---------------------------------------------------------------------------

/// Trait for building role-specific instruction prompts.
///
/// Each method returns the instructions for the given role.  Tool name types
/// with associated constants are passed so prompt implementations can
/// reference actual MCP method names without depending on the MCP modules.
///
/// Implementations live in separate crates (e.g. `zbobr-prompts`) so the
/// prompt module is replaceable, the same way backends are swappable.
pub trait PromptBuilder: Send + Sync {
    fn preparator_instructions(&self, tools: &PreparatorToolNames) -> String;
    fn planner_instructions(&self, tools: &PlannerToolNames) -> String;
    fn worker_instructions(&self, tools: &WorkerToolNames) -> String;
    fn reviewer_instructions(&self, tools: &ReviewerToolNames) -> String;
    fn tester_instructions(&self, tools: &TesterToolNames) -> String;
    fn merger_instructions(&self, tools: &MergerToolNames) -> String;
}
