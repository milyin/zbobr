#![allow(clippy::needless_borrows_for_generic_args)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Context;
use clap::{Args, Parser};

// bring in the generic git helpers from utility crate
use zbobr_utility::{git, git_check, git_output};

use crate::{
    Comment, Task, TaskDir, ToolExecutor, ZbobrDispatcher,
    mcp::common::get_hostname,
    task::{Model, Tool},
};
use zbobr_api::config::{PipelineConfig, StageDefinition};
use zbobr_api::CommentTag;

// ---------------------------------------------------------------------------
// CLI types
// ---------------------------------------------------------------------------

/// Configuration file path argument.
#[derive(Args, Clone)]
pub struct ConfigFileArg {
    /// Path to TOML configuration file
    #[arg(long = "config")]
    pub path: Option<PathBuf>,
}

/// Resolved config file location.
pub struct ConfigLocation {
    pub config_path: PathBuf,
    pub config_dir: PathBuf,
}

/// Resolve the config file path and its parent directory.
///
/// When `cli_path` is `Some`, the file must exist (its parent is used as
/// `config_dir`).  When `None`, `default_config_name` in the current
/// directory is used and `config_dir` is `std::env::current_dir()`.
pub fn resolve_config_location(
    cli_path: &Option<PathBuf>,
    default_config_name: &str,
) -> anyhow::Result<ConfigLocation> {
    let config_path = cli_path
        .clone()
        .unwrap_or_else(|| default_config_name.into());

    let config_dir = if cli_path.is_some() {
        std::fs::canonicalize(&config_path)
            .with_context(|| format!("Cannot resolve config path: {}", config_path.display()))?
            .parent()
            .expect("config file must have a parent directory")
            .to_path_buf()
    } else {
        std::env::current_dir()?
    };

    Ok(ConfigLocation {
        config_path,
        config_dir,
    })
}

/// Global arguments that should be hoisted before subcommands.
/// This includes only dispatcher and executor config, not backend-specific settings.
#[derive(Args, Clone)]
pub struct GlobalArgs {
    #[command(
        flatten,
        next_help_heading = "[config] Meta options and config file overrides"
    )]
    pub config_file: ConfigFileArg,

    #[command(flatten, next_help_heading = "[dispatcher]")]
    pub dispatcher: crate::config::ZbobrDispatcherArgs,

    #[command(flatten, next_help_heading = "[executor]")]
    pub executor: crate::ZbobrExecutorArgs,
}

// ---------------------------------------------------------------------------
// CLI parsing
// ---------------------------------------------------------------------------

/// Parse CLI allowing global options both before and after the subcommand.
///
/// Global options are hoisted to appear before the subcommand so clap can
/// parse them regardless of where the user places them.
pub fn parse_cli<C: Parser + clap::CommandFactory>(
    app_name: &'static str,
    app_about: &'static str,
    app_long_about: &'static str,
) -> C {
    let cmd = C::command()
        .name(app_name)
        .about(app_about)
        .long_about(app_long_about);

    let subcommands: std::collections::HashSet<String> = cmd
        .get_subcommands()
        .map(|s| s.get_name().to_owned())
        .collect();

    let global_tmp = GlobalArgs::augment_args(clap::Command::new(""));
    let global_flags: std::collections::HashMap<String, bool> = global_tmp
        .get_arguments()
        .filter_map(|a| {
            a.get_long().map(|long| {
                let takes_value = !matches!(
                    a.get_action(),
                    clap::ArgAction::SetTrue | clap::ArgAction::SetFalse | clap::ArgAction::Count
                );
                (format!("--{long}"), takes_value)
            })
        })
        .collect();

    let raw_args: Vec<String> = std::env::args().collect();
    if raw_args.is_empty() {
        let m = cmd.get_matches_from(raw_args);
        return C::from_arg_matches(&m).unwrap_or_else(|e| e.exit());
    }

    let mut before_sub = vec![raw_args[0].clone()];
    let mut sub_and_after: Vec<String> = Vec::new();
    let mut found_sub = false;

    let mut i = 1;
    while i < raw_args.len() {
        let arg = &raw_args[i];

        if !found_sub {
            if subcommands.contains(arg.as_str()) {
                found_sub = true;
                sub_and_after.push(arg.clone());
            } else {
                before_sub.push(arg.clone());
            }
            i += 1;
            continue;
        }

        let base = arg.split('=').next().unwrap_or(arg);
        if let Some(&takes_value) = global_flags.get(base) {
            if arg.contains('=') {
                before_sub.push(arg.clone());
                i += 1;
            } else if takes_value && i + 1 < raw_args.len() {
                before_sub.push(arg.clone());
                before_sub.push(raw_args[i + 1].clone());
                i += 2;
            } else {
                before_sub.push(arg.clone());
                i += 1;
            }
        } else {
            sub_and_after.push(arg.clone());
            i += 1;
        }
    }

    before_sub.extend(sub_and_after);
    let m = cmd.get_matches_from(before_sub);
    C::from_arg_matches(&m).unwrap_or_else(|e| e.exit())
}

// ---------------------------------------------------------------------------
// Task display
// ---------------------------------------------------------------------------

