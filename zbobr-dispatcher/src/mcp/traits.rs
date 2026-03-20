use crate::{
    mcp::common::get_hostname,
    task::{ChecklistItem, Model, RoleSession, Tool},
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

    /// Returns the name of the current stage, used to compute the retry signal
    fn stage_name(&self) -> &str;

    /// Record a tool call for transition mapping.
    fn record_tool(&self, tool_name: &str) {
        self.session().record_tool_call(tool_name);
    }

    // -- History tools --

    async fn get_history_index_impl(&self) -> String {
        tracing::info!(
            "[{}#{}] get_history_index",
            self.role_name(),
            self.session().task_id(),
        );

        match self.session().get_history_index().await {
            Ok(index) => {
                tracing::info!(
                    "[{}#{}] get_history_index returned {} entries",
                    self.role_name(),
                    self.session().task_id(),
                    index.entries.len()
                );
                match serde_json::to_string_pretty(&index) {
                    Ok(json) => json,
                    Err(e) => format!("Error serializing: {e}"),
                }
            }
            Err(e) => {
                let response = format!("Error: {e}");
                log_mcp_string_response(
                    self.role_name(),
                    self.session().task_id(),
                    "get_history_index",
                    &response,
                );
                response
            }
        }
    }

    async fn get_history_record_impl(&self, index: usize) -> String {
        tracing::info!(
            "[{}#{}] get_history_record index={}",
            self.role_name(),
            self.session().task_id(),
            index
        );

        match self.session().get_history_record(index).await {
            Ok(text) => {
                log_mcp_string_response(
                    self.role_name(),
                    self.session().task_id(),
                    "get_history_record",
                    &text,
                );
                text
            }
            Err(e) => {
                let response = format!("{e}");
                log_mcp_string_response(
                    self.role_name(),
                    self.session().task_id(),
                    "get_history_record",
                    &response,
                );
                response
            }
        }
    }

    // -- Report tools --

    async fn report_success_impl(&self, message: &str) -> String {
        tracing::info!(
            "[{}#{}] report_success",
            self.role_name(),
            self.session().task_id()
        );
        let hostname = get_hostname();
        let body = format!("[report_success]\n{message}");

        if let Err(e) = self
            .session()
            .post_comment(&body, self.stage_name(), &hostname, Some(self.mcp_tool()), Some(self.mcp_model()), true, false)
            .await
        {
            tracing::error!(
                "Failed to post success message for task {}: {e}",
                self.session().task_id()
            );
            let response = format!("Error posting success message: {e}");
            log_mcp_string_response(
                self.role_name(),
                self.session().task_id(),
                "report_success",
                &response,
            );
            return response;
        }

        self.record_tool("report_success");

        let response = "Results reported successfully".to_string();
        log_mcp_string_response(
            self.role_name(),
            self.session().task_id(),
            "report_success",
            &response,
        );
        response
    }

    async fn report_failure_impl(&self, message: &str) -> String {
        tracing::info!(
            "[{}#{}] report_failure",
            self.role_name(),
            self.session().task_id()
        );
        let hostname = get_hostname();
        let body = format!("[report_failure]\n{message}");

        if let Err(e) = self
            .session()
            .post_comment(&body, self.stage_name(), &hostname, Some(self.mcp_tool()), Some(self.mcp_model()), true, false)
            .await
        {
            tracing::error!(
                "Failed to post failure message for task {}: {e}",
                self.session().task_id()
            );
            let response = format!("Error posting failure message: {e}");
            log_mcp_string_response(
                self.role_name(),
                self.session().task_id(),
                "report_failure",
                &response,
            );
            return response;
        }

        self.record_tool("report_failure");

        let response = "Failure reported".to_string();
        log_mcp_string_response(
            self.role_name(),
            self.session().task_id(),
            "report_failure",
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
            .post_comment(&body, self.stage_name(), &hostname, Some(self.mcp_tool()), Some(self.mcp_model()), false, true)
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

        // Set the retry signal so the task returns to this stage after the user intervenes.
        let retry = format!("go_{}", self.stage_name());
        if let Err(e) = self.session().set_signal(&retry).await {
            tracing::warn!(
                "Failed to set retry signal for task {} after reporting error: {e}",
                self.session().task_id()
            );
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
            .post_comment(&body, self.stage_name(), &hostname, Some(self.mcp_tool()), Some(self.mcp_model()), false, false)
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

        // Set the retry signal so the task returns to this stage after the user responds.
        let retry = format!("go_{}", self.stage_name());
        if let Err(e) = self.session().set_signal(&retry).await {
            tracing::warn!(
                "Failed to set retry signal for task {} after asking user: {e}",
                self.session().task_id()
            );
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

        // Validate work_branch_postfix: if provided, work_branch must not already be set
        if work_branch_postfix.is_some() {
            match self.session().get_work_branch().await {
                Ok(Some(_)) => return "Error: work_branch is already set".to_string(),
                Err(e) => return format!("Error: {e}"),
                Ok(None) => {}
            }
        }

        let session = self.session();
        let work_branch = work_branch_postfix.map(|v| session.create_branch_name(&v));

        let response = match session
            .modify_task(move |mut task| {
                if let Some(repo) = destination_repository {
                    task.destination_repository = Some(repo);
                }
                if let Some(branch) = destination_branch {
                    task.destination_branch = Some(branch);
                }
                if let Some(wb) = work_branch {
                    task.work_branch = Some(wb);
                }
                task
            })
            .await
        {
            Ok(()) => "Worktree configured".to_string(),
            Err(e) => format!("Error: {e}"),
        };
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
        let item_id = id.to_string();
        let item_text = text.to_string();

        // Validate: id must be unique
        match self.session().get_checklist().await {
            Ok(items) => {
                if items.iter().any(|item| item.id == item_id) {
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

        let response = match self
            .session()
            .modify_task(move |mut task| {
                task.checklist.push(ChecklistItem {
                    id: item_id,
                    checked: false,
                    text: item_text,
                });
                task
            })
            .await
        {
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
        let item_id = id.to_string();
        let response = match self
            .session()
            .modify_task(move |mut task| {
                if let Some(item) = task.checklist.iter_mut().find(|item| item.id == item_id) {
                    item.checked = true;
                }
                task
            })
            .await
        {
            Ok(()) => format!("Checklist item '{}' checked", id),
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
        let item_id = id.to_string();

        // Pre-validate: check the item exists and is not checked
        match self.session().get_checklist().await {
            Ok(items) => {
                if let Some(item) = items.iter().find(|i| i.id == item_id) {
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

        let response = match self
            .session()
            .modify_task(move |mut task| {
                task.checklist.retain(|item| item.id != item_id);
                task
            })
            .await
        {
            Ok(()) => format!("Checklist item '{}' deleted", id),
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
