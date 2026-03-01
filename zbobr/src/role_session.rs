use std::{path::PathBuf, sync::mpsc};

use anyhow::Context;
use tokio::process::Command;
use zbobr_dispatcher::{
    Signal, Stage, ToolExecutor, ZbobrDispatcherDyn, ZbobrExecutorConfig,
    task::{Model, Parameter, Role, Tool},
};
use zbobr_executor_claude::ClaudeExecutor;
use zbobr_executor_copilot::CopilotExecutor;
use zbobr_executor_mcp_tester::McpTesterExecutor;

use crate::prompts::Prompts;

/// A lightweight object capturing all of the parameters needed to execute
/// or inspect a role session.  By consolidating the data in one struct we can
/// centralize prompt construction and make callers responsible for printing the
/// prompt when `show_prompt` is requested.
// `RoleSession` is primarily a data carrier and isn't routinely
// printed for debugging.  The original `#[derive(Debug)]` triggered errors
// because several referenced config types do not implement `Debug`.  We
// provide a manual implementation that only includes the simple fields.
pub(crate) struct RoleSession<'a> {
    zbobr: &'a ZbobrDispatcherDyn,
    task_id: u64,
    role: Role,
    model: Option<Model>,
    prompts: &'a Prompts,
    executor_config: &'a ZbobrExecutorConfig,
}

impl<'a> std::fmt::Debug for RoleSession<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RoleSession")
            .field("task_id", &self.task_id)
            .field("role", &self.role)
            .field("model", &self.model)
            // skip the reference fields that don't implement Debug
            .finish()
    }
}

impl<'a> RoleSession<'a> {
    pub(crate) fn new(
        zbobr: &'a ZbobrDispatcherDyn,
        task_id: u64,
        role: Role,
        model: Option<Model>,
        prompts: &'a Prompts,
        executor_config: &'a ZbobrExecutorConfig,
    ) -> Self {
        Self {
            zbobr,
            task_id,
            role,
            model,
            prompts,
            executor_config,
        }
    }

    /// Construct the full prompt text that will be provided to the AI tool.
    ///
    /// This implementation now simply forwards to the shared logic owned by
    /// [`Prompts`].
    pub(crate) fn prompt(&self) -> anyhow::Result<String> {
        self.prompts.build_prompt(self.role)
    }

    /// Execute the session (assumes prompt already printed if requested).
    pub(crate) async fn run(&self) -> anyhow::Result<()> {
        // Adapted from previous run_role_session body, but without prompt
        // generation/dumping or `show_prompt` handling.
        let cli_tool = self.zbobr.config().cli_tool;
        // `self.model` is an `Option<Model>`; clone it so we don't move out of `&self`
        let model = resolve_model(cli_tool, self.model.clone(), self.executor_config);

        // update task stage based on role; we implement `From<Role> for Stage`
        self.zbobr
            .set_task_stage(self.task_id, self.role.into())
            .await?;

        let task_dir = self
            .zbobr
            .config()
            .workspaces
            .join(format!("task#{}", self.task_id));
        tokio::fs::create_dir_all(&task_dir).await?;

        let work_dir = prepare_workspace(self.zbobr, self.task_id, self.role, &task_dir).await?;

        if !matches!(self.role, Role::Preparator) {
            ensure_pr_url(self.zbobr, self.task_id).await?;
        }

        if should_try_early_merge(self.role) {
            if try_early_merge(self.zbobr, self.task_id, &work_dir).await? {
                return Ok(());
            }
        }

        let (assigned_port, server_handle) =
            start_mcp_server(self.zbobr.clone(), self.role, self.task_id).await?;

        // Build the URL using explicit parameters; `role` and `task_id` are
        // fields on `self` and therefore not directly available to `format!`
        // named args.
        let mcp_url = format!(
            "http://127.0.0.1:{assigned_port}/{role}/{task_id}",
            assigned_port = assigned_port,
            role = self.role,
            task_id = self.task_id
        );

        let prompt_text = self.prompt()?;
        let (execution_interrupted, execution_error) = execute_tool(
            cli_tool,
            self.executor_config,
            self.task_id,
            self.role,
            &model,
            assigned_port,
            &prompt_text,
            &work_dir,
            &mcp_url,
            &self.zbobr,
        )
        .await;

        // Pass a reference to avoid attempting to clone `anyhow::Error`.
        finalize_session(
            self.zbobr,
            self.task_id,
            self.role,
            &work_dir,
            execution_interrupted,
            execution_error.as_ref(),
        )
        .await?;

        server_handle.abort();
        if let Some(e) = execution_error {
            return Err(e);
        }

        Ok(())
    }
}

// ---------- helpers --------------------------------------------------------

