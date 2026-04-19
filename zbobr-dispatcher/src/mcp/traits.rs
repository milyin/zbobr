use zbobr_api::{Pipeline, Stage, config::Role, config_tools::McpTool, task::ContextRecordType};

use crate::{
    mcp::common::parse_ctx_rec_id,
    task::{Executor, Model, RoleSession},
};

/// Log a string response from MCP methods.
fn log_mcp_string_response(role_name: &str, task_id: u64, tool_name: &str, response: &str) {
    tracing::debug!(
        "[{}#{}] {} response: {}",
        role_name,
        task_id,
        tool_name,
        response
    );

    if response.starts_with("Error") {
        tracing::info!(
            "[{}#{}] {} error: {}",
            role_name,
            task_id,
            tool_name,
            response
        );
    } else {
        let display_str = if response.chars().count() > 100 {
            format!("{}...", response.chars().take(100).collect::<String>())
        } else {
            response.to_string()
        };
        tracing::info!(
            "[{}#{}] {} result: {}",
            role_name,
            task_id,
            tool_name,
            display_str
        );
    }
}

/// Common trait for all MCP services — unified across all roles.
/// Per-role traits have been removed; all tool implementations live here.
#[allow(async_fn_in_trait)]
pub trait CommonMcpImpl: Send + Sync {
    fn session(&self) -> &RoleSession;

    fn role(&self) -> &Role;

    /// Returns the tool that is executing this MCP session
    fn executor(&self) -> &Executor;

    /// Returns the concrete model currently in use by the agent tool
    fn model(&self) -> &Model;

    /// Returns the name of the current stage.
    fn stage(&self) -> &Stage;

    /// Returns the pipeline name for this session.
    fn pipeline(&self) -> &Pipeline;

    /// Record a tool call for transition mapping.
    fn record_tool(&self, tool: McpTool) {
        self.session().record_tool_call(tool);
    }

    // -- Report tools --

    async fn report_impl(&self, tool: McpTool, brief: &str, full_report: &str) -> String {
        let tool_name = tool.as_str();
        tracing::info!(
            "[{}#{}] {}",
            self.role(),
            self.session().task_id(),
            tool_name,
        );

        // Determine context record type from the tool
        let record_type = match tool {
            McpTool::ReportSuccess => ContextRecordType::Success,
            McpTool::ReportFailure => ContextRecordType::Failure,
            _ => ContextRecordType::Comment,
        };

        // Store the report file and get the link
        let base_name = format!(
            "report_{}_{}_{}",
            self.pipeline(),
            self.stage(),
            tool_name,
        );
        let report_link = match self.session().store_report(&base_name, full_report).await {
            Ok(filename) => Some(filename),
            Err(e) => {
                let response = format!("Error storing report: {e}");
                log_mcp_string_response(
                    self.role(),
                    self.session().task_id(),
                    tool_name,
                    &response,
                );
                return response;
            }
        };

        // Add context record (results stored in context, not as comments)
        if let Err(e) = self
            .session()
            .add_context_record(record_type, brief.to_string(), report_link)
            .await
        {
            let response = format!("Error adding context record: {e}");
            log_mcp_string_response(self.role(), self.session().task_id(), tool_name, &response);
            return response;
        }

        self.record_tool(tool);

        let response = "Report stored".to_string();
        log_mcp_string_response(self.role(), self.session().task_id(), tool_name, &response);
        response
    }

    async fn report_success_impl(&self, brief: &str, full_report: &str) -> String {
        self.report_impl(McpTool::ReportSuccess, brief, full_report)
            .await
    }

    async fn report_failure_impl(&self, brief: &str, full_report: &str) -> String {
        self.report_impl(McpTool::ReportFailure, brief, full_report)
            .await
    }

    async fn report_intermediate_impl(&self, brief: &str, full_report: &str) -> String {
        self.report_impl(McpTool::ReportIntermediate, brief, full_report)
            .await
    }

    // -- Checklist / context record tools --

    async fn add_checklist_item_impl(&self, brief: &str, full_report: &str) -> String {
        let tool_name = McpTool::AddChecklistItem.as_str();
        tracing::info!(
            "[{}#{}] {}",
            self.role(),
            self.session().task_id(),
            tool_name,
        );

        let base_name = format!(
            "checklist_{}_{}_item",
            self.pipeline(),
            self.stage(),
        );
        let report_link = match self.session().store_report(&base_name, full_report).await {
            Ok(filename) => Some(filename),
            Err(e) => {
                let response = format!("Error storing full report: {e}");
                log_mcp_string_response(
                    self.role(),
                    self.session().task_id(),
                    tool_name,
                    &response,
                );
                return response;
            }
        };

        // Find the most recent report record to use as parent
        match self
            .session()
            .add_checkbox_record(brief.to_string(), report_link)
            .await
        {
            Ok(id) => {
                let response = format!("Checklist item added (ctx_rec_{id})");
                log_mcp_string_response(
                    self.role(),
                    self.session().task_id(),
                    tool_name,
                    &response,
                );
                response
            }
            Err(e) => {
                let response = format!("Error: {e}");
                log_mcp_string_response(
                    self.role(),
                    self.session().task_id(),
                    tool_name,
                    &response,
                );
                response
            }
        }
    }

