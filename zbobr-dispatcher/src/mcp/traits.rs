use crate::{
    CommentType, Signal,
    mcp::common::get_hostname,
    task::{ChecklistItem, Parameter, Role, RoleSession, Tool, Model},
};

// Helper functions for logging MCP responses

/// Log a comments response (typically from get_plan). Parses JSON and logs each comment.
fn log_mcp_comments_response(role_name: &str, task_id: u64, response: &str) {
    // Log exact JSON in debug level
    tracing::debug!(
        "[{}#{}] response: {}",
        role_name,
        task_id,
        response
    );

    // Log info level with parsed comments
    if response.starts_with('[') {
        if let Ok(comments) = serde_json::from_str::<Vec<zbobr_api::Comment>>(response) {
            tracing::info!(
                "[{}#{}] get_plan returned {} comment(s)",
                role_name,
                task_id,
                comments.len()
            );
            for comment in comments {
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
            tracing::info!("[{}#{}] get_plan response (failed to parse): {}", role_name, task_id, response);
        }
    } else if response.starts_with("Error") {
        tracing::info!("[{}#{}] get_plan error: {}", role_name, task_id, response);
    } else {
        tracing::info!("[{}#{}] get_plan response: {}", role_name, task_id, response);
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
            tracing::info!("[{}#{}] {} response (failed to parse): {}", role_name, task_id, tool_name, response);
        }
    } else if response.starts_with('{') {
        if let Ok(_) = serde_json::from_str::<serde_json::Value>(response) {
            tracing::info!("[{}#{}] {} succeeded", role_name, task_id, tool_name);
        } else {
            tracing::info!("[{}#{}] {} response (failed to parse): {}", role_name, task_id, tool_name, response);
        }
    } else if response.starts_with("Error") {
        tracing::info!("[{}#{}] {} error: {}", role_name, task_id, tool_name, response);
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
        tracing::info!("[{}#{}] {} error: {}", role_name, task_id, tool_name, response);
    } else {
        let display_str = if response.len() > 100 {
            format!("{}...", &response[..100])
        } else {
            response.to_string()
        };
        tracing::info!("[{}#{}] {} result: {}", role_name, task_id, tool_name, display_str);
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

    async fn get_plan_impl(&self, offset: i32) -> String {
        tracing::info!(
            "[{}#{}] get_plan offset={}",
            self.role_name(),
            self.session().task_id(),
            offset
        );

        let comments = match self.session().get_comments().await {
            Ok(c) => c,
            Err(e) => {
                let response = format!("Error: {e}");
                log_mcp_string_response(self.role_name(), self.session().task_id(), "get_plan", &response);
                return response;
            },
        };

        // Find indices of all cut-boundary comments (Reject or Done).
        // Each cut marker starts a new context chunk.
        let cut_indices: Vec<usize> = comments
            .iter()
            .enumerate()
            .filter(|(_, c)| c.comment_type.is_cut())
            .map(|(i, _)| i)
            .collect();

        if cut_indices.is_empty() {
            // No chunks yet — check if there is any plan comment; if not, return task description.
            let has_plan = comments.iter().any(|c| c.comment_type == CommentType::Plan);
            if !has_plan {
                let desc = match self.session().get_description().await {
                    Ok(d) if !d.is_empty() => d,
                    Ok(_) => "No task description provided.".to_string(),
                    Err(e) => {
                        let response = format!("Error fetching description: {e}");
                        log_mcp_string_response(self.role_name(), self.session().task_id(), "get_plan", &response);
                        return response;
                    },
                };
                let synthetic = vec![zbobr_api::Comment {
                    comment_type: CommentType::Request,
                    timestamp: String::new(),
                    role: None,
                    hostname: String::new(),
                    tool: None,
                    model: None,
                    text: desc,
                }];
                let response = match serde_json::to_string_pretty(&synthetic) {
                    Ok(json) => json,
                    Err(e) => format!("Error serializing: {e}"),
                };
                log_mcp_comments_response(self.role_name(), self.session().task_id(), &response);
                return response;
            }
            // There is a plan but no cuts yet — the whole comment list is one chunk.
            if offset < -1 {
                let response = format!("offset {} out of range: only 1 chunk available", offset);
                log_mcp_string_response(self.role_name(), self.session().task_id(), "get_plan", &response);
                return response;
            }
            let result_comments: Vec<zbobr_api::Comment> = comments
                .iter()
                .filter(|c| {
                    c.comment_type != CommentType::Error && c.comment_type != CommentType::Done
                })
                .cloned()
                .collect();
            if result_comments.is_empty() {
                let response = "Error: No messages found in chunk (task may already be complete, or all comments have been filtered)".to_string();
                log_mcp_string_response(self.role_name(), self.session().task_id(), "get_plan", &response);
                return response;
            }
            let response = match serde_json::to_string_pretty(&result_comments) {
                Ok(json) => json,
                Err(e) => format!("Error serializing: {e}"),
            };
            log_mcp_comments_response(self.role_name(), self.session().task_id(), &response);
            return response;
        }

        // Chunks: chunk[0] = comments[0..cut[0]], chunk[i] = comments[cut[i-1]..cut[i]], ...
        // Last chunk = comments[cut.last()..end].
        // Number of chunks = cut_indices.len() + 1 (but chunk 0 may be empty if first comment is cut).
        // We expose chunks as: index 0 = last chunk, -1 = second-to-last, etc.
        let num_chunks = cut_indices.len() + 1;
        let target_chunk = if offset >= 0 {
            num_chunks - 1
        } else {
            let back = (-offset) as usize;
            if back >= num_chunks {
                let response = format!(
                    "offset {} out of range: only {} chunk(s) available",
                    offset, num_chunks
                );
                log_mcp_string_response(self.role_name(), self.session().task_id(), "get_plan", &response);
                return response;
            }
            num_chunks - 1 - back
        };

        let (start_idx, end_idx) = if target_chunk == 0 {
            (0, cut_indices[0])
        } else if target_chunk == num_chunks - 1 {
            (cut_indices[target_chunk - 1], comments.len())
        } else {
            (cut_indices[target_chunk - 1], cut_indices[target_chunk])
        };

        // Return comments in the chunk, excluding Error and Done (but keeping Reject).
        let result_comments: Vec<zbobr_api::Comment> = comments[start_idx..end_idx]
            .iter()
            .filter(|c| c.comment_type != CommentType::Error && c.comment_type != CommentType::Done)
            .cloned()
            .collect();

        if result_comments.is_empty() {
            let response = "Error: No messages found in chunk (task may already be complete, or all comments have been filtered)".to_string();
            log_mcp_string_response(self.role_name(), self.session().task_id(), "get_plan", &response);
            return response;
        }

        let response = match serde_json::to_string_pretty(&result_comments) {
            Ok(json) => json,
            Err(e) => format!("Error serializing: {e}"),
        };
        log_mcp_comments_response(self.role_name(), self.session().task_id(), &response);
        response
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
            log_mcp_string_response(self.role_name(), self.session().task_id(), "report_error", &response);
            return response;
        }

        // Set pause flag to stop task processing and wait for user response.
        if let Err(e) = self.session().set_pause(true).await {
            tracing::error!(
                "Failed to set pause for task {} after reporting error: {e}",
                self.session().task_id()
            );
            let response = format!("Error reporting error but error pausing task: {e}");
            log_mcp_string_response(self.role_name(), self.session().task_id(), "report_error", &response);
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
        log_mcp_string_response(self.role_name(), self.session().task_id(), "report_error", &response);
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
                &hostname, Some(self.mcp_tool()), Some(self.mcp_model()),
            )
            .await
        {
            tracing::error!(
                "Failed to post results message for task {}: {e}",
                self.session().task_id()
            );
            let response = format!("Error posting results message: {e}");
            log_mcp_string_response(self.role_name(), self.session().task_id(), "report_results", &response);
            return response;
        }

        let response = "Results reported successfully".to_string();
        log_mcp_string_response(self.role_name(), self.session().task_id(), "report_results", &response);
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
                &hostname, Some(self.mcp_tool()), Some(self.mcp_model()),
            )
            .await
        {
            tracing::error!(
                "Failed to post ask_user message for task {}: {e}",
                self.session().task_id()
            );
            let response = format!("Error posting ask_user message: {e}");
            log_mcp_string_response(self.role_name(), self.session().task_id(), "ask_user", &response);
            return response;
        }

        // Set pause flag to stop task processing and wait for user response.
        if let Err(e) = self.session().set_pause(true).await {
            tracing::error!(
                "Failed to set pause for task {} after asking user: {e}",
                self.session().task_id()
            );
            let response = format!("Error asking user but error pausing task: {e}");
            log_mcp_string_response(self.role_name(), self.session().task_id(), "ask_user", &response);
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
        log_mcp_string_response(self.role_name(), self.session().task_id(), "ask_user", &response);
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
        log_mcp_json_response(self.role_name(), self.session().task_id(), "get_checklist", &response);
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
            .modify_task(move |task| {
                if let Some(item) = task.checklist.iter_mut().find(|item| item.id == item_id) {
                    item.checked = checked;
                }
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
        log_mcp_string_response(self.role_name(), self.session().task_id(), "check_checklist_item", &response);
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
                    log_mcp_string_response(self.role_name(), self.session().task_id(), "insert_checklist_item", &response);
                    return response;
                }
                if let Some(ref aid) = after
                    && !items.iter().any(|item| item.id == *aid)
                {
                    let response = format!("Error: Checklist item with id '{}' not found", aid);
                    log_mcp_string_response(self.role_name(), self.session().task_id(), "insert_checklist_item", &response);
                    return response;
                }
            }
            Err(e) => {
                let response = format!("Error: {e}");
                log_mcp_string_response(self.role_name(), self.session().task_id(), "insert_checklist_item", &response);
                return response;
            }
        }

        let response = match self
            .session()
            .modify_task(move |task| {
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
            })
            .await
        {
            Ok(()) => format!("Checklist item '{}' inserted", id),
            Err(e) => format!("Error updating task: {e}"),
        };
        log_mcp_string_response(self.role_name(), self.session().task_id(), "insert_checklist_item", &response);
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
            .modify_task(move |task| {
                if let Some(item) = task.checklist.iter_mut().find(|item| item.id == item_id) {
                    item.text = item_text;
                }
            })
            .await
        {
            Ok(()) => format!("Checklist item '{}' updated", id),
            Err(e) => format!("Error updating task: {e}"),
        };
        log_mcp_string_response(self.role_name(), self.session().task_id(), "update_checklist_item", &response);
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
                        log_mcp_string_response(self.role_name(), self.session().task_id(), "delete_checklist_item", &response);
                        return response;
                    }
                } else {
                    let response = format!("Error: Checklist item with id '{}' not found", id);
                    log_mcp_string_response(self.role_name(), self.session().task_id(), "delete_checklist_item", &response);
                    return response;
                }
            }
            Err(e) => {
                let response = format!("Error: {e}");
                log_mcp_string_response(self.role_name(), self.session().task_id(), "delete_checklist_item", &response);
                return response;
            }
        }

        let response = match self
            .session()
            .modify_task(move |task| {
                task.checklist.retain(|item| item.id != item_id);
            })
            .await
        {
            Ok(()) => format!("Checklist item '{}' deleted", id),
            Err(e) => format!("Error updating task: {e}"),
        };
        log_mcp_string_response(self.role_name(), self.session().task_id(), "delete_checklist_item", &response);
        response
    }

    async fn get_param_impl(&self, param: Parameter) -> String {
        tracing::info!(
            "[{}#{}] get_param_{}",
            self.role_name(),
            self.session().task_id(),
            param.name()
        );
        let response = match self.session().get_parameter(param).await {
            Ok(Some(value)) => value,
            Ok(None) => format!("{} is not set", param.name()),
            Err(e) => format!("Error: {e}"),
        };
        log_mcp_string_response(self.role_name(), self.session().task_id(), &format!("get_param_{}", param.name()), &response);
        response
    }

    async fn set_param_impl(&self, param: Parameter, value: Option<String>) -> String {
        tracing::info!(
            "[{}#{}] set_param_{} value={:?}",
            self.role_name(),
            self.session().task_id(),
            param.name(),
            value
        );
        let response = match self.session().set_parameter(param, value).await {
            Ok(()) => format!("{} updated", param.name()),
            Err(e) => format!("Error: {e}"),
        };
        log_mcp_string_response(self.role_name(), self.session().task_id(), &format!("set_param_{}", param.name()), &response);
        response
    }
}