fn resolve_model(
    cli_tool: Tool,
    override_model: Option<Model>,
    executor_config: &ZbobrExecutorConfig,
) -> Model {
    override_model.unwrap_or_else(|| match cli_tool {
        Tool::Claude => executor_config.claude.default_model.clone(),
        Tool::Copilot => executor_config.copilot.default_model.clone(),
        Tool::McpTester => Model::default(),
    })
}

/// Prepare the workspace directory for the given role.  Returns the directory
/// that the agent should operate in.
async fn prepare_workspace(
    zbobr: &ZbobrDispatcherDyn,
    task_id: u64,
    role: Role,
    task_dir: &PathBuf,
) -> anyhow::Result<PathBuf> {
    match role {
        Role::Preparator => Ok(task_dir.clone()),
        Role::Merger => {
            // Merger works on an existing directory, we don't want to re-clone
            let task = zbobr.get_task(task_id).await?;
            let dest_repo = task
                .parameters
                .get(&Parameter::DestinationRepository)
                .ok_or_else(|| {
                    anyhow::anyhow!("Task #{task_id} has no destination_repository parameter")
                })?
                .as_str();
            let repo_name = dest_repo.rsplit('/').next().unwrap_or(dest_repo);
            Ok(task_dir.join(repo_name))
        }
        _ => {
            // Planner / Worker / Reviewer: clone and check out work branch.
            let task = zbobr.get_task(task_id).await?;
            let dest_repo = task
                .parameters
                .get(&Parameter::DestinationRepository)
                .ok_or_else(|| {
                    anyhow::anyhow!("Task #{task_id} has no destination_repository parameter")
                })?
                .clone();
            let work_branch = task
                .parameters
                .get(&Parameter::WorkBranch)
                .ok_or_else(|| anyhow::anyhow!("Task #{task_id} has no work_branch parameter"))?
                .clone();
            let dest_branch_for_setup = task
                .parameters
                .get(&Parameter::DestinationBranch)
                .cloned()
                .unwrap_or_else(|| "main".to_string());
            match zbobr
                .clone_and_setup(&dest_repo, &work_branch, &dest_branch_for_setup, task_id)
                .await
            {
                Ok(path) => Ok(path),
                Err(e) => {
                    let msg = format!("Failed to prepare workspace for task #{task_id}: {e:#}");
                    tracing::error!("{msg}");
                    let hostname = zbobr_dispatcher::mcp::common::get_hostname();
                    if let Err(post_err) = zbobr
                        .task_session(task_id)
                        .post_message(&msg, "error", &hostname)
                        .await
                    {
                        tracing::warn!("Failed to post error to task discussion: {post_err}");
                    }
                    Err(anyhow::anyhow!(msg))
                }
            }
        }
    }
}

async fn ensure_pr_url(zbobr: &ZbobrDispatcherDyn, task_id: u64) -> anyhow::Result<()> {
    let role_session = zbobr.role_session(task_id);
    match role_session.ensure_pr_url().await {
        Ok(_pr_url) => Ok(()),
        Err(e) => {
            let msg = format!("Could not ensure PR URL for task #{task_id}: {e}");
            tracing::error!("{msg}");
            let hostname = zbobr_dispatcher::mcp::common::get_hostname();
            let task_session = zbobr.task_session(task_id);
            if let Err(post_err) = task_session.post_message(&msg, "error", &hostname).await {
                tracing::warn!("Failed to post error to task discussion: {post_err}");
            }
            Err(anyhow::anyhow!(msg))
        }
    }
}

fn should_try_early_merge(role: Role) -> bool {
    matches!(role, Role::Planner | Role::Worker | Role::Reviewer)
}

/// Attempt to merge the destination branch into the work directory in order to
/// catch conflicts early.  If a conflict is detected the task is moved back to
/// PENDING, the conflict flag is set, and `Ok(true)` is returned.
async fn try_early_merge(
    zbobr: &ZbobrDispatcherDyn,
    task_id: u64,
    work_dir: &PathBuf,
) -> anyhow::Result<bool> {
    let task = zbobr.get_task(task_id).await?;
    let dest_branch = task
        .parameters
        .get(&Parameter::DestinationBranch)
        .cloned()
        .unwrap_or_else(|| "main".to_string());

    let merge = Command::new("git")
        .args(["merge", &dest_branch, "--no-edit"])
        .current_dir(&work_dir)
        .output()
        .await
        .context("Failed to run git merge for conflict detection")?;

    if !merge.status.success() {
        tracing::warn!(
            "Merge conflict detected for task #{task_id} \
                 (merging '{dest_branch}' into work branch). \
                 Setting conflict flag and deferring to Merger."
        );
        zbobr
            .task_session(task_id)
            .set_conflict(true)
            .await
            .context("Failed to set conflict flag")?;
        zbobr
            .set_task_stage(task_id, Stage::Pending)
            .await
            .context("Failed to reset stage to Pending after conflict")?;
        return Ok(true);
    }

    Ok(false)
}

