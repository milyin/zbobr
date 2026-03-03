use crate::{
    Signal, CommentType,
    mcp::common::get_hostname,
    task::{ChecklistItem, Parameter, Role, RoleSession},
};

/// Common trait for MCP services (Planner, Worker) - shared implementations
#[allow(async_fn_in_trait)]
pub trait CommonMcpImpl: Send + Sync {
    fn session(&self) -> &RoleSession;
    fn role(&self) -> Role;

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
            Err(e) => return format!("Error: {e}"),
        };

        // Find indices of all PLAN comments
        let plan_indices: Vec<usize> = comments
            .iter()
            .enumerate()
            .filter(|(_, c)| c.comment_type == CommentType::Plan)
            .map(|(i, _)| i)
            .collect();

        if plan_indices.is_empty() {
            // No plan posted yet — return task description as a user Request comment
            let desc = match self.session().get_description().await {
                Ok(d) if !d.is_empty() => d,
                Ok(_) => "No task description provided.".to_string(),
                Err(e) => return format!("Error fetching description: {e}"),
            };
            let synthetic = vec![zbobr_api::Comment {
                comment_type: CommentType::Request,
                timestamp: String::new(),
                role: None,
                hostname: String::new(),
                model: None,
                text: desc,
            }];
            return match serde_json::to_string_pretty(&synthetic) {
                Ok(json) => json,
                Err(e) => format!("Error serializing: {e}"),
            };
        }

        // Select plan by offset (0 = last, -1 = second-to-last, etc.)
        let target_idx = if offset >= 0 {
            plan_indices.len() - 1
        } else {
            let back = (-offset) as usize;
            if back >= plan_indices.len() {
                return format!(
                    "offset {} out of range: only {} plan(s) available",
                    offset,
                    plan_indices.len()
                );
            }
            plan_indices.len() - 1 - back
        };

        let plan_comment_idx = plan_indices[target_idx];
        // End boundary: the next plan comment (exclusive) or end of comments
        let end_idx = plan_indices
            .get(target_idx + 1)
            .copied()
            .unwrap_or(comments.len());

        // Return the plan comment + all following comments until next plan as JSON
        let result_comments: Vec<zbobr_api::Comment> =
            comments[plan_comment_idx..end_idx].to_vec();
        match serde_json::to_string_pretty(&result_comments) {
            Ok(json) => json,
            Err(e) => format!("Error serializing: {e}"),
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
                None,
            )
            .await
        {
            tracing::error!(
                "Failed to post error message for task {}: {e}",
                self.session().task_id()
            );
            return format!("Error posting error message: {e}");
        }

        // Set pause flag to stop task processing and wait for user response.
        if let Err(e) = self.session().set_pause(true).await {
            tracing::error!(
                "Failed to set pause for task {} after reporting error: {e}",
                self.session().task_id()
            );
            return format!("Error reporting error but error pausing task: {e}");
        }

        // Set the retry signal so the task returns to this role after the user intervenes.
        if let Err(e) = self.session().set_signal(self.retry_signal()).await {
            tracing::warn!(
                "Failed to set retry signal for task {} after reporting error: {e}",
                self.session().task_id()
            );
        }

        "Error reported to user - task paused pending response".to_string()
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
                None,
            )
            .await
        {
            tracing::error!(
                "Failed to post results message for task {}: {e}",
                self.session().task_id()
            );
            return format!("Error posting results message: {e}");
        }

        "Results reported successfully".to_string()
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
                None,
            )
            .await
        {
            tracing::error!(
                "Failed to post ask_user message for task {}: {e}",
                self.session().task_id()
            );
            return format!("Error posting ask_user message: {e}");
        }

        // Set pause flag to stop task processing and wait for user response.
        if let Err(e) = self.session().set_pause(true).await {
            tracing::error!(
                "Failed to set pause for task {} after asking user: {e}",
                self.session().task_id()
            );
            return format!("Error asking user but error pausing task: {e}");
        }

        // Set the retry signal so the task returns to this role after the user responds.
        if let Err(e) = self.session().set_signal(self.retry_signal()).await {
            tracing::warn!(
                "Failed to set retry signal for task {} after asking user: {e}",
                self.session().task_id()
            );
        }

        "User asked for guidance".to_string()
    }

    async fn get_checklist_impl(&self) -> String {
        tracing::info!(
            "[{}#{}] get_checklist",
            self.role_name(),
            self.session().task_id()
        );
        match self.session().get_checklist().await {
            Ok(items) => match serde_json::to_string_pretty(&items) {
                Ok(json) => json,
                Err(e) => format!("Error serializing checklist: {e}"),
            },
            Err(e) => format!("Error: {e}"),
        }
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
        match self
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
        }
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
                    return format!("Error: Checklist item with id '{}' already exists", id);
                }
                if let Some(ref aid) = after
                    && !items.iter().any(|item| item.id == *aid)
                {
                    return format!("Error: Checklist item with id '{}' not found", aid);
                }
            }
            Err(e) => return format!("Error: {e}"),
        }

        match self
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
        }
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
        match self
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
        }
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
                        return format!(
                            "Error: Cannot delete checked checklist item '{}'. Checked items are preserved as work history.",
                            id
                        );
                    }
                } else {
                    return format!("Error: Checklist item with id '{}' not found", id);
                }
            }
            Err(e) => return format!("Error: {e}"),
        }

        match self
            .session()
            .modify_task(move |task| {
                task.checklist.retain(|item| item.id != item_id);
            })
            .await
        {
            Ok(()) => format!("Checklist item '{}' deleted", id),
            Err(e) => format!("Error updating task: {e}"),
        }
    }

    async fn get_param_impl(&self, param: Parameter) -> String {
        tracing::info!(
            "[{}#{}] get_param_{}",
            self.role_name(),
            self.session().task_id(),
            param.name()
        );
        match self.session().get_parameter(param).await {
            Ok(Some(value)) => value,
            Ok(None) => format!("{} is not set", param.name()),
            Err(e) => format!("Error: {e}"),
        }
    }

    async fn set_param_impl(&self, param: Parameter, value: Option<String>) -> String {
        tracing::info!(
            "[{}#{}] set_param_{} value={:?}",
            self.role_name(),
            self.session().task_id(),
            param.name(),
            value
        );
        match self.session().set_parameter(param, value).await {
            Ok(()) => format!("{} updated", param.name()),
            Err(e) => format!("Error: {e}"),
        }
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
            .post_comment(
                CommentType::Plan,
                plan,
                Some(self.role()),
                &hostname,
                None,
            )
            .await
        {
            tracing::error!(
                "Failed to post plan comment for task {}: {e}",
                self.session().task_id()
            );
            return format!("Error posting plan: {e}");
        }


        "Plan posted and task ready for worker implementation".to_string()
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
            .post_comment(CommentType::Request, message, Some(self.role()), &hostname, None)
            .await
        {
            tracing::error!(
                "Failed to post worker->planner message for task {}: {e}",
                self.session().task_id()
            );
            return format!("Error posting message: {e}");
        }

        // Pass task back to planner agent for clarification or re-planning
        if let Err(e) = self.session().set_signal(crate::Signal::GoPlan).await {
            tracing::error!(
                "Failed to set signal GoPlan for task {} after ask_planner: {e}",
                self.session().task_id()
            );
            return format!("Message posted but error returning to planner: {e}");
        }
        "Message posted to planner - task returned for clarification".to_string()
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
        if let Err(e) = self
            .session()
            .post_comment(
                CommentType::Report,
                message,
                Some(self.role()),
                &hostname,
                None,
            )
            .await
        {
            return format!("Error posting review acceptance: {e}");
        }
        // No signal set — finalize_session will call mark_done when signal is None.
        "Review accepted — task will be marked done".to_string()
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
                CommentType::Report,
                message,
                Some(self.role()),
                &hostname,
                None,
            )
            .await
        {
            return format!("Error posting review rejection: {e}");
        }
        if let Err(e) = self.session().set_signal(crate::Signal::GoPlan).await {
            tracing::warn!(
                "Failed to set GoPlan signal for task {}: {e}",
                self.session().task_id()
            );
        }
        "Review rejected — task routed back to planner".to_string()
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