/// Preparator-specific MCP implementations
#[allow(async_fn_in_trait)]
pub trait PreparatorMcpImpl: CommonMcpImpl {
    async fn get_param_destination_repository_impl(&self) -> String {
        self.get_param_impl(Parameter::DestinationRepository).await
    }

    async fn set_param_destination_repository_impl(&self, value: Option<String>) -> String {
        self.set_param_impl(Parameter::DestinationRepository, value)
            .await
    }

    async fn get_param_destination_branch_impl(&self) -> String {
        self.get_param_impl(Parameter::DestinationBranch).await
    }

    async fn set_param_destination_branch_impl(&self, value: Option<String>) -> String {
        self.set_param_impl(Parameter::DestinationBranch, value)
            .await
    }

    async fn get_param_work_branch_impl(&self) -> String {
        self.get_param_impl(Parameter::WorkBranch).await
    }

    async fn set_param_work_branch_postfix_impl(&self, value: Option<String>) -> String {
        match self.session().get_parameter(Parameter::WorkBranch).await {
            Ok(Some(_)) => return "Error: work_branch is already set".to_string(),
            Err(e) => return format!("Error: {e}"),
            Ok(None) => {}
        }
        let branch = value.map(|v| self.session().create_branch_name(&v));
        self.set_param_impl(Parameter::WorkBranch, branch).await
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
            .post_comment(CommentType::Plan, plan, Some(self.role()), &hostname, Some(self.mcp_tool()), None)
            .await
        {
            tracing::error!(
                "Failed to post plan comment for task {}: {e}",
                self.session().task_id()
            );
            let response = format!("Error posting plan: {e}");
            log_mcp_string_response(self.role_name(), self.session().task_id(), "post_plan", &response);
            return response;
        }

        let response = "Plan posted and task ready for worker implementation".to_string();
        log_mcp_string_response(self.role_name(), self.session().task_id(), "post_plan", &response);
        response
    }

