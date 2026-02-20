use crate::task::{ChecklistItem, Parameter, Role, TaskSession};

use crate::mcp::common::get_hostname;

/// Common trait for MCP services (Planner, Worker) - shared implementations
#[allow(async_fn_in_trait)]
pub trait CommonMcpImpl: Send + Sync {
    fn session(&self) -> &TaskSession;
    fn role(&self) -> Role;

    fn role_name(&self) -> &'static str {
        self.role().as_str()
    }

    async fn get_description_impl(&self) -> String {
        tracing::info!(
            "[{}#{}] get_description",
            self.role_name(),
            self.session().task_id()
        );
        match self.session().get_description().await {
            Ok(desc) => desc,
            Err(e) => format!("Error: {e}"),
        }
    }

    async fn get_discussion_impl(&self) -> String {
        tracing::info!(
            "[{}#{}] get_discussion",
            self.role_name(),
            self.session().task_id()
        );
        match self.session().get_discussion().await {
            Ok(msgs) => {
                if msgs.is_empty() {
                    "No messages yet.".to_string()
                } else {
                    msgs.join("\n\n---\n\n")
                }
            }
            Err(e) => format!("Error: {e}"),
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
            .post_message(message, "error", &hostname)
            .await
        {
            tracing::error!(
                "Failed to post error message for task {}: {e}",
                self.session().task_id()
            );
            return format!("Error posting error message: {e}");
        }

        // Signal to pause task processing and wait for user response
        if let Err(e) = self.session().set_signal(crate::Signal::GoAsk).await {
            tracing::error!(
                "Failed to set signal GoAsk for task {} after reporting error: {e}",
                self.session().task_id()
            );
            return format!("Error reporting error but error pausing task: {e}");
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
            .post_message(message, self.role().as_str(), &hostname)
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

    async fn get_plan_impl(&self) -> String {
        tracing::info!(
            "[{}#{}] get_plan",
            self.role_name(),
            self.session().task_id()
        );
        match self.session().get_plan().await {
            Ok(plan) => plan,
            Err(e) => format!("Error: {e}"),
        }
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
        let plan_text = plan.to_string();
        match self
            .session()
            .modify_task(move |task| {
                task.plan = plan_text;
            })
            .await
        {
            Ok(()) => {
                // Mark plan as ready for worker to implement
                if let Err(e) = self.session().set_signal(crate::Signal::GoWork).await {
                    tracing::error!(
                        "Failed to set signal GoWork for task {} after posting plan: {e}",
                        self.session().task_id()
                    );
                    return format!("Plan posted but error marking task ready for work: {e}");
                }
                "Plan posted and task ready for worker implementation".to_string()
            }
            Err(e) => format!("Error updating task: {e}"),
        }
    }

    async fn pull_work_impl(&self) -> String {
        tracing::info!("[planner#{}] pull_work", self.session().task_id());
        match self.session().pull_work().await {
            Ok(path) => path,
            Err(e) => format!("Error: {e}"),
        }
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

    async fn ask_user_impl(&self, message: &str) -> String {
        tracing::info!("[worker#{}] ask_user", self.session().task_id());
        let hostname = get_hostname();

        if let Err(e) = self
            .session()
            .post_message(message, self.role().as_str(), &hostname)
            .await
        {
            tracing::error!(
                "Failed to post worker message for task {}: {e}",
                self.session().task_id()
            );
            return format!("Error posting message: {e}");
        }

        // Signal to pause task processing and wait for user response
        if let Err(e) = self.session().set_signal(crate::Signal::GoAsk).await {
            tracing::error!(
                "Failed to set signal GoAsk for task {} after ask_user: {e}",
                self.session().task_id()
            );
            return format!("Question posted but error pausing task: {e}");
        }
        "Message posted to user - task paused pending response".to_string()
    }

    async fn ask_planner_impl(&self, message: &str) -> String {
        tracing::info!("[worker#{}] ask_planner", self.session().task_id());
        let hostname = get_hostname();

        if let Err(e) = self
            .session()
            .post_message(message, self.role().as_str(), &hostname)
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

    async fn pull_work_impl(&self) -> String {
        tracing::info!("[worker#{}] pull_work", self.session().task_id());
        match self.session().pull_work().await {
            Ok(path) => path,
            Err(e) => format!("Error: {e}"),
        }
    }

    async fn push_work_impl(&self) -> String {
        tracing::info!("[worker#{}] push_work", self.session().task_id());
        match self.session().push_work().await {
            Ok(()) => "Work branch pushed successfully".to_string(),
            Err(e) => format!("Error: {e}"),
        }
    }

    async fn mark_done_impl(&self) -> String {
        tracing::info!("[worker#{}] mark_done", self.session().task_id());
        match self.session().mark_done().await {
            Ok(()) => "Task marked as done".to_string(),
            Err(e) => format!("Error: {e}"),
        }
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

    async fn pull_work_impl(&self) -> String {
        tracing::info!("[reviewer#{}] pull_work", self.session().task_id());
        match self.session().pull_work().await {
            Ok(path) => path,
            Err(e) => format!("Error: {e}"),
        }
    }
}

// -- Merger MCP service --

pub trait MergerMcpImpl: CommonMcpImpl {
    async fn get_param_destination_branch_impl(&self) -> String {
        self.get_param_impl(Parameter::DestinationBranch).await
    }

    async fn get_param_work_branch_impl(&self) -> String {
        self.get_param_impl(Parameter::WorkBranch).await
    }

    async fn pull_work_impl(&self) -> String {
        tracing::info!("[merger#{}] pull_work", self.session().task_id());
        match self.session().pull_work().await {
            Ok(path) => path,
            Err(e) => format!("Error: {e}"),
        }
    }

    async fn push_work_impl(&self) -> String {
        tracing::info!("[merger#{}] push_work", self.session().task_id());
        match self.session().push_work().await {
            Ok(()) => "Merged conflicts pushed successfully".to_string(),
            Err(e) => format!("Error: {e}"),
        }
    }

    async fn ask_user_impl(&self, message: &str) -> String {
        tracing::info!("[merger#{}] ask_user", self.session().task_id());
        let hostname = get_hostname();

        if let Err(e) = self
            .session()
            .post_message(message, self.role().as_str(), &hostname)
            .await
        {
            tracing::error!(
                "Failed to post merger message for task {}: {e}",
                self.session().task_id()
            );
            return format!("Error posting message: {e}");
        }

        // Signal to pause task processing and wait for user response
        if let Err(e) = self.session().set_signal(crate::Signal::GoAsk).await {
            tracing::error!(
                "Failed to set signal GoAsk for task {} after asking user: {e}",
                self.session().task_id()
            );
            return format!("Error pausing task: {e}");
        }

        "Message posted to user - task paused pending response".to_string()
    }
}