async fn start_mcp_server(
    zbobr: ZbobrDispatcherDyn,
    role: Role,
    task_id: u64,
) -> anyhow::Result<(u16, tokio::task::JoinHandle<()>)> {
    let (port_tx, port_rx) = mpsc::channel();
    let server_handle = tokio::spawn(async move {
        match zbobr_dispatcher::mcp::run_role_mcp_server(zbobr, role, task_id).await {
            Ok(assigned_port) => {
                let _ = port_tx.send(assigned_port);
                tracing::info!("MCP server assigned port {assigned_port}");
            }
            Err(e) => {
                tracing::error!("MCP server error: {e}");
            }
        }
    });

    let assigned_port = port_rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .context("MCP server failed to report assigned port in time")?;

    Ok((assigned_port, server_handle))
}

/// Run the selected tool executor and watch for Ctrl+C.
async fn execute_tool(
    cli_tool: Tool,
    executor_config: &ZbobrExecutorConfig,
    task_id: u64,
    role: Role,
    model: &Model,
    assigned_port: u16,
    prompt: &str,
    work_dir: &PathBuf,
    mcp_url: &str,
    zbobr: &ZbobrDispatcherDyn,
) -> (bool, Option<anyhow::Error>) {
    let executor: Box<dyn ToolExecutor> = match cli_tool {
        Tool::Copilot => Box::new(CopilotExecutor {
            config: executor_config.copilot.clone(),
        }),
        Tool::Claude => Box::new(ClaudeExecutor {
            config: executor_config.claude.clone(),
        }),
        Tool::McpTester => Box::new(McpTesterExecutor {
            config: executor_config.mcp_tester.clone(),
        }),
    };
    let agent_token = &zbobr.config().agent_github_token;
    let copilot_token = match cli_tool {
        Tool::Copilot => &executor_config.copilot.copilot_github_token,
        _ => "",
    };

    tokio::select! {
        result = executor.execute(task_id, role, model, assigned_port, prompt, work_dir, mcp_url, agent_token, copilot_token) => {
            match result {
                Ok(()) => (false, None),
                Err(e) => {
                    tracing::error!("Tool execution failed: {e}");
                    (false, Some(e))
                }
            }
        }
        _ = tokio::signal::ctrl_c() => {
            tracing::warn!("Received shutdown signal during execution");
            (true, None)
        }
    }
}

async fn finalize_session(
    zbobr: &ZbobrDispatcherDyn,
    task_id: u64,
    role: Role,
    work_dir: &PathBuf,
    execution_interrupted: bool,
    execution_error: Option<&anyhow::Error>,
) -> anyhow::Result<()> {
    let task_session = zbobr.task_session(task_id);

    if execution_interrupted {
        task_session.set_stage(Stage::Pending).await?;
        tracing::info!("Session interrupted for task #{task_id}, moved to PENDING");
        return Ok(());
    }

    if let Some(e) = execution_error {
        let error_msg = format!("Execution failed: {e}");
        let hostname = zbobr_dispatcher::mcp::common::get_hostname();
        if let Err(post_err) = task_session
            .post_message(&error_msg, "error", &hostname)
            .await
        {
            tracing::error!("Failed to post error to task #{task_id}: {post_err}");
        }
        if let Err(pause_err) = task_session
            .modify_task(|task| {
                task.pause = true;
            })
            .await
        {
            tracing::error!("Failed to set pause for task #{task_id}: {pause_err}");
        }
        task_session.set_stage(Stage::Pending).await?;
        tracing::info!("Session failed for task #{task_id}, moved to PENDING with pause");
        return Ok(());
    }

    tracing::info!("Session complete for task #{task_id}");

    if role == Role::Worker || role == Role::Merger {
        perform_auto_commit_and_push(zbobr, task_id, work_dir, role).await?;
    }

    // Decide final state based on role and checklist
    let current_task = zbobr.get_task(task_id).await?;
    let has_unchecked = current_task.checklist.iter().any(|i| !i.checked);
    match role {
        Role::Preparator => {
            if current_task.signal.is_none() && !current_task.pause {
                task_session.set_signal(Some(Signal::GoPlan)).await?;
            }
            task_session.set_stage(Stage::Pending).await?;
        }
        Role::Planner => {
            task_session.set_stage(Stage::Pending).await?;
        }
        Role::Worker => {
            if current_task.signal.is_none() && !current_task.pause {
                if has_unchecked {
                    task_session.set_signal(Some(Signal::GoWork)).await?;
                } else {
                    task_session.set_signal(Some(Signal::GoReview)).await?;
                }
            }
            task_session.set_stage(Stage::Pending).await?;
        }
        Role::Reviewer => {
            if current_task.signal.is_none() && !current_task.pause {
                if has_unchecked {
                    task_session.set_signal(Some(Signal::GoWork)).await?;
                    task_session.set_stage(Stage::Pending).await?;
                } else {
                    task_session.mark_done().await?;
                }
            } else {
                task_session.set_stage(Stage::Pending).await?;
            }
        }
        Role::Merger => {
            task_session.set_conflict(false).await?;
            task_session.set_stage(Stage::Pending).await?;
        }
    }

    Ok(())
}