    async fn get_param_destination_branch_impl(&self) -> String {
        self.get_param_impl(Parameter::DestinationBranch).await
    }

    async fn get_param_work_branch_impl(&self) -> String {
        self.get_param_impl(Parameter::WorkBranch).await
    }
}

/// Worker-specific MCP implementations
#[allow(async_fn_in_trait)]
pub trait WorkerMcpImpl: CommonMcpImpl {
    async fn get_param_destination_branch_impl(&self) -> String {
        self.get_param_impl(Parameter::DestinationBranch).await
    }

    async fn get_param_work_branch_impl(&self) -> String {
        self.get_param_impl(Parameter::WorkBranch).await
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
                &hostname, Some(self.mcp_tool()), Some(self.mcp_model()),
            )
            .await
        {
            tracing::error!(
                "Failed to post worker->planner message for task {}: {e}",
                self.session().task_id()
            );
            let response = format!("Error posting message: {e}");
            log_mcp_string_response(self.role_name(), self.session().task_id(), "ask_planner", &response);
            return response;
        }

        // Pass task back to planner agent for clarification or re-planning
        if let Err(e) = self.session().set_signal(crate::Signal::GoPlan).await {
            tracing::error!(
                "Failed to set signal GoPlan for task {} after ask_planner: {e}",
                self.session().task_id()
            );
            let response = format!("Message posted but error returning to planner: {e}");
            log_mcp_string_response(self.role_name(), self.session().task_id(), "ask_planner", &response);
            return response;
        }
        let response = "Message posted to planner - task returned for clarification".to_string();
        log_mcp_string_response(self.role_name(), self.session().task_id(), "ask_planner", &response);
        response
    }
}

