use crate::{
    CommentType, Signal,
    mcp::common::get_hostname,
    task::{ChecklistItem, Model, Role, RoleSession, Tool},
};

// Helper functions for logging MCP responses

/// Log a comments response (typically from get_history). Parses JSON and logs each comment.
fn log_mcp_comments_response(role_name: &str, task_id: u64, response: &str) {
    // Log exact JSON in debug level
    tracing::debug!("[{}#{}] response: {}", role_name, task_id, response);

    // Log info level with parsed comments
    if response.starts_with('{') {
        if let Ok(chunk) = serde_json::from_str::<zbobr_api::HistoryChunk>(response) {
            tracing::info!(
                "[{}#{}] history chunk {}/{} ({} comment(s))",
                role_name,
                task_id,
                chunk.current_chunk,
                chunk.last_chunk,
                chunk.comments.len()
            );
            for comment in chunk.comments {
                let stripped_text = comment.text.lines().next().unwrap_or("").trim();
                let display_text = if stripped_text.len() > 80 {
                    format!("{}...", &stripped_text[..80])
                } else {
                    stripped_text.to_string()
                };
                tracing::info!(
                    "[{}#{}] comment type={:?} text={}",
                    role_name,
                    task_id,
                    comment.comment_type,
                    display_text
                );
            }
        } else {
            tracing::info!(
                "[{}#{}] get_history response (failed to parse): {}",
                role_name,
                task_id,
                response
            );
        }
    } else if response.starts_with("Error") {
        tracing::info!(
            "[{}#{}] get_history error: {}",
            role_name,
            task_id,
            response
        );
    } else {
        tracing::info!(
            "[{}#{}] get_history response: {}",
            role_name,
            task_id,
            response
        );
    }
}

