use crate::{
    mcp::common::get_hostname,
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

/// Log a JSON response (e.g., checklist items).
fn log_mcp_json_response(role_name: &str, task_id: u64, tool_name: &str, response: &str) {
    tracing::debug!(
        "[{}#{}] {} response: {}",
        role_name,
        task_id,
        tool_name,
        response
    );

    if response.starts_with('[') {
        if let Ok(items) = serde_json::from_str::<Vec<serde_json::Value>>(response) {
            tracing::info!(
                "[{}#{}] {} returned {} item(s)",
                role_name,
                task_id,
                tool_name,
                items.len()
            );
        } else {
            tracing::info!(
                "[{}#{}] {} response (failed to parse): {}",
                role_name,
                task_id,
                tool_name,
                response
            );
        }
    } else if response.starts_with("Error") {
        tracing::info!(
            "[{}#{}] {} error: {}",
            role_name,
            task_id,
            tool_name,
            response
        );
    } else {
        tracing::info!("[{}#{}] {}: {}", role_name, task_id, tool_name, response);
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
    fn record_tool(&self, tool_name: &str) {
        self.session().record_tool_call(tool_name);
    }

    // -- History tools --

    async fn get_history_impl(&self) -> String {
        tracing::info!(
            "[{}#{}] get_history",
            self.role_name(),
            self.session().task_id(),
        );

        match self
            .session()
            .get_history_for_run(self.pipeline_run_id())
            .await
        {
            Ok(text) => {
                log_mcp_string_response(
                    self.role_name(),
                    self.session().task_id(),
                    "get_history",
                    &text,
                );
                text
            }
            Err(e) => {
                let response = format!("Error: {e}");
                log_mcp_string_response(
                    self.role_name(),
                    self.session().task_id(),
                    "get_history",
                    &response,
                );
                response
            }
        }
    }

    // -- Report tools --

    async fn report_impl(&self, tool_name: &str, brief: &str, full_report: &str) -> String {
        tracing::info!(
            "[{}#{}] {}",
            self.role_name(),
            self.session().task_id(),
            tool_name,
        );

        let hostname = get_hostname();
        let body = format!("[{tool_name}]\n{brief}");

        if let Err(e) = self
            .session()
            .post_comment(
                &body,
                self.stage_name(),
                &hostname,
                Some(self.mcp_tool()),
                Some(self.mcp_model()),
                Some(full_report),
            )
            .await
        {
            tracing::error!(
                "Failed to post {tool_name} message for task {}: {e}",
                self.session().task_id()
            );
            let response = format!("Error posting {tool_name} message: {e}");
            log_mcp_string_response(
                self.role_name(),
                self.session().task_id(),
                tool_name,
                &response,
            );
            return response;
        }

        self.record_tool(tool_name);

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
        self.report_impl("report_success", brief, full_report).await
    }

    async fn report_failure_impl(&self, brief: &str, full_report: &str) -> String {
        self.report_impl("report_failure", brief, full_report).await
    }

    async fn get_full_report_impl(&self, name: &str) -> String {
        tracing::info!(
            "[{}#{}] get_full_report name={}",
            self.role_name(),
            self.session().task_id(),
            name,
        );

        let response = match self.session().read_report(name).await {
            Ok(content) => content,
            Err(e) => format!("Error: {e}"),
        };
        log_mcp_string_response(
            self.role_name(),
            self.session().task_id(),
            "get_full_report",
            &response,
        );
        response
    }

    async fn stop_with_error_impl(&self, message: &str) -> String {
        tracing::info!(
            "[{}#{}] stop_with_error",
            self.role_name(),
            self.session().task_id()
        );
        let hostname = get_hostname();
        let body = format!("[stop_with_error]\n{message}");

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
                "Failed to post error message for task {}: {e}",
                self.session().task_id()
            );
            let response = format!("Error posting error message: {e}");
            log_mcp_string_response(
                self.role_name(),
                self.session().task_id(),
                "stop_with_error",
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
                "stop_with_error",
                &response,
            );
            return response;
        }

        let response = "Error reported to user - task paused pending response".to_string();
        log_mcp_string_response(
            self.role_name(),
            self.session().task_id(),
            "stop_with_error",
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
        let body = format!("[stop_with_question]\n{message}");

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
                "stop_with_question",
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
                "stop_with_question",
                &response,
            );
            return response;
        }

        let response = "Question posted - task paused pending user response".to_string();
        log_mcp_string_response(
            self.role_name(),
            self.session().task_id(),
            "stop_with_question",
            &response,
        );
        response
    }

    // -- Worktree configuration --

    async fn configure_worktree_impl(
        &self,
        destination_repository: Option<String>,
        destination_branch: Option<String>,
        work_branch_postfix: Option<String>,
    ) -> String {
        tracing::info!(
            "[{}#{}] configure_worktree repo={:?} branch={:?} postfix={:?}",
            self.role_name(),
            self.session().task_id(),
            destination_repository,
            destination_branch,
            work_branch_postfix,
        );

        let session = self.session();
        let config = session.dispatcher_config();

        // Apply defaults from config when agent doesn't provide values
        let effective_repo = destination_repository
            .or_else(|| config.default_destination_repository.clone());
        let effective_branch = destination_branch
            .or_else(|| config.default_destination_branch.clone());
        let work_branch = work_branch_postfix.map(|v| session.create_branch_name(&v));

        // Validate work_branch_postfix: if provided and work_branch is already set,
        // treat identical repeated setup as a successful no-op.
        if let Some(requested_work_branch) = work_branch.as_deref() {
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

                    let repo_matches = effective_repo
                        .as_ref()
                        .map(|v| Some(v) == existing_repo.as_ref())
                        .unwrap_or(true);
                    let branch_matches = effective_branch
                        .as_ref()
                        .map(|v| Some(v) == existing_branch.as_ref())
                        .unwrap_or(true);

                    if existing_work_branch == requested_work_branch
                        && repo_matches
                        && branch_matches
                    {
                        let response = format!(
                            "Worktree configured: destination_repository={}, destination_branch={}, work_branch={} (values were already set)",
                            existing_repo.as_deref().unwrap_or("(not set)"),
                            existing_branch.as_deref().unwrap_or("(not set)"),
                            existing_work_branch,
                        );
                        log_mcp_string_response(
                            self.role_name(),
                            self.session().task_id(),
                            "configure_worktree",
                            &response,
                        );
                        return response;
                    }

                    return self
                        .configure_worktree_error(
                            "work_branch is already set and differs from requested values"
                                .to_string(),
                        )
                        .await;
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
            "configure_worktree",
            &response,
        );
        response
    }

    async fn configure_worktree_error(&self, error: String) -> String {
        if let Err(pause_err) = self.session().set_pause(true).await {
            tracing::error!(
                "Failed to pause task after configure_worktree error: {pause_err}"
            );
        }
        let response = format!("Error: {error}");
        log_mcp_string_response(
            self.role_name(),
            self.session().task_id(),
            "configure_worktree",
            &response,
        );
        response
    }

    // -- Checklist tools --

    async fn get_checklist_impl(&self) -> String {
        tracing::info!(
            "[{}#{}] get_checklist",
            self.role_name(),
            self.session().task_id()
        );
        let response = match self.session().get_checklist().await {
            Ok(items) => {
                // Filter to unchecked items only
                let unchecked: Vec<_> = items.into_iter().filter(|i| !i.checked).collect();
                match serde_json::to_string_pretty(&unchecked) {
                    Ok(json) => json,
                    Err(e) => format!("Error serializing checklist: {e}"),
                }
            }
            Err(e) => format!("Error: {e}"),
        };
        log_mcp_json_response(
            self.role_name(),
            self.session().task_id(),
            "get_checklist",
            &response,
        );
        response
    }

    async fn add_checklist_item_impl(&self, id: &str, text: &str) -> String {
        tracing::info!(
            "[{}#{}] add_checklist_item id={}",
            self.role_name(),
            self.session().task_id(),
            id
        );
        // Validate: id must be unique
        match self.session().get_checklist().await {
            Ok(items) => {
                if items.iter().any(|item| item.id == id) {
                    let response = format!("Error: Checklist item with id '{}' already exists", id);
                    log_mcp_string_response(
                        self.role_name(),
                        self.session().task_id(),
                        "add_checklist_item",
                        &response,
                    );
                    return response;
                }
            }
            Err(e) => {
                let response = format!("Error: {e}");
                log_mcp_string_response(
                    self.role_name(),
                    self.session().task_id(),
                    "add_checklist_item",
                    &response,
                );
                return response;
            }
        }

        let response = match self.session().add_checklist_item(id, text).await {
            Ok(()) => format!("Checklist item '{}' added", id),
            Err(e) => format!("Error updating task: {e}"),
        };
        log_mcp_string_response(
            self.role_name(),
            self.session().task_id(),
            "add_checklist_item",
            &response,
        );
        response
    }

    async fn check_checklist_item_impl(&self, id: &str) -> String {
        tracing::info!(
            "[{}#{}] check_checklist_item id={}",
            self.role_name(),
            self.session().task_id(),
            id,
        );
        let response = match self.session().check_checklist_item(id).await {
            Ok(true) => format!("Checklist item '{}' checked", id),
            Ok(false) => format!("Error: Checklist item with id '{}' not found", id),
            Err(e) => format!("Error: {e}"),
        };
        log_mcp_string_response(
            self.role_name(),
            self.session().task_id(),
            "check_checklist_item",
            &response,
        );
        response
    }

    async fn delete_checklist_item_impl(&self, id: &str) -> String {
        tracing::info!(
            "[{}#{}] delete_checklist_item id={}",
            self.role_name(),
            self.session().task_id(),
            id
        );
        // Pre-validate: check the item exists and is not checked
        match self.session().get_checklist().await {
            Ok(items) => {
                if let Some(item) = items.iter().find(|i| i.id == id) {
                    if item.checked {
                        let response = format!(
                            "Error: Cannot delete checked checklist item '{}'. Checked items are preserved as work history.",
                            id
                        );
                        log_mcp_string_response(
                            self.role_name(),
                            self.session().task_id(),
                            "delete_checklist_item",
                            &response,
                        );
                        return response;
                    }
                } else {
                    let response = format!("Error: Checklist item with id '{}' not found", id);
                    log_mcp_string_response(
                        self.role_name(),
                        self.session().task_id(),
                        "delete_checklist_item",
                        &response,
                    );
                    return response;
                }
            }
            Err(e) => {
                let response = format!("Error: {e}");
                log_mcp_string_response(
                    self.role_name(),
                    self.session().task_id(),
                    "delete_checklist_item",
                    &response,
                );
                return response;
            }
        }

        let response = match self.session().delete_checklist_item(id).await {
            Ok(true) => format!("Checklist item '{}' deleted", id),
            Ok(false) => format!("Error: Checklist item with id '{}' not found", id),
            Err(e) => format!("Error updating task: {e}"),
        };
        log_mcp_string_response(
            self.role_name(),
            self.session().task_id(),
            "delete_checklist_item",
            &response,
        );
        response
    }
}