async fn perform_auto_commit_and_push(
    zbobr: &ZbobrDispatcherDyn,
    task_id: u64,
    work_dir: &PathBuf,
    role: Role,
) -> anyhow::Result<()> {
    tracing::info!("Checking for uncommitted changes in {}", work_dir.display());

    match Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(&work_dir)
        .output()
        .await
    {
        Ok(status_output) if status_output.status.success() => {
            let uncommitted = String::from_utf8_lossy(&status_output.stdout)
                .trim()
                .to_string();
            if !uncommitted.is_empty() {
                tracing::info!("Found uncommitted changes, auto-committing...");
                let _ = Command::new("git")
                    .args(["add", "."])
                    .current_dir(&work_dir)
                    .status()
                    .await;

                let commit_msg = format!("Auto-commit by {} agent", role.as_str());
                match Command::new("git")
                    .args(["commit", "-m", &commit_msg])
                    .current_dir(&work_dir)
                    .status()
                    .await
                {
                    Ok(commit_status) if commit_status.success() => {
                        tracing::info!("Auto-commit successful");
                    }
                    _ => tracing::warn!("Auto-commit failed"),
                }
            } else {
                tracing::info!("No uncommitted changes found");
            }
        }
        _ => tracing::warn!("Failed to check git status for auto-commit"),
    }

    let role_session = zbobr.task_session(task_id).role_session();
    if let Err(e) = role_session.push_branch_commits().await {
        tracing::warn!("Could not push branch commits for task #{task_id}: {e}");
    }

    if zbobr.config().overwrite_author {
        rewrite_commit_authors(zbobr, task_id, work_dir, role).await?;
    }

    Ok(())
}

async fn rewrite_commit_authors(
    zbobr: &ZbobrDispatcherDyn,
    task_id: u64,
    work_dir: &PathBuf,
    _role: Role,
) -> anyhow::Result<()> {
    let task = zbobr.get_task(task_id).await?;
    let dest_branch = task
        .parameters
        .get(&Parameter::DestinationBranch)
        .cloned()
        .unwrap_or_else(|| "main".to_string());

    let git_user_name = &zbobr.config().git_user_name;
    let git_user_email = &zbobr.config().git_user_email;

    let config_user = Command::new("git")
        .args(["config", "--local", "user.name", git_user_name])
        .current_dir(&work_dir)
        .output()
        .await;

    let config_email = Command::new("git")
        .args(["config", "--local", "user.email", git_user_email])
        .current_dir(&work_dir)
        .output()
        .await;

    if let (Ok(user_out), Ok(email_out)) = (config_user, config_email) {
        if user_out.status.success() && email_out.status.success() {
            let rebase_cmd = format!(
                "git rebase --exec 'git commit --amend --no-edit --reset-author' '{}'",
                dest_branch
            );
            let rebase_output = Command::new("sh")
                .arg("-c")
                .arg(&rebase_cmd)
                .env("GIT_AUTHOR_NAME", git_user_name)
                .env("GIT_AUTHOR_EMAIL", git_user_email)
                .env("GIT_COMMITTER_NAME", git_user_name)
                .env("GIT_COMMITTER_EMAIL", git_user_email)
                .current_dir(&work_dir)
                .output()
                .await;

            match rebase_output {
                Ok(output) if output.status.success() => {
                    tracing::info!("Successfully rewrote commit authors");
                    if let Err(e) = zbobr
                        .task_session(task_id)
                        .role_session()
                        .push_branch_commits()
                        .await
                    {
                        tracing::warn!("Could not push rewritten commits for task #{task_id}: {e}");
                    }
                }
                Ok(output) => {
                    tracing::warn!(
                        "Failed to rewrite commit authors: {}",
                        String::from_utf8_lossy(&output.stderr)
                    );
                }
                Err(e) => {
                    tracing::warn!("Error running git rebase for author rewriting: {e}");
                }
            }
        } else {
            tracing::warn!("Failed to set up git config for author rewriting");
        }
    } else {
        tracing::warn!("Error executing git config commands for author rewriting");
    }

    Ok(())
}