/// Print a task to stdout in a human-readable format.
pub fn print_task(task: &Task, discussion: &[Comment]) {
    println!("ID:          {}", task.id);
    println!("Title:       {}", task.title);
    println!("State:       {}", task.state);
    println!(
        "Signal:      {}",
        task.signal
            .as_deref()
            .unwrap_or("(none)")
    );
    if !task.stack.is_empty() {
        println!("Stack:       {:?}", task.stack);
    }
    println!("Pause:       {}", task.pause);
    if let Some(ref repo) = task.destination_repository {
        println!("Dest Repo:   {}", repo);
    }
    if let Some(ref branch) = task.destination_branch {
        println!("Dest Branch: {}", branch);
    }
    if let Some(ref branch) = task.work_branch {
        println!("Work Branch: {}", branch);
    }
    if let Some(ref url) = task.pr_url {
        println!("PR URL: {}", url);
    }
    if !task.description.is_empty() {
        println!("Description:\n{}", task.description);
    }
    // show latest plan comment if present (look for [post_plan] section marker)
    if !discussion.is_empty()
        && let Some(plan_comment) = discussion
            .iter()
            .rev()
            .find(|c| c.text.starts_with("[post_plan]"))
    {
        println!("Plan (from comment):\n{}", plan_comment.text);
    }
    if !task.checklist.is_empty() {
        println!("Checklist:");
        for item in &task.checklist {
            let mark = if item.checked { "[x]" } else { "[ ]" };
            println!("  {} {}", mark, item.text);
        }
    }
    if !discussion.is_empty() {
        println!("Discussion ({} comment(s)):", discussion.len());
        for (i, c) in discussion.iter().enumerate() {
            let tag = CommentTag::new(
                c.stage.clone(),
                c.hostname.clone(),
                c.tool,
                c.model.clone(),
                c.boundary,
                c.hidden,
            );
            println!("  [{}] {}\n{}", i + 1, tag, c.text);
        }
    }
}

// ---------------------------------------------------------------------------
// CliStageRunner — stage execution
// ---------------------------------------------------------------------------

struct CliStageRunner<'a> {
    zbobr: &'a ZbobrDispatcher,
    task_id: u64,
    stage_def: &'a StageDefinition,
    pipeline: &'a PipelineConfig,
}

impl<'a> CliStageRunner<'a> {
    fn new(
        zbobr: &'a ZbobrDispatcher,
        task_id: u64,
        stage_def: &'a StageDefinition,
        pipeline: &'a PipelineConfig,
    ) -> Self {
        Self {
            zbobr,
            task_id,
            stage_def,
            pipeline,
        }
    }

    fn running_state(&self) -> String {
        format!("{}_{}", self.stage_def.mode, self.stage_def.name)
    }

    async fn prompt(&self) -> anyhow::Result<String> {
        self.zbobr
            .prompt_builder()
            .build_for_stage(self.stage_def, self.task_id, &**self.zbobr.task_backend())
            .await
    }