/// Reviewer-specific MCP implementations
#[allow(async_fn_in_trait)]
pub trait ReviewerMcpImpl: CommonMcpImpl {
    async fn get_param_destination_branch_impl(&self) -> String {
        self.get_param_impl(Parameter::DestinationBranch).await
    }

    async fn get_param_work_branch_impl(&self) -> String {
        self.get_param_impl(Parameter::WorkBranch).await
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
                &hostname, Some(self.mcp_tool()), Some(self.mcp_model()),
            )
            .await
        {
            format!("Error posting review acceptance: {e}")
        } else {
            // No signal set — finalize_session will call finish when signal is None.
            "Review accepted — task will be marked done".to_string()
        };
        log_mcp_string_response(self.role_name(), self.session().task_id(), "review_accept", &response);
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
                &hostname, Some(self.mcp_tool()), Some(self.mcp_model()),
            )
            .await
        {
            let response = format!("Error posting review rejection: {e}");
            log_mcp_string_response(self.role_name(), self.session().task_id(), "review_reject", &response);
            return response;
        }
        if let Err(e) = self.session().set_signal(crate::Signal::GoPlan).await {
            tracing::warn!(
                "Failed to set GoPlan signal for task {}: {e}",
                self.session().task_id()
            );
        }
        let response = "Review rejected — task routed back to planner".to_string();
        log_mcp_string_response(self.role_name(), self.session().task_id(), "review_reject", &response);
        response
    }
}

