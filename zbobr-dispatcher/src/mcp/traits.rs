use zbobr_api::config_tools::McpTool;
use zbobr_api::task::ContextRecordType;

use crate::{
    mcp::common::{get_hostname, parse_ctx_rec_id},
    task::{Model, RoleSession, Tool},
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
        let display_str = if response.len() > 100 {
            format!("{}...", &response[..100])
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

    fn role_name(&self) -> &str;

    /// Returns the tool that is executing this MCP session
    fn mcp_tool(&self) -> Tool;

    /// Returns the concrete model currently in use by the agent tool
    fn mcp_model(&self) -> Model;

    /// Returns the name of the current stage.
    fn stage_name(&self) -> &str;

    /// Returns the pipeline name for this session.
    fn pipeline_name(&self) -> &str;

    /// Returns the pipeline run ID for this session.
    fn pipeline_run_id(&self) -> u64;

    /// Record a tool call for transition mapping.
    fn record_tool(&self, tool: McpTool) {
        self.session().record_tool_call(tool);
    }

    // -- Report tools --

    async fn report_impl(&self, tool: McpTool, brief: &str, full_report: &str) -> String {
        let tool_name = tool.as_str();
        tracing::info!(
            "[{}#{}] {}",
            self.role_name(),
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
            "report_{}_{}_{}_{}",
            self.pipeline_name(),
            self.pipeline_run_id(),
            self.stage_name(),
            tool_name,
        );
        let report_link = match self.session().store_report(&base_name, full_report).await {
            Ok(filename) => Some(filename),
            Err(e) => {
                let response = format!("Error storing report: {e}");
                log_mcp_string_response(
                    self.role_name(),
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
            log_mcp_string_response(
                self.role_name(),
                self.session().task_id(),
                tool_name,
                &response,
            );
            return response;
        }

        self.record_tool(tool);

        let response = "Report stored".to_string();
        log_mcp_string_response(
            self.role_name(),
            self.session().task_id(),
            tool_name,
            &response,
        );
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
            self.role_name(),
            self.session().task_id(),
            tool_name,
        );

        let base_name = format!(
            "checklist_{}_{}_{}_item",
            self.pipeline_name(),
            self.pipeline_run_id(),
            self.stage_name(),
        );
        let report_link = match self.session().store_report(&base_name, full_report).await {
            Ok(filename) => Some(filename),
            Err(e) => {
                let response = format!("Error storing full report: {e}");
                log_mcp_string_response(
                    self.role_name(),
                    self.session().task_id(),
                    tool_name,
                    &response,
                );
                return response;
            }
        };

        // Find the most recent report record to use as parent
        let parent_record_id = match self.session().get_task().await {
            Ok(task) => task.context.stages.last().and_then(|stage| {
                stage
                    .records
                    .iter()
                    .rev()
                    .find(|r| {
                        matches!(
                            r.record_type,
                            ContextRecordType::Success
                                | ContextRecordType::Failure
                                | ContextRecordType::Comment
                        )
                    })
                    .map(|r| r.id)
            }),
            Err(_) => None,
        };

        match self
            .session()
            .add_checkbox_record(brief.to_string(), report_link, parent_record_id)
            .await
        {
            Ok(id) => {
                let response = format!("Checklist item added (ctx_rec_{id})");
                log_mcp_string_response(
                    self.role_name(),
                    self.session().task_id(),
                    tool_name,
                    &response,
                );
                response
            }
            Err(e) => {
                let response = format!("Error: {e}");
                log_mcp_string_response(
                    self.role_name(),
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
            self.role_name(),
            self.session().task_id(),
            tool_name,
            id_str,
        );

        let record_id = match parse_ctx_rec_id(id_str) {
            Ok(id) => id,
            Err(e) => {
                let response = format!("Error: {e}");
                log_mcp_string_response(
                    self.role_name(),
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
                    self.role_name(),
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
                        self.role_name(),
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
                    self.role_name(),
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
                    self.role_name(),
                    self.session().task_id(),
                    tool_name,
                    &response,
                );
                response
            }
            Err(e) => {
                let response = format!("Error: {e}");
                log_mcp_string_response(
                    self.role_name(),
                    self.session().task_id(),
                    tool_name,
                    &response,
                );
                response
            }
        }
    }

    async fn delete_ctx_rec_impl(&self, id_str: &str) -> String {
        let tool_name = McpTool::DeleteCtxRec.as_str();
        tracing::info!(
            "[{}#{}] {} id={}",
            self.role_name(),
            self.session().task_id(),
            tool_name,
            id_str,
        );

        let record_id = match parse_ctx_rec_id(id_str) {
            Ok(id) => id,
            Err(e) => {
                let response = format!("Error: {e}");
                log_mcp_string_response(
                    self.role_name(),
                    self.session().task_id(),
                    tool_name,
                    &response,
                );
                return response;
            }
        };

        match self.session().delete_context_record(record_id).await {
            Ok(true) => {
                let response = format!("Record ctx_rec_{} deleted", record_id);
                log_mcp_string_response(
                    self.role_name(),
                    self.session().task_id(),
                    tool_name,
                    &response,
                );
                response
            }
            Ok(false) => {
                let response = format!("Error: record ctx_rec_{} not found", record_id);
                log_mcp_string_response(
                    self.role_name(),
                    self.session().task_id(),
                    tool_name,
                    &response,
                );
                response
            }
            Err(e) => {
                let response = format!("Error: {e}");
                log_mcp_string_response(
                    self.role_name(),
                    self.session().task_id(),
                    tool_name,
                    &response,
                );
                response
            }
        }
    }

    async fn stop_with_error_impl(&self, message: &str) -> String {
        tracing::info!(
            "[{}#{}] stop_with_error",
            self.role_name(),
            self.session().task_id()
        );

        if let Err(e) = self.session().set_error(Some(message.to_string())).await {
            tracing::error!(
                "Failed to set error for task {}: {e}",
                self.session().task_id()
            );
            let response = format!("Error setting error on task: {e}");
            log_mcp_string_response(
                self.role_name(),
                self.session().task_id(),
                McpTool::StopWithError.as_str(),
                &response,
            );
            return response;
        }

        // Set pause flag to stop task processing and wait for user response.
        if let Err(e) = self.session().set_pause(true).await {
            tracing::error!(
                "Failed to set pause for task {} after reporting error: {e}",
                self.session().task_id()
            );
            let response = format!("Error reporting error but error pausing task: {e}");
            log_mcp_string_response(
                self.role_name(),
                self.session().task_id(),
                McpTool::StopWithError.as_str(),
                &response,
            );
            return response;
        }

        let response = "Error reported to user - task paused pending response".to_string();
        log_mcp_string_response(
            self.role_name(),
            self.session().task_id(),
            McpTool::StopWithError.as_str(),
            &response,
        );
        response
    }

    async fn stop_with_question_impl(&self, message: &str) -> String {
        tracing::info!(
            "[{}#{}] stop_with_question",
            self.role_name(),
            self.session().task_id()
        );
        let hostname = get_hostname();
        let body = format!("[{}]\n{message}", McpTool::StopWithQuestion.as_str());

        if let Err(e) = self
            .session()
            .post_comment(
                &body,
                self.stage_name(),
                &hostname,
                Some(self.mcp_tool()),
                Some(self.mcp_model()),
                None,
            )
            .await
        {
            tracing::error!(
                "Failed to post question for task {}: {e}",
                self.session().task_id()
            );
            let response = format!("Error posting question: {e}");
            log_mcp_string_response(
                self.role_name(),
                self.session().task_id(),
                McpTool::StopWithQuestion.as_str(),
                &response,
            );
            return response;
        }

        // Set pause flag to stop task processing and wait for user response.
        if let Err(e) = self.session().set_pause(true).await {
            tracing::error!(
                "Failed to set pause for task {} after asking user: {e}",
                self.session().task_id()
            );
            let response = format!("Error asking user but error pausing task: {e}");
            log_mcp_string_response(
                self.role_name(),
                self.session().task_id(),
                McpTool::StopWithQuestion.as_str(),
                &response,
            );
            return response;
        }

        let response = "Question posted - task paused pending user response".to_string();
        log_mcp_string_response(
            self.role_name(),
            self.session().task_id(),
            McpTool::StopWithQuestion.as_str(),
            &response,
        );
        response
    }

    // -- Worktree configuration --

    async fn configure_worktree_impl(
        &self,
        destination_repository: Option<String>,
        destination_branch: Option<String>,
        work_branch_postfix: String,
    ) -> String {
        tracing::info!(
            "[{}#{}] configure_worktree repo={:?} branch={:?} postfix={:?}",
            self.role_name(),
            self.session().task_id(),
            destination_repository,
            destination_branch,
            work_branch_postfix,
        );

        if work_branch_postfix.trim().is_empty() {
            return self
                .configure_worktree_error(
                    "work_branch_postfix is required and must not be empty".to_string(),
                )
                .await;
        }

        let session = self.session();
        let config = session.dispatcher_config();

        // Apply defaults from config when agent doesn't provide values
        let effective_repo =
            destination_repository.or_else(|| config.default_destination_repository.clone());
        let effective_branch =
            destination_branch.or_else(|| config.default_destination_branch.clone());
        let work_branch = Some(session.create_branch_name(&work_branch_postfix));

        // Validate work_branch_postfix: if provided and work_branch is already set,
        // treat identical repeated setup as a successful no-op.
        if let Some(_requested_work_branch) = work_branch.as_deref() {
            match session.get_work_branch().await {
                Ok(Some(existing_work_branch)) => {
                    let existing_repo = match session.get_destination_repository().await {
                        Ok(v) => v,
                        Err(e) => return self.configure_worktree_error(e.to_string()).await,
                    };
                    let existing_branch = match session.get_destination_branch().await {
                        Ok(v) => v,
                        Err(e) => return self.configure_worktree_error(e.to_string()).await,
                    };

                    // If work_branch is already set, ignore requested values and return current config.
                    let response = format!(
                        "Worktree already configured: destination_repository={}, destination_branch={}, work_branch={} (requested values ignored)",
                        existing_repo.as_deref().unwrap_or("(not set)"),
                        existing_branch.as_deref().unwrap_or("(not set)"),
                        existing_work_branch,
                    );
                    log_mcp_string_response(
                        self.role_name(),
                        self.session().task_id(),
                        McpTool::ConfigureWorktree.as_str(),
                        &response,
                    );
                    return response;
                }
                Err(e) => {
                    return self.configure_worktree_error(e.to_string()).await;
                }
                Ok(None) => {}
            }
        }

        let repo_for_response = effective_repo.clone();
        let branch_for_response = effective_branch.clone();
        let wb_for_response = work_branch.clone();

        let response = match session
            .modify_task(move |mut task| {
                if let Some(repo) = effective_repo {
                    task.destination_repository = Some(repo);
                }
                if let Some(branch) = effective_branch {
                    task.destination_branch = Some(branch);
                }
                if let Some(wb) = work_branch {
                    task.work_branch = Some(wb);
                }
                task
            })
            .await
        {
            Ok(()) => {
                format!(
                    "Worktree configured: destination_repository={}, destination_branch={}, work_branch={}",
                    repo_for_response.as_deref().unwrap_or("(not set)"),
                    branch_for_response.as_deref().unwrap_or("(not set)"),
                    wb_for_response.as_deref().unwrap_or("(not set)"),
                )
            }
            Err(e) => return self.configure_worktree_error(e.to_string()).await,
        };
        log_mcp_string_response(
            self.role_name(),
            self.session().task_id(),
            McpTool::ConfigureWorktree.as_str(),
            &response,
        );
        response
    }

    async fn configure_worktree_error(&self, error: String) -> String {
        if let Err(pause_err) = self.session().set_pause(true).await {
            tracing::error!("Failed to pause task after configure_worktree error: {pause_err}");
        }
        let response = format!("Error: {error}");
        log_mcp_string_response(
            self.role_name(),
            self.session().task_id(),
            McpTool::ConfigureWorktree.as_str(),
            &response,
        );
        response
    }
}