/// Log a JSON response (e.g., checklist items).
fn log_mcp_json_response(role_name: &str, task_id: u64, tool_name: &str, response: &str) {
    // Log exact JSON in debug level
    tracing::debug!(
        "[{}#{}] {} response: {}",
        role_name,
        task_id,
        tool_name,
        response
    );

    // Log info level with summary
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
    } else if response.starts_with('{') {
        if serde_json::from_str::<serde_json::Value>(response).is_ok() {
            tracing::info!("[{}#{}] {} succeeded", role_name, task_id, tool_name);
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

/// Log a string response from MCP methods.
fn log_mcp_string_response(role_name: &str, task_id: u64, tool_name: &str, response: &str) {
    // Log exact response in debug level
    tracing::debug!(
        "[{}#{}] {} response: {}",
        role_name,
        task_id,
        tool_name,
        response
    );

    // Log info level with key information
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

/// Common trait for MCP services (Planner, Worker) - shared implementations
#[allow(async_fn_in_trait)]
pub trait CommonMcpImpl: Send + Sync {
    fn session(&self) -> &RoleSession;
    fn role(&self) -> Role;

    /// Returns the tool that is executing this MCP session
    fn mcp_tool(&self) -> Tool;

    /// Returns the concrete model currently in use by the agent tool
    fn mcp_model(&self) -> Model;

    fn role_name(&self) -> &'static str {
        self.role().as_str()
    }

    /// Returns the signal that should re-trigger this role after an interruption
    /// (e.g. after `report_error` or `ask_user` pauses the task).
    fn retry_signal(&self) -> Signal {
        match self.role() {
            Role::Preparator => Signal::GoPrepare,
            Role::Planner => Signal::GoPlan,
            Role::Worker => Signal::GoWork,
            Role::Reviewer => Signal::GoReview,
            Role::Tester => Signal::GoTest,
            Role::Merger => Signal::GoWork,
        }
    }

    async fn get_history_impl(&self, offset: Option<usize>) -> String {
        tracing::info!(
            "[{}#{}] get_history offset={:?}",
            self.role_name(),
            self.session().task_id(),
            offset
        );

        match self.session().get_history(offset).await {
            Ok(chunk) => {
                if chunk.comments.is_empty() {
                    tracing::warn!(
                        "[{}#{}] get_history returned 0 comment(s) for offset={:?}",
                        self.role_name(),
                        self.session().task_id(),
                        offset
                    );
                } else {
                    tracing::info!(
                        "[{}#{}] get_history returned {} comment(s) for offset={:?} (chunk {}/{})",
                        self.role_name(),
                        self.session().task_id(),
                        chunk.comments.len(),
                        offset,
                        chunk.current_chunk,
                        chunk.last_chunk
                    );
                }
                let response = match serde_json::to_string_pretty(&chunk) {
                    Ok(json) => json,
                    Err(e) => format!("Error serializing: {e}"),
                };
                log_mcp_comments_response(self.role_name(), self.session().task_id(), &response);
                response
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

    async fn report_error_impl(&self, message: &str) -> String {
        tracing::info!(
            "[{}#{}] report_error",
            self.role_name(),
            self.session().task_id()
        );
        let hostname = get_hostname();

        if let Err(e) = self
            .session()
            .post_comment(
                CommentType::Error,
                message,
                Some(self.role()),
                &hostname,
                Some(self.mcp_tool()),
                Some(self.mcp_model()),
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
                "report_error",
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
                "report_error",
                &response,
            );
            return response;
        }

        // Set the retry signal so the task returns to this role after the user intervenes.
        if let Err(e) = self.session().set_signal(self.retry_signal()).await {
            tracing::warn!(
                "Failed to set retry signal for task {} after reporting error: {e}",
                self.session().task_id()
            );
        }

        let response = "Error reported to user - task paused pending response".to_string();
        log_mcp_string_response(
            self.role_name(),
            self.session().task_id(),
            "report_error",
            &response,
        );
        response
    }

    async fn report_results_impl(&self, message: &str) -> String {
        tracing::info!(
            "[{}#{}] report_results",
            self.role_name(),
            self.session().task_id()
        );
        let hostname = get_hostname();

        if let Err(e) = self
            .session()
            .post_comment(
                CommentType::Report,
                message,
                Some(self.role()),
                &hostname,
                Some(self.mcp_tool()),
                Some(self.mcp_model()),
            )
            .await
        {
            tracing::error!(
                "Failed to post results message for task {}: {e}",
                self.session().task_id()
            );
            let response = format!("Error posting results message: {e}");
            log_mcp_string_response(
                self.role_name(),
                self.session().task_id(),
                "report_results",
                &response,
            );
            return response;
        }

        let response = "Results reported successfully".to_string();
        log_mcp_string_response(
            self.role_name(),
            self.session().task_id(),
            "report_results",
            &response,
        );
        response
    }

    async fn ask_user_impl(&self, message: &str) -> String {
        tracing::info!(
            "[{}#{}] ask_user",
            self.role_name(),
            self.session().task_id()
        );
        let hostname = get_hostname();

        if let Err(e) = self
            .session()
            .post_comment(
                CommentType::Request,
                message,
                Some(self.role()),
                &hostname,
                Some(self.mcp_tool()),
                Some(self.mcp_model()),
            )
            .await
        {
            tracing::error!(
                "Failed to post ask_user message for task {}: {e}",
                self.session().task_id()
            );
            let response = format!("Error posting ask_user message: {e}");
            log_mcp_string_response(
                self.role_name(),
                self.session().task_id(),
                "ask_user",
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
                "ask_user",
                &response,
            );
            return response;
        }

        // Set the retry signal so the task returns to this role after the user responds.
        if let Err(e) = self.session().set_signal(self.retry_signal()).await {
            tracing::warn!(
                "Failed to set retry signal for task {} after asking user: {e}",
                self.session().task_id()
            );
        }

        let response = "User asked for guidance".to_string();
        log_mcp_string_response(
            self.role_name(),
            self.session().task_id(),
            "ask_user",
            &response,
        );
        response
    }

    async fn get_checklist_impl(&self) -> String {
        tracing::info!(
            "[{}#{}] get_checklist",
            self.role_name(),
            self.session().task_id()
        );
        let response = match self.session().get_checklist().await {
            Ok(items) => match serde_json::to_string_pretty(&items) {
                Ok(json) => json,
                Err(e) => format!("Error serializing checklist: {e}"),
            },
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

    async fn check_checklist_item_impl(&self, id: &str, checked: bool) -> String {
        tracing::info!(
            "[{}#{}] check_checklist_item id={} checked={}",
            self.role_name(),
            self.session().task_id(),
            id,
            checked
        );
        let item_id = id.to_string();
        let response = match self
            .session()
            .modify_task(move |mut task| {
                if let Some(item) = task.checklist.iter_mut().find(|item| item.id == item_id) {
                    item.checked = checked;
                }
                task
            })
            .await
        {
            Ok(()) => {
                // Checklist item state updated; signal transitions are handled by
                // the main/run loop after a role session completes. Do not set
                // task signal here to avoid racing state transitions.
                format!(
                    "Checklist item '{}' checked state updated to {}",
                    id, checked
                )
            }
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

    async fn insert_checklist_item_impl(
        &self,
        id: &str,
        after_id: Option<String>,
        text: &str,
    ) -> String {
        tracing::info!(
            "[{}#{}] insert_checklist_item id={} after_id={:?}",
            self.role_name(),
            self.session().task_id(),
            id,
            after_id
        );
        let item_id = id.to_string();
        let item_text = text.to_string();
        let after = after_id.clone();

        // Validate first by reading the task
        match self.session().get_checklist().await {
            Ok(items) => {
                if items.iter().any(|item| item.id == item_id) {
                    let response = format!("Error: Checklist item with id '{}' already exists", id);
                    log_mcp_string_response(
                        self.role_name(),
                        self.session().task_id(),
                        "insert_checklist_item",
                        &response,
                    );
                    return response;
                }
                if let Some(ref aid) = after
                    && !items.iter().any(|item| item.id == *aid)
                {
                    let response = format!("Error: Checklist item with id '{}' not found", aid);
                    log_mcp_string_response(
                        self.role_name(),
                        self.session().task_id(),
                        "insert_checklist_item",
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
                    "insert_checklist_item",
                    &response,
                );
                return response;
            }
        }

        let response = match self
            .session()
            .modify_task(move |mut task| {
                let new_item = ChecklistItem {
                    id: item_id,
                    checked: false,
                    text: item_text,
                };

                if let Some(ref after_id) = after {
                    if let Some(pos) = task.checklist.iter().position(|item| item.id == *after_id) {
                        task.checklist.insert(pos + 1, new_item);
                    } else {
                        task.checklist.push(new_item);
                    }
                } else {
                    task.checklist.push(new_item);
                }
                task
            })
            .await
        {
            Ok(()) => format!("Checklist item '{}' inserted", id),
            Err(e) => format!("Error updating task: {e}"),
        };
        log_mcp_string_response(
            self.role_name(),
            self.session().task_id(),
            "insert_checklist_item",
            &response,
        );
        response
    }

    async fn update_checklist_item_impl(&self, id: &str, text: &str) -> String {
        tracing::info!(
            "[{}#{}] update_checklist_item id={}",
            self.role_name(),
            self.session().task_id(),
            id
        );
        let item_id = id.to_string();
        let item_text = text.to_string();
        let response = match self
            .session()
            .modify_task(move |mut task| {
                if let Some(item) = task.checklist.iter_mut().find(|item| item.id == item_id) {
                    item.text = item_text;
                }
                task
            })
            .await
        {
            Ok(()) => format!("Checklist item '{}' updated", id),
            Err(e) => format!("Error updating task: {e}"),
        };
        log_mcp_string_response(
            self.role_name(),
            self.session().task_id(),
            "update_checklist_item",
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

    async fn get_destination_repository_impl(&self) -> String {
        tracing::info!(
            "[{}#{}] get_param_destination_repository",
            self.role_name(),
            self.session().task_id(),
        );
        let response = match self.session().get_destination_repository().await {
            Ok(Some(value)) => value,
            Ok(None) => "destination_repository is not set".to_string(),
            Err(e) => format!("Error: {e}"),
        };
        log_mcp_string_response(
            self.role_name(),
            self.session().task_id(),
            "get_param_destination_repository",
            &response,
        );
        response
    }

    async fn set_destination_repository_impl(&self, value: Option<String>) -> String {
        tracing::info!(
            "[{}#{}] set_param_destination_repository value={:?}",
            self.role_name(),
            self.session().task_id(),
            value
        );
        let response = match self.session().set_destination_repository(value).await {
            Ok(()) => "destination_repository updated".to_string(),
            Err(e) => format!("Error: {e}"),
        };
        log_mcp_string_response(
            self.role_name(),
            self.session().task_id(),
            "set_param_destination_repository",
            &response,
        );
        response
    }

    async fn get_destination_branch_impl(&self) -> String {
        tracing::info!(
            "[{}#{}] get_param_destination_branch",
            self.role_name(),
            self.session().task_id(),
        );
        let response = match self.session().get_destination_branch().await {
            Ok(Some(value)) => value,
            Ok(None) => "destination_branch is not set".to_string(),
            Err(e) => format!("Error: {e}"),
        };
        log_mcp_string_response(
            self.role_name(),
            self.session().task_id(),
            "get_param_destination_branch",
            &response,
        );
        response
    }

    async fn set_destination_branch_impl(&self, value: Option<String>) -> String {
        tracing::info!(
            "[{}#{}] set_param_destination_branch value={:?}",
            self.role_name(),
            self.session().task_id(),
            value
        );
        let response = match self.session().set_destination_branch(value).await {
            Ok(()) => "destination_branch updated".to_string(),
            Err(e) => format!("Error: {e}"),
        };
        log_mcp_string_response(
            self.role_name(),
            self.session().task_id(),
            "set_param_destination_branch",
            &response,
        );
        response
    }

    async fn get_work_branch_impl(&self) -> String {
        tracing::info!(
            "[{}#{}] get_param_work_branch",
            self.role_name(),
            self.session().task_id(),
        );
        let response = match self.session().get_work_branch().await {
            Ok(Some(value)) => value,
            Ok(None) => "work_branch is not set".to_string(),
            Err(e) => format!("Error: {e}"),
        };
        log_mcp_string_response(
            self.role_name(),
            self.session().task_id(),
            "get_param_work_branch",
            &response,
        );
        response
    }

    async fn set_work_branch_postfix_impl(&self, value: Option<String>) -> String {
        match self.session().get_work_branch().await {
            Ok(Some(_)) => return "Error: work_branch is already set".to_string(),
            Err(e) => return format!("Error: {e}"),
            Ok(None) => {}
        }
        let branch = value.map(|v| self.session().create_branch_name(&v));
        tracing::info!(
            "[{}#{}] set_param_work_branch value={:?}",
            self.role_name(),
            self.session().task_id(),
            branch
        );
        let response = match self.session().set_work_branch(branch).await {
            Ok(()) => "work_branch updated".to_string(),
            Err(e) => format!("Error: {e}"),
        };
        log_mcp_string_response(
            self.role_name(),
            self.session().task_id(),
            "set_param_work_branch",
            &response,
        );
        response
    }
}

/// Preparator-specific MCP implementations
#[allow(async_fn_in_trait)]
pub trait PreparatorMcpImpl: CommonMcpImpl {
    async fn get_param_destination_repository_impl(&self) -> String {
        self.get_destination_repository_impl().await
    }

    async fn set_param_destination_repository_impl(&self, value: Option<String>) -> String {
        self.set_destination_repository_impl(value).await
    }

    async fn get_param_destination_branch_impl(&self) -> String {
        self.get_destination_branch_impl().await
    }

    async fn set_param_destination_branch_impl(&self, value: Option<String>) -> String {
        self.set_destination_branch_impl(value).await
    }

    async fn get_param_work_branch_impl(&self) -> String {
        self.get_work_branch_impl().await
    }

    async fn set_param_work_branch_postfix_impl(&self, value: Option<String>) -> String {
        self.set_work_branch_postfix_impl(value).await
    }
}

/// Planner-specific MCP implementations
#[allow(async_fn_in_trait)]
pub trait PlannerMcpImpl: CommonMcpImpl {
    async fn post_plan_impl(&self, plan: &str) -> String {
        tracing::info!("[planner#{}] post_plan", self.session().task_id());
        let hostname = get_hostname();

        // Post the plan as a PLAN comment to preserve history
        if let Err(e) = self
            .session()
            .post_comment(
                CommentType::Plan,
                plan,
                Some(self.role()),
                &hostname,
                Some(self.mcp_tool()),
                None,
            )
            .await
        {
            tracing::error!(
                "Failed to post plan comment for task {}: {e}",
                self.session().task_id()
            );
            let response = format!("Error posting plan: {e}");
            log_mcp_string_response(
                self.role_name(),
                self.session().task_id(),
                "post_plan",
                &response,
            );
            return response;
        }

        let response = "Plan posted and task ready for worker implementation".to_string();
        log_mcp_string_response(
            self.role_name(),
            self.session().task_id(),
            "post_plan",
            &response,
        );
        response
    }

    async fn get_param_destination_branch_impl(&self) -> String {
        self.get_destination_branch_impl().await
    }

    async fn get_param_work_branch_impl(&self) -> String {
        self.get_work_branch_impl().await
    }
}

/// Worker-specific MCP implementations
#[allow(async_fn_in_trait)]
pub trait WorkerMcpImpl: CommonMcpImpl {
    async fn get_param_destination_branch_impl(&self) -> String {
        self.get_destination_branch_impl().await
    }

    async fn get_param_work_branch_impl(&self) -> String {
        self.get_work_branch_impl().await
    }

    async fn ask_planner_impl(&self, message: &str) -> String {
        tracing::info!("[worker#{}] ask_planner", self.session().task_id());
        let hostname = get_hostname();

        if let Err(e) = self
            .session()
            .post_comment(
                CommentType::Request,
                message,
                Some(self.role()),
                &hostname,
                Some(self.mcp_tool()),
                Some(self.mcp_model()),
            )
            .await
        {
            tracing::error!(
                "Failed to post worker->planner message for task {}: {e}",
                self.session().task_id()
            );
            let response = format!("Error posting message: {e}");
            log_mcp_string_response(
                self.role_name(),
                self.session().task_id(),
                "ask_planner",
                &response,
            );
            return response;
        }

        // Pass task back to planner agent for clarification or re-planning
        if let Err(e) = self.session().set_signal(crate::Signal::GoPlan).await {
            tracing::error!(
                "Failed to set signal GoPlan for task {} after ask_planner: {e}",
                self.session().task_id()
            );
            let response = format!("Message posted but error returning to planner: {e}");
            log_mcp_string_response(
                self.role_name(),
                self.session().task_id(),
                "ask_planner",
                &response,
            );
            return response;
        }
        let response = "Message posted to planner - task returned for clarification".to_string();
        log_mcp_string_response(
            self.role_name(),
            self.session().task_id(),
            "ask_planner",
            &response,
        );
        response
    }
}

/// Reviewer-specific MCP implementations
#[allow(async_fn_in_trait)]
pub trait ReviewerMcpImpl: CommonMcpImpl {
    async fn get_param_destination_branch_impl(&self) -> String {
        self.get_destination_branch_impl().await
    }

    async fn get_param_work_branch_impl(&self) -> String {
        self.get_work_branch_impl().await
    }

    async fn review_accept_impl(&self, message: &str) -> String {
        tracing::info!(
            "[{}#{}] review_accept",
            self.role_name(),
            self.session().task_id()
        );
        let hostname = get_hostname();
        let response = if let Err(e) = self
            .session()
            .post_comment(
                CommentType::Report,
                message,
                Some(self.role()),
                &hostname,
                Some(self.mcp_tool()),
                Some(self.mcp_model()),
            )
            .await
        {
            format!("Error posting review acceptance: {e}")
        } else {
            // No signal set — finalize_session will call finish when signal is None.
            "Review accepted — task will be marked done".to_string()
        };
        log_mcp_string_response(
            self.role_name(),
            self.session().task_id(),
            "review_accept",
            &response,
        );
        response
    }

    async fn review_reject_impl(&self, message: &str) -> String {
        tracing::info!(
            "[{}#{}] review_reject",
            self.role_name(),
            self.session().task_id()
        );
        let hostname = get_hostname();
        if let Err(e) = self
            .session()
            .post_comment(
                CommentType::Reject,
                message,
                Some(self.role()),
                &hostname,
                Some(self.mcp_tool()),
                Some(self.mcp_model()),
            )
            .await
        {
            let response = format!("Error posting review rejection: {e}");
            log_mcp_string_response(
                self.role_name(),
                self.session().task_id(),
                "review_reject",
                &response,
            );
            return response;
        }
        if let Err(e) = self.session().set_signal(crate::Signal::GoPlan).await {
            tracing::warn!(
                "Failed to set GoPlan signal for task {}: {e}",
                self.session().task_id()
            );
        }
        let response = "Review rejected — task routed back to planner".to_string();
        log_mcp_string_response(
            self.role_name(),
            self.session().task_id(),
            "review_reject",
            &response,
        );
        response
    }
}

// -- Tester MCP service --

#[allow(async_fn_in_trait)]
pub trait TesterMcpImpl: CommonMcpImpl {
    async fn get_param_destination_branch_impl(&self) -> String {
        self.get_destination_branch_impl().await
    }

    async fn get_param_work_branch_impl(&self) -> String {
        self.get_work_branch_impl().await
    }

    async fn test_accept_impl(&self, message: &str) -> String {
        tracing::info!(
            "[{}#{}] test_accept",
            self.role_name(),
            self.session().task_id()
        );
        let hostname = get_hostname();
        let response = if let Err(e) = self
            .session()
            .post_comment(
                CommentType::Report,
                message,
                Some(self.role()),
                &hostname,
                Some(self.mcp_tool()),
                Some(self.mcp_model()),
            )
            .await
        {
            format!("Error posting test acceptance: {e}")
        } else {
            // No signal set — finalize_session will call finish when signal is None.
            "Testing accepted — task will be marked done".to_string()
        };
        log_mcp_string_response(
            self.role_name(),
            self.session().task_id(),
            "test_accept",
            &response,
        );
        response
    }

    async fn test_reject_impl(&self, message: &str) -> String {
        tracing::info!(
            "[{}#{}] test_reject",
            self.role_name(),
            self.session().task_id()
        );
        let hostname = get_hostname();
        if let Err(e) = self
            .session()
            .post_comment(
                CommentType::Reject,
                message,
                Some(self.role()),
                &hostname,
                Some(self.mcp_tool()),
                Some(self.mcp_model()),
            )
            .await
        {
            let response = format!("Error posting test rejection: {e}");
            log_mcp_string_response(
                self.role_name(),
                self.session().task_id(),
                "test_reject",
                &response,
            );
            return response;
        }
        if let Err(e) = self.session().set_signal(crate::Signal::GoPlan).await {
            tracing::warn!(
                "Failed to set GoPlan signal for task {}: {e}",
                self.session().task_id()
            );
        }
        let response = "Testing rejected — task routed back to planner".to_string();
        log_mcp_string_response(
            self.role_name(),
            self.session().task_id(),
            "test_reject",
            &response,
        );
        response
    }
}

// -- Merger MCP service --

#[allow(async_fn_in_trait)]
pub trait MergerMcpImpl: CommonMcpImpl {
    async fn get_param_destination_branch_impl(&self) -> String {
        self.get_destination_branch_impl().await
    }

    async fn get_param_work_branch_impl(&self) -> String {
        self.get_work_branch_impl().await
    }
}
