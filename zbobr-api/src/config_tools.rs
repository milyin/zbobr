use std::{fmt, str::FromStr};

/// All possible static MCP tools across all roles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum McpTool {
    GetHistory,
    ReportSuccess,
    ReportFailure,
    ReportIntermediate,
    GetFullReport,
    StopWithError,
    StopWithQuestion,
    ConfigureWorktree,
}

impl McpTool {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GetHistory => "get_history",
            Self::ReportSuccess => "report_success",
            Self::ReportFailure => "report_failure",
            Self::ReportIntermediate => "report_intermediate",
            Self::GetFullReport => "get_full_report",
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
            "get_history" => Ok(Self::GetHistory),
            "report_success" => Ok(Self::ReportSuccess),
            "report_failure" => Ok(Self::ReportFailure),
            "report_intermediate" => Ok(Self::ReportIntermediate),
            "get_full_report" => Ok(Self::GetFullReport),
            "stop_with_error" => Ok(Self::StopWithError),
            "stop_with_question" => Ok(Self::StopWithQuestion),
            "configure_worktree" => Ok(Self::ConfigureWorktree),
            other => Err(format!("unknown MCP tool: {other}")),
        }
    }
}

pub const ALL_TOOLS: &[McpTool] = &[
    McpTool::GetHistory,
    McpTool::ReportSuccess,
    McpTool::ReportFailure,
    McpTool::ReportIntermediate,
    McpTool::GetFullReport,
    McpTool::StopWithError,
    McpTool::StopWithQuestion,
    McpTool::ConfigureWorktree,
];

/// All possible static MCP tool names across all roles.
pub const ALL_TOOL_NAMES: &[&str] = &[
    McpTool::GetHistory.as_str(),
    McpTool::ReportSuccess.as_str(),
    McpTool::ReportFailure.as_str(),
    McpTool::ReportIntermediate.as_str(),
    McpTool::GetFullReport.as_str(),
    McpTool::StopWithError.as_str(),
    McpTool::StopWithQuestion.as_str(),
    McpTool::ConfigureWorktree.as_str(),
];