    async fn run(&self) -> anyhow::Result<()> {
        let role = &self.stage_def.role;
        let cli_tool = self.zbobr.config().tool_for_stage(self.stage_def);
        let model = self.zbobr.config().model_for_stage(self.stage_def);

        // Set state to running
        self.zbobr
            .task_session(
                Arc::clone(self.zbobr.task_backend()),
                Arc::clone(self.zbobr.repo_backend()),
                self.task_id,
            )
            .set_state(&self.running_state())
            .await?;

        let task_dir = TaskDir::new(self.zbobr.config().workspaces.as_path(), self.task_id);
        tokio::fs::create_dir_all(task_dir.path()).await?;

        // Seed default config values (unconditional)
        seed_defaults(self.zbobr, self.task_id).await?;

        // Unified worktree detection and problem handling
        let work_dir = match detect_and_handle_worktree(
            self.zbobr,
            self.task_id,
            self.stage_def,
            task_dir.path(),
        )
        .await?
        {
            WorktreeResult::Ready(path) => path,
            WorktreeResult::HandlerCalled | WorktreeResult::Paused => return Ok(()),
        };

        // Ensure PR URL if identity exists
        {
            let task = self.zbobr.task_backend().get_task(self.task_id).await?.snapshot().await?;
            if task.identity().is_some() {
                ensure_pr_url(self.zbobr, self.task_id).await?;
            }
        }

        // Reset worktree retries on successful worktree setup
        reset_worktree_retries(self.zbobr, self.task_id).await?;

        // Clear the triggering signal before the agent session starts.
        {
            let task_session = self.zbobr.task_session(
                Arc::clone(self.zbobr.task_backend()),
                Arc::clone(self.zbobr.repo_backend()),
                self.task_id,
            );
            task_session
                .set_signal(None)
                .await
                .context("Failed to clear signal on stage entry")?;
        }

        // Pre-flight check
        {
            let history =
                crate::get_history(&**self.zbobr.task_backend(), self.task_id, None)
                    .await
                    .context("Pre-flight get_history check failed")?;
            tracing::info!(
                "Task #{} pre-flight: get_history returned {} comment(s)",
                self.task_id,
                history.comments.len()
            );
            if history.comments.is_empty() {
                anyhow::bail!(
                    "Task #{} has no actionable messages — nothing for the agent to do",
                    self.task_id
                );
            }
        }

        let allowed_tools: std::collections::HashSet<String> = self
            .pipeline
            .role_definition(role)
            .map(|d| d.tools.iter().cloned().collect())
            .unwrap_or_else(|| {
                // No explicit role definition — allow all tools for backward compatibility.
                crate::mcp::unified::ALL_TOOL_NAMES
                    .iter()
                    .map(|s| s.to_string())
                    .collect()
            });

        let tool_tracker = Arc::new(std::sync::Mutex::new(None::<String>));
        let comment_buffer: crate::task::CommentBuffer =
            Arc::new(std::sync::Mutex::new(Vec::new()));
        let (assigned_port, server_handle) = start_mcp_server(
            self.zbobr.clone(),
            role,
            self.task_id,
            cli_tool,
            model.clone(),
            self.stage_def.name.clone(),
            self.stage_def.transitions.clone(),
            allowed_tools,
            Arc::clone(&tool_tracker),
            Arc::clone(&comment_buffer),
        )
        .await?;

        let mcp_url = format!(
            "http://127.0.0.1:{}/{}/{}",
            assigned_port, role, self.task_id,
        );

        let prompt_text = self.prompt().await?;
        let executor = self.zbobr.build_executor(cli_tool, model.clone());
        let copilot_token = match cli_tool {
            Tool::Copilot => self.zbobr.copilot_github_token(),
            _ => "",
        };

        let outcome = execute_tool(
            executor,
            copilot_token,
            self.task_id,
            role,
            assigned_port,
            &prompt_text,
            &work_dir,
            &mcp_url,
            self.zbobr,
        )
        .await;

        // Read the last mapped tool from the shared tracker
        let last_mapped_tool = tool_tracker.lock().unwrap().clone();

        if let Some(e) = finalize_stage_session(
            self.zbobr,
            self.task_id,
            self.stage_def,
            self.pipeline,
            &work_dir,
            outcome,
            last_mapped_tool.as_deref(),
            comment_buffer,
        )
        .await?
        {
            server_handle.abort();
            return Err(e);
        }

        server_handle.abort();

        Ok(())
    }
}

/// Compute the post-execution signal from the stage's transitions map.
/// `last_mapped_tool` is the last MCP tool call that matched a transition key.
/// Falls back to "default" transition, then to "return" if nothing matches.
fn compute_post_stage_signal(
    stage_def: &StageDefinition,
    last_mapped_tool: Option<&str>,
) -> String {
    if let Some(tool_name) = last_mapped_tool {
        if let Some(signal) = stage_def.transitions.get(tool_name) {
            return signal.clone();
        }
    }
    if let Some(signal) = stage_def.transitions.get("default") {
        return signal.clone();
    }
    "return".to_string()
}

/// Parse a compound call signal like "call_aux,go_step_three".
/// Returns `Some((call_part, after_return))` if compound, `None` otherwise.
fn parse_compound_call(signal: &str) -> Option<(&str, &str)> {
    if !signal.starts_with("call_") {
        return None;
    }
    let (call_part, after) = signal.split_once(',')?;
    Some((call_part, after.trim()))
}

/// Result of worktree detection before a stage runs.
enum WorktreeResult {
    /// Worktree is ready; proceed with stage execution at this path.
    Ready(PathBuf),
    /// A worktree problem handler mode was called; the caller should return.
    HandlerCalled,
    /// The task was paused due to unresolvable worktree problem.
    Paused,
}

// ---------------------------------------------------------------------------
// Stage processing helpers
// ---------------------------------------------------------------------------

/// Process a task according to its current state and signal (single-step).
pub async fn process_task(
    zbobr: &ZbobrDispatcher,
    task: &Task,
    pipeline: &PipelineConfig,
) -> anyhow::Result<()> {
    if task.state == "DONE" {
        println!("Task #{} is DONE — nothing to process", task.id);
        return Ok(());
    }
    if task.state == "PAUSE" || task.pause {
        println!("Task #{} is paused — skipped", task.id);
        return Ok(());
    }

    // Use state machine to determine what to do
    let action = crate::state_machine::resolve_next_action(task, pipeline)?;
    match action {
        crate::state_machine::StateAction::RunStage(stage_def) => {
            let runner = CliStageRunner::new(zbobr, task.id, stage_def, pipeline);
            runner.run().await?;
        }
        crate::state_machine::StateAction::Done => {
            let task_session = zbobr.task_session(
                Arc::clone(zbobr.task_backend()),
                Arc::clone(zbobr.repo_backend()),
                task.id,
            );
            if let Some(entry) = task_session.pop_stack().await? {
                // Return from sub-mode — fire the stored after-return signal
                task_session.set_signal(Some(&entry.signal)).await?;
                task_session
                    .set_state(&format!("{}_PENDING", entry.mode))
                    .await?;
                println!(
                    "Task #{} returning to mode '{}' with signal '{}'",
                    task.id, entry.mode, entry.signal
                );
            } else {
                task_session.finish().await?;
                println!("Task #{} completed", task.id);
            }
        }
        crate::state_machine::StateAction::Paused => {
            println!("Task #{} is paused — skipped", task.id);
        }
        crate::state_machine::StateAction::Idle => {
            println!("Task #{} is idle (state={}, signal={:?}) — skipped", task.id, task.state, task.signal);
        }
    }
    Ok(())
}