    async fn check_checklist_item_impl(&self, id_str: &str) -> String {
        let tool_name = McpTool::CheckChecklistItem.as_str();
        tracing::info!(
            "[{}#{}] {} id={}",
            self.role(),
            self.session().task_id(),
            tool_name,
            id_str,
        );

        let record_id = match parse_ctx_rec_id(id_str) {
            Ok(id) => id,
            Err(e) => {
                let response = format!("Error: {e}");
                log_mcp_string_response(
                    self.role(),
                    self.session().task_id(),
                    tool_name,
                    &response,
                );
                return response;
            }
        };

        // Verify the record exists and is a checkbox
        let task = match self.session().get_task().await {
            Ok(t) => t,
            Err(e) => {
                let response = format!("Error: {e}");
                log_mcp_string_response(
                    self.role(),
                    self.session().task_id(),
                    tool_name,
                    &response,
                );
                return response;
            }
        };

        match task.context.find_record(record_id) {
            Some((_, record)) => {
                if !matches!(record.record_type, ContextRecordType::Checkbox(_)) {
                    let response = format!(
                        "Error: record ctx_rec_{} is not a checkbox (it is a {})",
                        record_id, record.record_type
                    );
                    log_mcp_string_response(
                        self.role(),
                        self.session().task_id(),
                        tool_name,
                        &response,
                    );
                    return response;
                }
            }
            None => {
                let response = format!("Error: record ctx_rec_{} not found", record_id);
                log_mcp_string_response(
                    self.role(),
                    self.session().task_id(),
                    tool_name,
                    &response,
                );
                return response;
            }
        }

        match self.session().check_checkbox_record(record_id).await {
            Ok(()) => {
                let response = format!("Checklist item ctx_rec_{} checked", record_id);
                log_mcp_string_response(
                    self.role(),
                    self.session().task_id(),
                    tool_name,
                    &response,
                );
                response
            }
            Err(e) => {
                let response = format!("Error: {e}");
                log_mcp_string_response(
                    self.role(),
                    self.session().task_id(),
                    tool_name,
                    &response,
                );
                response
            }
        }
    }

    async fn get_ctx_rec_impl(&self, id_str: &str) -> String {
        let tool_name = McpTool::GetCtxRec.as_str();
        tracing::info!(
            "[{}#{}] {} id={}",
            self.role(),
            self.session().task_id(),
            tool_name,
            id_str,
        );

        let record_id = match parse_ctx_rec_id(id_str) {
            Ok(id) => id,
            Err(e) => {
                let response = format!("Error: {e}");
                log_mcp_string_response(
                    self.role(),
                    self.session().task_id(),
                    tool_name,
                    &response,
                );
                return response;
            }
        };

        match self.session().get_context_record_content(record_id).await {
            Ok(Some(content)) => {
                log_mcp_string_response(self.role(), self.session().task_id(), tool_name, &content);
                content
            }
            Ok(None) => {
                let response = format!("Error: record ctx_rec_{} not found", record_id);
                log_mcp_string_response(
                    self.role(),
                    self.session().task_id(),
                    tool_name,
                    &response,
                );
                response
            }
            Err(e) => {
                let response = format!("Error: {e}");
                log_mcp_string_response(
                    self.role(),
                    self.session().task_id(),
                    tool_name,
                    &response,
                );
                response
            }
        }
    }

    /// Shared implementation: format a status message, pause the task, and optionally add a
    /// context record (for questions). For errors (`add_context_record = false`), only the
    /// STATUS field is set. For questions (`add_context_record = true`), a Question context
    /// record is also added so the question appears in the agent report.
    async fn pause_with_status_impl(
        &self,
        tool: McpTool,
        icon: char,
        message: &str,
        add_context_record: bool,
    ) -> String {
        let ts = chrono::Utc::now().with_timezone(&self.session().config().fixed_offset());
        let status = zbobr_api::format_status(icon, &ts, message);

        if let Err(e) = self.session().set_pause_with_status(status).await {
            tracing::error!(
                "Failed to set pause+status for task {}: {e}",
                self.session().task_id()
            );
            let response = format!("Error setting status on task: {e}");
            log_mcp_string_response(
                self.role(),
                self.session().task_id(),
                tool.as_str(),
                &response,
            );
            return response;
        }

        if add_context_record
            && let Err(e) = self
                .session()
                .add_context_record(
                    zbobr_api::task::ContextRecordType::Question,
                    message.to_string(),
                    None,
                )
                .await
        {
            tracing::error!(
                "Failed to add question context record for task {}: {e}",
                self.session().task_id()
            );
            let response = format!("Error adding question to context: {e}");
            log_mcp_string_response(
                self.role(),
                self.session().task_id(),
                tool.as_str(),
                &response,
            );
            return response;
        }

        let response = if add_context_record {
            "Question recorded - task paused pending user response".to_string()
        } else {
            "Error reported - task paused pending response".to_string()
        };
        log_mcp_string_response(
            self.role(),
            self.session().task_id(),
            tool.as_str(),
            &response,
        );
        response
    }

    async fn stop_with_error_impl(&self, message: &str) -> String {
        tracing::info!(
            "[{}#{}] stop_with_error",
            self.role(),
            self.session().task_id()
        );
        self.pause_with_status_impl(
            McpTool::StopWithError,
            zbobr_api::ERROR_PREFIX,
            message,
            false,
        )
        .await
    }

    async fn stop_with_question_impl(&self, message: &str) -> String {
        tracing::info!(
            "[{}#{}] stop_with_question",
            self.role(),
            self.session().task_id()
        );
        self.pause_with_status_impl(
            McpTool::StopWithQuestion,
            zbobr_api::QUESTION_PREFIX,
            message,
            true,
        )
        .await
    }
}