// -- Tester MCP service --

#[allow(async_fn_in_trait)]
pub trait TesterMcpImpl: CommonMcpImpl {
    async fn get_param_destination_branch_impl(&self) -> String {
        self.get_param_impl(Parameter::DestinationBranch).await
    }

    async fn get_param_work_branch_impl(&self) -> String {
        self.get_param_impl(Parameter::WorkBranch).await
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
                &hostname, Some(self.mcp_tool()), Some(self.mcp_model()),
            )
            .await
        {
            format!("Error posting test acceptance: {e}")
        } else {
            // No signal set — finalize_session will call finish when signal is None.
            "Testing accepted — task will be marked done".to_string()
        };
        log_mcp_string_response(self.role_name(), self.session().task_id(), "test_accept", &response);
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
                &hostname, Some(self.mcp_tool()), Some(self.mcp_model()),
            )
            .await
        {
            let response = format!("Error posting test rejection: {e}");
            log_mcp_string_response(self.role_name(), self.session().task_id(), "test_reject", &response);
            return response;
        }
        if let Err(e) = self.session().set_signal(crate::Signal::GoPlan).await {
            tracing::warn!(
                "Failed to set GoPlan signal for task {}: {e}",
                self.session().task_id()
            );
        }
        let response = "Testing rejected — task routed back to planner".to_string();
        log_mcp_string_response(self.role_name(), self.session().task_id(), "test_reject", &response);
        response
    }
}

// -- Merger MCP service --

#[allow(async_fn_in_trait)]
pub trait MergerMcpImpl: CommonMcpImpl {
    async fn get_param_destination_branch_impl(&self) -> String {
        self.get_param_impl(Parameter::DestinationBranch).await
    }

    async fn get_param_work_branch_impl(&self) -> String {
        self.get_param_impl(Parameter::WorkBranch).await
    }
}