/// Main manager loop: polls for tasks and dispatches role sessions.
pub async fn run_manager_loop(
    zbobr: &ZbobrDispatcher,
    interval_secs: u64,
    cleanup_interval_secs: u64,
    pipeline: &PipelineConfig,
) -> anyhow::Result<()> {
    let task_backend = zbobr.task_backend();
    let repo_backend = zbobr.repo_backend();
    let prompt_builder = zbobr.prompt_builder();
    tracing::info!(
        "Manager loop started (task_backend: {}, repo_backend: {})",
        task_backend.debug_state(),
        repo_backend.debug_state()
    );
    tracing::info!("Poll interval: {interval_secs}s, Cleanup interval: {cleanup_interval_secs}s");
    tracing::info!("Global CLI Tool default: {:?}", zbobr.config().tool);
    tracing::info!("Global model default: {:?}", zbobr.config().model);
    if let Some(base) = prompt_builder.base_path() {
        tracing::info!("Prompts base path: {}", base.display());
    }

    // Dump stage-specific settings for visibility
    for stage_def in &pipeline.stages {
        let tool = zbobr.config().tool_for_stage(stage_def);
        let model = zbobr.config().model_for_stage(stage_def);
        tracing::info!(
            "Stage {}/{}: role={:?}, tool={:?}, model={:?}, prompts={:?}",
            stage_def.mode,
            stage_def.name,
            stage_def.role,
            tool,
            model,
            stage_def.main_prompt
        );
    }

    let mut last_cleanup = std::time::Instant::now();

    loop {
        let loop_start = std::time::Instant::now();

        if last_cleanup.elapsed().as_secs() >= cleanup_interval_secs {
            tracing::info!("Running workspaces cleanup...");
            if let Err(e) = zbobr.cleanup_closed_tasks(&**task_backend, false).await {
                tracing::warn!("Cleanup failed: {e}");
            }
            last_cleanup = std::time::Instant::now();
        }

        let all_weak = match task_backend.list_tasks().await {
            Ok(tasks) => tasks,
            Err(e) => {
                tracing::error!("Failed to list tasks: {e}");
                vec![]
            }
        };
        let mut all_tasks: Vec<Task> = Vec::new();
        for w in &all_weak {
            match w.snapshot().await {
                Ok(t) => all_tasks.push(t),
                Err(e) => tracing::warn!("Failed to snapshot task: {e}"),
            }
        }

        let mut session_run = false;
        for task in &all_tasks {
            if task.pause || task.state == "DONE" {
                continue;
            }

            let action = match crate::state_machine::resolve_next_action(task, pipeline) {
                Ok(a) => a,
                Err(e) => {
                    tracing::warn!("State machine error for task #{}: {e}", task.id);
                    continue;
                }
            };

            match action {
                crate::state_machine::StateAction::RunStage(stage_def) => {
                    tracing::info!(
                        "Processing task #{} (state={}, signal={:?}) — running stage {}/{}",
                        task.id,
                        task.state,
                        task.signal,
                        stage_def.mode,
                        stage_def.name,
                    );
                    let runner = CliStageRunner::new(zbobr, task.id, stage_def, pipeline);
                    if let Err(e) = runner.run().await {
                        tracing::error!("Stage {}/{} failed for task #{}: {e}", stage_def.mode, stage_def.name, task.id);
                    }
                    session_run = true;
                    break;
                }
                crate::state_machine::StateAction::Done => {
                    let task_session = zbobr.task_session(
                        Arc::clone(task_backend),
                        Arc::clone(repo_backend),
                        task.id,
                    );
                    match task_session.pop_stack().await {
                        Ok(Some(entry)) => {
                            // Return from sub-mode — fire stored after-return signal
                            if let Err(e) = task_session.set_signal(Some(&entry.signal)).await {
                                tracing::error!("Failed to set return signal for task #{}: {e}", task.id);
                            }
                            if let Err(e) = task_session
                                .set_state(&format!("{}_PENDING", entry.mode))
                                .await
                            {
                                tracing::error!("Failed to set return state for task #{}: {e}", task.id);
                            }
                        }
                        Ok(None) => {
                            if let Err(e) = task_session.finish().await {
                                tracing::error!("Failed to finish task #{}: {e}", task.id);
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to pop stack for task #{}: {e}", task.id);
                        }
                    }
                }
                crate::state_machine::StateAction::Paused | crate::state_machine::StateAction::Idle => {}
            }
        }

        if session_run {
            continue;
        }

        // Task statistics
        let mut state_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for task in &all_tasks {
            *state_counts.entry(task.state.clone()).or_default() += 1;
        }
        let stats: Vec<String> = state_counts.iter().map(|(k, v)| format!("{k}={v}")).collect();
        tracing::info!("Task statistics: {}", stats.join(", "));

        let elapsed = loop_start.elapsed();
        let sleep_dur = std::time::Duration::from_secs(interval_secs).saturating_sub(elapsed);
        if sleep_dur.is_zero() {
            tracing::info!(
                "No processable tasks. Interval already elapsed, continuing immediately."
            );
        } else {
            tracing::info!("No processable tasks. Sleeping {}s...", sleep_dur.as_secs());
            tokio::select! {
                _ = tokio::time::sleep(sleep_dur) => {}
                _ = tokio::signal::ctrl_c() => {
                    tracing::info!("Received shutdown signal, exiting...");
                    break;
                }
            }
        }
    }

    tracing::info!("Manager loop terminated gracefully");
    Ok(())
}

// ---------------------------------------------------------------------------
// Low-level helpers
// ---------------------------------------------------------------------------

/// Unified worktree detection and problem dispatch.
///
/// Checks whether the task has a valid identity (routing params), sets up the
/// worktree, and attempts to merge upstream. If a problem is detected (undefined
/// identity or merge conflict), dispatches to the configured handler mode.
async fn detect_and_handle_worktree(
    zbobr: &ZbobrDispatcher,
    task_id: u64,
    stage_def: &StageDefinition,
    task_dir: &Path,
) -> anyhow::Result<WorktreeResult> {
    let task_backend = zbobr.task_backend();
    let repo_backend = zbobr.repo_backend();
    let task = task_backend.get_task(task_id).await?.snapshot().await?;

    // 1. Check if identity is defined
    let identity = match task.identity() {
        Some(id) => id,
        None => {
            // If we ARE the undefined handler mode, proceed with task_dir
            if zbobr.config().on_undefined.as_deref() == Some(&stage_def.mode) {
                return Ok(WorktreeResult::Ready(task_dir.to_path_buf()));
            }
            // Otherwise, dispatch to undefined handler
            return handle_worktree_problem(
                zbobr,
                task_id,
                stage_def,
                zbobr_api::task::WorktreeProblem::Undefined,
            )
            .await;
        }
    };

    // 2. Update worktree
    let is_uptodate = match zbobr.update_worktree(&**repo_backend, &identity).await {
        Ok(up) => up,
        Err(e) => {
            let msg = format!("Failed to prepare workspace for task #{task_id}: {e:#}");
            tracing::error!("{msg}");
            let hostname = get_hostname();
            if let Err(post_err) = zbobr
                .task_session(Arc::clone(task_backend), Arc::clone(repo_backend), task_id)
                .post_comment("error", &hostname, None, None, &msg, false, true)
                .await
            {
                tracing::warn!("Failed to post error to task discussion: {post_err}");
            }
            return Err(anyhow::anyhow!(msg));
        }
    };

    // 3. Compute work_dir from identity
    let dest_repo = &identity.destination_repository;
    let repo_name = dest_repo.rsplit('/').next().unwrap_or(dest_repo);
    let work_dir = TaskDir::new(zbobr.config().workspaces.as_path(), task_id)
        .path()
        .join(repo_name);

    // 4. If up-to-date, no merge needed
    if is_uptodate {
        return Ok(WorktreeResult::Ready(work_dir));
    }

    // 5. Attempt merge
    let dest_branch = task
        .destination_branch
        .clone()
        .unwrap_or_else(|| "main".to_string());

    // If we ARE the conflict handler, start the merge but don't abort on failure —
    // the agent needs to see conflict markers in the working tree.
    let is_conflict_handler = zbobr.config().on_conflict.as_deref() == Some(&stage_def.mode);

    let merged_ok = git_check(
        &work_dir,
        &["merge", &format!("origin/{}", dest_branch), "--no-edit"],
    )
    .await
    .context("Failed to run git merge for upstream sync")?;

    if merged_ok {
        return Ok(WorktreeResult::Ready(work_dir));
    }

    if is_conflict_handler {
        // We're the conflict handler — leave the tree in conflicted state
        // so the agent can resolve the merge markers.
        tracing::info!(
            "Task #{task_id}: merge failed inside conflict handler — agent will resolve"
        );
        return Ok(WorktreeResult::Ready(work_dir));
    }

    // Merge failed in a normal mode — abort and dispatch to conflict handler
    let _ = git(&work_dir, &["merge", "--abort"]).await;

    let Some(ref _conflict_mode) = zbobr.config().on_conflict else {
        // No on_conflict configured — continue with conflicts in working tree (backward compat).
        tracing::warn!(
            "Task #{task_id}: upstream merge failed but no on_conflict configured"
        );
        return Ok(WorktreeResult::Ready(work_dir));
    };

    handle_worktree_problem(
        zbobr,
        task_id,
        stage_def,
        zbobr_api::task::WorktreeProblem::Conflict,
    )
    .await
}

/// Unified dispatch for worktree problems (Undefined identity or Conflict).
async fn handle_worktree_problem(
    zbobr: &ZbobrDispatcher,
    task_id: u64,
    stage_def: &StageDefinition,
    problem: zbobr_api::task::WorktreeProblem,
) -> anyhow::Result<WorktreeResult> {
    let config = zbobr.config();
    let (handler_mode, max_retries) = match problem {
        zbobr_api::task::WorktreeProblem::Undefined => {
            match config.on_undefined.as_deref() {
                Some(m) => (m, config.max_retries_undefined),
                None => {
                    anyhow::bail!(
                        "Task #{task_id} has no routing parameters and no on_undefined handler configured"
                    );
                }
            }
        }
        zbobr_api::task::WorktreeProblem::Conflict => {
            match config.on_conflict.as_deref() {
                Some(m) => (m, config.max_retries_conflict),
                None => {
                    // Backward compat: no handler, warn and continue
                    tracing::warn!(
                        "Task #{task_id}: worktree problem {:?} but no handler configured",
                        problem
                    );
                    // Can't return Ready since we don't have a work_dir in all cases
                    anyhow::bail!(
                        "Task #{task_id}: worktree conflict with no on_conflict handler"
                    );
                }
            }
        }
    };

    let task_session = zbobr.task_session(
        Arc::clone(zbobr.task_backend()),
        Arc::clone(zbobr.repo_backend()),
        task_id,
    );
    let pending_state = format!("{}_PENDING", stage_def.mode);

    // Recursion guard: if already inside the handler mode, pause
    if stage_def.mode == handler_mode {
        tracing::error!(
            "Task #{task_id}: worktree problem {:?} inside handler mode '{handler_mode}' — pausing",
            problem
        );
        let hostname = get_hostname();
        let msg = format!(
            "Worktree problem {:?} inside handler mode '{handler_mode}'. Manual intervention required.",
            problem
        );
        task_session
            .post_comment("error", &hostname, None, None, &msg, false, true)
            .await
            .ok();
        task_session
            .modify_task(|mut t| {
                t.pause = true;
                t
            })
            .await?;
        task_session.set_state(&pending_state).await?;
        return Ok(WorktreeResult::Paused);
    }

    // Retry limit check
    let task = task_session.get_task().await?;
    if task.worktree_retries >= max_retries {
        tracing::error!(
            "Task #{task_id}: worktree problem {:?} retry limit ({max_retries}) reached — pausing",
            problem
        );
        let hostname = get_hostname();
        let msg = format!(
            "Worktree problem {:?} retry limit ({max_retries}) reached. Manual intervention required.",
            problem
        );
        task_session
            .post_comment("error", &hostname, None, None, &msg, false, true)
            .await
            .ok();
        task_session
            .modify_task(|mut t| {
                t.pause = true;
                t
            })
            .await?;
        task_session.set_state(&pending_state).await?;
        return Ok(WorktreeResult::Paused);
    }

    // Increment worktree_retries
    task_session
        .modify_task(|mut t| {
            t.worktree_retries += 1;
            t
        })
        .await?;

    // Push stack: re-run the interrupted stage upon return
    task_session
        .push_stack(&stage_def.mode, &format!("go_{}", stage_def.name))
        .await?;
    task_session
        .set_signal(Some(&format!("call_{}", handler_mode)))
        .await?;
    task_session.set_state(&pending_state).await?;

    tracing::info!(
        "Task #{task_id}: worktree problem {:?} — calling handler mode '{handler_mode}'",
        problem
    );
    Ok(WorktreeResult::HandlerCalled)
}

/// Reset worktree retries counter when a stage proceeds normally.
async fn reset_worktree_retries(
    zbobr: &ZbobrDispatcher,
    task_id: u64,
) -> anyhow::Result<()> {
    let task_session = zbobr.task_session(
        Arc::clone(zbobr.task_backend()),
        Arc::clone(zbobr.repo_backend()),
        task_id,
    );
    let task = task_session.get_task().await?;
    if task.worktree_retries > 0 {
        task_session
            .modify_task(|mut t| {
                t.worktree_retries = 0;
                t
            })
            .await?;
    }
    Ok(())
}

async fn ensure_pr_url(
    zbobr: &ZbobrDispatcher,
    task_id: u64,
) -> anyhow::Result<()> {
    let task_backend = zbobr.task_backend();
    let repo_backend = zbobr.repo_backend();
    let role_session = zbobr.role_session(Arc::clone(task_backend), task_id);
    let task = role_session.get_task().await?;
    if task.pr_url.is_some() {
        return Ok(());
    }
    let identity = match task.identity() {
        Some(id) => id,
        None => {
            let msg = format!(
                "Task #{task_id} is missing routing parameters (destination_repository, destination_branch, work_branch)"
            );
            tracing::error!("{msg}");
            return Err(anyhow::anyhow!(msg));
        }
    };
    match repo_backend.update_pr(&identity).await {
        Ok(pr_url) => {
            role_session
                .modify_task(move |mut task| {
                    task.pr_url = Some(pr_url);
                    task
                })
                .await?;
            Ok(())
        }
        Err(e) => {
            let msg = format!("Could not ensure PR URL for task #{task_id}: {e}");
            tracing::error!("{msg}");
            let hostname = get_hostname();
            let task_session =
                zbobr.task_session(Arc::clone(task_backend), Arc::clone(repo_backend), task_id);
            if let Err(post_err) = task_session
                .post_comment("error", &hostname, None, None, &msg, false, true)
                .await
            {
                tracing::warn!("Failed to post error to task discussion: {post_err}");
            }
            Err(anyhow::anyhow!(msg))
        }
    }
}

/// Pre-populate task parameters from dispatcher config defaults.
/// Only sets a parameter if it is not already present, so a previously
/// prepared task keeps its values unchanged. Called unconditionally at
/// the start of every stage run.
async fn seed_defaults(
    zbobr: &ZbobrDispatcher,
    task_id: u64,
) -> anyhow::Result<()> {
    let task_backend = zbobr.task_backend();
    let config = zbobr.config();
    let task = task_backend.get_task(task_id).await?.snapshot().await?;
    let role_session = zbobr.role_session(Arc::clone(task_backend), task_id);

    if let Some(default_repo) = &config.default_destination_repository
        && task.destination_repository.is_none()
    {
        role_session
            .set_destination_repository(Some(default_repo.clone()))
            .await?;
    }

    if let Some(default_branch) = &config.default_destination_branch
        && task.destination_branch.is_none()
    {
        role_session
            .set_destination_branch(Some(default_branch.clone()))
            .await?;
    }

    Ok(())
}

async fn start_mcp_server(
    zbobr: ZbobrDispatcher,
    role_name: &str,
    task_id: u64,
    tool: Tool,
    model: Model,
    stage_name: String,
    transitions: std::collections::HashMap<String, String>,
    allowed_tools: std::collections::HashSet<String>,
    tool_tracker: Arc<std::sync::Mutex<Option<String>>>,
    comment_buffer: crate::task::CommentBuffer,
) -> anyhow::Result<(u16, tokio::task::JoinHandle<()>)> {
    let (port_tx, port_rx) = tokio::sync::oneshot::channel();
    let task_backend = Arc::clone(zbobr.task_backend());
    let role_name = role_name.to_string();
    let server_handle = tokio::spawn(async move {
        match crate::mcp::run_role_mcp_server(zbobr, task_backend, &role_name, task_id, tool, model, stage_name, transitions, allowed_tools, tool_tracker, comment_buffer).await
        {
            Ok(assigned_port) => {
                let _ = port_tx.send(assigned_port);
                tracing::info!("MCP server assigned port {assigned_port}");
            }
            Err(e) => {
                tracing::error!("MCP server error: {e}");
            }
        }
    });

    let assigned_port = tokio::time::timeout(std::time::Duration::from_secs(5), port_rx)
        .await
        .context("MCP server failed to report assigned port in time")?
        .context("MCP server task dropped before sending port")?;

    Ok((assigned_port, server_handle))
}

struct SessionOutcome {
    execution_interrupted: bool,
    execution_error: Option<anyhow::Error>,
}

#[allow(clippy::too_many_arguments)]
async fn execute_tool(
    executor: Box<dyn ToolExecutor>,
    copilot_token: &str,
    task_id: u64,
    role: &str,
    assigned_port: u16,
    prompt: &str,
    work_dir: &Path,
    mcp_url: &str,
    zbobr: &ZbobrDispatcher,
) -> SessionOutcome {
    let agent_token = &zbobr.config().agent_github_token;

    tokio::select! {
        result = executor.execute(task_id, role, assigned_port, prompt, work_dir, mcp_url, agent_token, copilot_token) => {
            match result {
                Ok(()) => SessionOutcome {
                    execution_interrupted: false,
                    execution_error: None,
                },
                Err(e) => {
                    tracing::error!("Tool execution failed: {e}");
                    SessionOutcome {
                        execution_interrupted: false,
                        execution_error: Some(e),
                    }
                }
            }
        }
        _ = tokio::signal::ctrl_c() => {
            tracing::warn!("Received shutdown signal during execution");
            SessionOutcome {
                execution_interrupted: true,
                execution_error: None,
            }
        }
    }
}

async fn finalize_stage_session(
    zbobr: &ZbobrDispatcher,
    task_id: u64,
    stage_def: &StageDefinition,
    _pipeline: &PipelineConfig,
    work_dir: &Path,
    outcome: SessionOutcome,
    last_mapped_tool: Option<&str>,
    comment_buffer: crate::task::CommentBuffer,
) -> anyhow::Result<Option<anyhow::Error>> {
    let role = stage_def.role.as_str();
    let task_backend = zbobr.task_backend();
    let repo_backend = zbobr.repo_backend();
    let task_session = zbobr.task_session(Arc::clone(task_backend), Arc::clone(repo_backend), task_id);
    let pending_state = format!("{}_PENDING", stage_def.mode);

    // Flush buffered MCP comments as a single combined comment signed by stage name.
    {
        let buffered: Vec<crate::task::BufferedComment> = {
            let mut buf = comment_buffer.lock().unwrap();
            std::mem::take(&mut *buf)
        };
        if !buffered.is_empty() {
            let boundary = buffered.iter().any(|c| c.boundary);
            let hostname = get_hostname();
            let combined_text = buffered
                .iter()
                .map(|c| c.body.as_str())
                .collect::<Vec<_>>()
                .join("\n\n");
            let cli_tool = zbobr.config().tool_for_stage(stage_def);
            let model = zbobr.config().model_for_stage(stage_def);
            if let Err(e) = task_session
                .post_comment(
                    &stage_def.name,
                    &hostname,
                    Some(cli_tool),
                    Some(model),
                    &combined_text,
                    boundary,
                    false,
                )
                .await
            {
                tracing::error!(
                    "Failed to flush buffered comments for task #{task_id}: {e}"
                );
            }
        }
    }

    if outcome.execution_interrupted {
        if let Err(e) = perform_stash_and_push(zbobr, task_id, work_dir, role).await {
            tracing::warn!("Stash/push failed during interruption for task #{task_id}: {e}");
        }
        task_session.set_state(&pending_state).await?;
        tracing::info!("Session interrupted for task #{task_id}, moved to {pending_state}");
        return Ok(None);
    }

    if let Some(e) = outcome.execution_error.as_ref() {
        if let Err(e) = perform_stash_and_push(zbobr, task_id, work_dir, role).await {
            tracing::warn!(
                "Stash/push failed during error handling for task #{task_id}: {e}"
            );
        }
        let error_msg = format!("Execution failed: {e}");
        let hostname = get_hostname();
        if let Err(post_err) = task_session
            .post_comment("error", &hostname, None, None, &error_msg, false, true)
            .await
        {
            tracing::error!("Failed to post error to task #{task_id}: {post_err}");
        }
        if let Err(pause_err) = task_session
            .modify_task(|mut task| {
                task.pause = true;
                task
            })
            .await
        {
            tracing::error!("Failed to set pause for task #{task_id}: {pause_err}");
        }
        task_session.set_state(&pending_state).await?;
        tracing::info!("Session failed for task #{task_id}, moved to {pending_state} with pause");
        return Ok(outcome.execution_error);
    }

    tracing::info!("Session complete for task #{task_id}");

    if let Err(e) = perform_stash_and_push(zbobr, task_id, work_dir, role).await {
        tracing::error!("Stash/push failed for task #{task_id}: {e}");
        let hostname = get_hostname();
        let msg = format!("Stash/push failed: {e}");
        if let Err(post_err) = task_session
            .post_comment("error", &hostname, None, None, &msg, false, true)
            .await
        {
            tracing::error!(
                "Failed to post stash/push error for task #{task_id}: {post_err}"
            );
        }
        if let Err(pause_err) = task_session
            .modify_task(|mut task| {
                task.pause = true;
                task
            })
            .await
        {
            tracing::error!(
                "Failed to pause task #{task_id} after stash/push failure: {pause_err}"
            );
        }
        task_session.set_state(&pending_state).await?;
        return Ok(None);
    }

    // Compute post-stage signal from transitions map.
    // If the agent already set a signal during the session (e.g. reject),
    // that signal takes priority.
    let current_task = task_backend.get_task(task_id).await?.snapshot().await?;
    if !current_task.pause && current_task.signal.is_none() {
        let raw_signal = compute_post_stage_signal(stage_def, last_mapped_tool);
        if let Some((call_part, after_return)) = parse_compound_call(&raw_signal) {
            // Push after-return signal onto stack, then set the call signal
            task_session
                .push_stack(&stage_def.mode, after_return)
                .await?;
            task_session.set_signal(Some(call_part)).await?;
        } else {
            task_session.set_signal(Some(&raw_signal)).await?;
        }
    }
    task_session.set_state(&pending_state).await?;

    Ok(None)
}

async fn perform_stash_and_push(
    zbobr: &ZbobrDispatcher,
    task_id: u64,
    work_dir: &Path,
    role: &str,
) -> anyhow::Result<()> {
    let task_backend = zbobr.task_backend();
    let repo_backend = zbobr.repo_backend();
    tracing::info!("Checking for uncommitted changes in {}", work_dir.display());

    match git_output(work_dir, &["status", "--porcelain"]).await {
        Ok(status) => {
            if !status.is_empty() {
                let stash_msg = format!("Stashed by {} agent for task #{}", role, task_id);
                tracing::info!("Found uncommitted changes, stashing...");
                match git(work_dir, &["stash", "push", "--include-untracked", "-m", &stash_msg]).await {
                    Ok(_) => tracing::info!("Git stash successful"),
                    Err(e) => tracing::warn!("Git stash failed: {e}"),
                }
            } else {
                tracing::info!("No uncommitted changes found");
            }
        }
        Err(e) => tracing::warn!("Failed to check git status for stash: {e}"),
    }

    let task = task_backend.get_task(task_id).await?.snapshot().await?;
    if let Some(identity) = task.identity() {
        if let Err(e) = repo_backend.update_pr(&identity).await {
            tracing::warn!("Could not push branch commits for task #{task_id}: {e}");
        }
        let dest_branch = identity.destination_branch.clone();
        repo_backend
            .rewrite_commit_authors(&identity, work_dir, &dest_branch)
            .await?;
    } else {
        tracing::warn!("Task #{task_id} missing routing parameters — skipping push");
    }

    Ok(())
}
