use std::{fmt, str::FromStr};

/// All possible static MCP tools across all roles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum McpTool {
    ReportSuccess,
    ReportFailure,
    ReportIntermediate,
    AddChecklistItem,
    CheckChecklistItem,
    DeleteCtxRec,
    StopWithError,
    StopWithQuestion,
    ConfigureWorktree,
}

impl McpTool {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ReportSuccess => "report_success",
            Self::ReportFailure => "report_failure",
            Self::ReportIntermediate => "report_intermediate",
            Self::AddChecklistItem => "add_checklist_item",
            Self::CheckChecklistItem => "check_checklist_item",
            Self::DeleteCtxRec => "delete_ctx_rec",
            Self::StopWithError => "stop_with_error",
            Self::StopWithQuestion => "stop_with_question",
            Self::ConfigureWorktree => "configure_worktree",
        }
    }

    pub const fn all() -> &'static [Self] {
        ALL_TOOLS
    }
}

impl fmt::Display for McpTool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for McpTool {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "report_success" => Ok(Self::ReportSuccess),
            "report_failure" => Ok(Self::ReportFailure),
            "report_intermediate" => Ok(Self::ReportIntermediate),
            "add_checklist_item" => Ok(Self::AddChecklistItem),
            "check_checklist_item" => Ok(Self::CheckChecklistItem),
            "delete_ctx_rec" => Ok(Self::DeleteCtxRec),
            "stop_with_error" => Ok(Self::StopWithError),
            "stop_with_question" => Ok(Self::StopWithQuestion),
            "configure_worktree" => Ok(Self::ConfigureWorktree),
            other => Err(format!("unknown MCP tool: {other}")),
        }
    }
}

pub const ALL_TOOLS: &[McpTool] = &[
    McpTool::ReportSuccess,
    McpTool::ReportFailure,
    McpTool::ReportIntermediate,
    McpTool::AddChecklistItem,
    McpTool::CheckChecklistItem,
    McpTool::DeleteCtxRec,
    McpTool::StopWithError,
    McpTool::StopWithQuestion,
    McpTool::ConfigureWorktree,
];

/// All possible static MCP tool names across all roles.
pub const ALL_TOOL_NAMES: &[&str] = &[
    McpTool::ReportSuccess.as_str(),
    McpTool::ReportFailure.as_str(),
    McpTool::ReportIntermediate.as_str(),
    McpTool::AddChecklistItem.as_str(),
    McpTool::CheckChecklistItem.as_str(),
    McpTool::DeleteCtxRec.as_str(),
    McpTool::StopWithError.as_str(),
    McpTool::StopWithQuestion.as_str(),
    McpTool::ConfigureWorktree.as_str(),
];
