#![allow(clippy::needless_borrows_for_generic_args)]

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Context;
use clap::{Args, Parser};
use zbobr_api::{
    CommentTag, Pipeline, Signal, StackEntry, State, config::StageDefinition, config_tools::McpTool,
};
use zbobr_executor_mcp_tester::ZbobrExecutorMcpTesterConfig;
// bring in the generic git helpers from utility crate
use zbobr_utility::{git, git_check, git_output};

use crate::{
    Comment, Task, TaskDir, ToolExecutor, ZbobrDispatcher,
    mcp::common::get_hostname,
    task::{Model, Tool},
};

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
            .as_ref()
            .map(|s| s.to_string())
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
    // show latest plan comment if present (look for [report_success] or legacy [post_plan] marker)
    if !discussion.is_empty()
        && let Some(plan_comment) = discussion
            .iter()
            .rev()
            .find(|c| c.text.starts_with("[report_success]") || c.text.starts_with("[post_plan]"))
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
            let mut tag = CommentTag::new(
                c.pipeline.clone(),
                c.pipeline_run_id,
                c.stage.clone(),
                c.hostname.clone(),
                c.tool,
                c.model.clone(),
            );
            if let (Some(caller_pipeline), Some(caller_run_id)) =
                (c.caller_pipeline.clone(), c.caller_pipeline_run_id)
            {
                tag = tag.with_caller(caller_pipeline, caller_run_id);
            }
            println!("  [{}] {}\n{}", i + 1, tag, c.text);
        }
    }
}

// ---------------------------------------------------------------------------
// CliStageRunner — stage execution
// ---------------------------------------------------------------------------

struct CliStageRunner<'a> {
    zbobr: &'a Arc<ZbobrDispatcher>,
    task_id: u64,
    pipeline_name: &'a Pipeline,
    stage_name: &'a str,
    stage_def: &'a StageDefinition,
    mcp_tester_override: Option<&'a ZbobrExecutorMcpTesterConfig>,
}

impl<'a> CliStageRunner<'a> {
    fn new(
        zbobr: &'a Arc<ZbobrDispatcher>,
        task_id: u64,
        pipeline_name: &'a Pipeline,
        stage_name: &'a str,
        stage_def: &'a StageDefinition,
        mcp_tester_override: Option<&'a ZbobrExecutorMcpTesterConfig>,
    ) -> Self {
        Self {
            zbobr,
            task_id,
            pipeline_name,
            stage_name,
            stage_def,
            mcp_tester_override,
        }
    }

    fn running_state(&self) -> State {
        State::running(self.pipeline_name.clone(), self.stage_name)
    }

    async fn prompt(&self, pipeline_run_id: u64) -> anyhow::Result<String> {
        let scope = Some((self.pipeline_name.as_str(), pipeline_run_id));
        self.zbobr
            .prompt_builder()
            .build_for_stage(self.stage_def, self.task_id, self.zbobr.task_backend(), scope)
            .await
    }

    async fn run(&self) -> anyhow::Result<()> {
        let role = self
            .stage_def
            .role_name()
            .expect("role stage must have role");
        let cli_tool = self
            .zbobr
            .config()
            .tool_for_stage(self.stage_def, self.zbobr.workflow().config());
        let model = self
            .zbobr
            .config()
            .model_for_stage(self.stage_def, self.zbobr.workflow().config());

        // Set state to running
        self.zbobr
            .task_session(self.task_id)
            .set_state(self.running_state())
            .await?;

        let task_dir = TaskDir::new(self.zbobr.config().workspaces.as_path(), self.task_id);
        tokio::fs::create_dir_all(task_dir.path()).await?;

        // Unified worktree detection and problem handling
        let work_dir = match detect_and_handle_worktree(
            self.zbobr,
            self.task_id,
            self.pipeline_name,
            self.stage_name,
            task_dir.path(),
        )
        .await?
        {
            WorktreeResult::Ready(path) => path,
            WorktreeResult::HandlerCalled | WorktreeResult::Paused => return Ok(()),
        };

        // Ensure PR URL if identity exists
        {
            let task = self
                .zbobr
                .task_backend()
                .get_task(self.task_id)
                .await?
                .snapshot(false)
                .await?;
            if task.identity().is_some() {
                ensure_pr_url(self.zbobr, self.task_id).await?;
            }
        }

        // Allocate pipeline run ID if this is a fresh task (run_id == 0).
        {
            let task_session = self.zbobr.task_session(self.task_id);
            let task = task_session.get_task().await?;
            if task.pipeline_run_id == 0 {
                task_session.allocate_pipeline_run_id().await?;
            }
        }

        // Clear the triggering signal before the agent session starts.
        {
            let task_session = self.zbobr.task_session(self.task_id);
            task_session
                .set_signal(None)
                .await
                .context("Failed to clear signal on stage entry")?;
        }

        // Pre-flight check
        {
            let weak = self.zbobr.task_backend().get_task(self.task_id).await?;
            let task_snap = weak.snapshot(false).await?;
            if task_snap.description.is_empty() {
                anyhow::bail!(
                    "Task #{} has no description — nothing for the agent to do",
                    self.task_id
                );
            }
        }

        let allowed_tools: std::collections::HashSet<McpTool> = self
            .zbobr
            .workflow()
            .role_definition(role)
            .map(|d| d.mcp.iter().copied().collect())
            .unwrap_or_else(|| {
                // No explicit role definition — allow all tools for backward compatibility.
                self.zbobr
                    .workflow()
                    .config()
                    .all_tool_names()
                    .into_iter()
                    .filter_map(|name| name.parse::<McpTool>().ok())
                    .collect()
            });

        // Read current pipeline_run_id for this session.
        let task_snap = self
            .zbobr
            .task_backend()
            .get_task(self.task_id)
            .await?
            .snapshot(false)
            .await?;
        let pipeline_run_id = task_snap.pipeline_run_id;

        let tool_tracker = Arc::new(std::sync::Mutex::new(None::<String>));
        let prompt_holder = Arc::new(std::sync::Mutex::new(None::<String>));
        let (assigned_port, server_handle) = start_mcp_server(
            Arc::clone(self.zbobr),
            role,
            self.task_id,
            cli_tool,
            model.clone(),
            self.stage_name.to_string(),
            allowed_tools,
            Arc::clone(&tool_tracker),
            self.pipeline_name.to_string(),
            pipeline_run_id,
            Arc::clone(&prompt_holder),
        )
        .await?;

        let mcp_url = format!(
            "http://127.0.0.1:{}/{}/{}",
            assigned_port, role, self.task_id,
        );

        let prompt_text = self.prompt(pipeline_run_id).await?;
        *prompt_holder.lock().unwrap() = Some(prompt_text.clone());
        let executor = self
            .zbobr
            .build_executor(cli_tool, model.clone(), self.mcp_tester_override);
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

        // Read the last mapped tool from the shared tracker.
        let last_mapped_tool = tool_tracker.lock().unwrap().clone();

        if let Some(e) = finalize_stage_session(
            self.zbobr,
            self.task_id,
            self.pipeline_name,
            self.stage_name,
            &work_dir,
            outcome,
            last_mapped_tool.as_deref(),
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

/// Result of computing the post-stage signal in the sequential pipeline model.
enum SequentialSignal {
    /// `report_failure` → immediate return from pipeline.
    ReturnFailure,
    /// `report_success` with a next stage → advance to it.
    Advance(String),
    /// `report_success` at the last stage → pipeline done, return.
    Return,
    /// No report tool called (crash/timeout/stop_with_error) → pause.
    Pause,
    /// Stage-configured pause → set pause flag, emit given signal on resume.
    PauseThenSignal(Signal),
}

/// Compute the post-execution signal for the sequential pipeline model.
fn compute_sequential_signal(
    pipeline_name: &Pipeline,
    stage_name: &str,
    stage_def: Option<&zbobr_api::config::StageDefinition>,
    workflow: &crate::workflow::Workflow,
    last_mapped_tool: Option<&str>,
) -> SequentialSignal {
    match last_mapped_tool {
        Some("report_failure") => {
            let transition = stage_def.and_then(|s| s.on_failure());
            let target = transition.and_then(|t| t.next.as_ref());
            let should_pause = transition.map_or(false, |t| t.pause);

            let signal = if let Some(target) = target {
                Signal::go(target.as_str())
            } else {
                Signal::ReturnFailure
            };

            if should_pause {
                SequentialSignal::PauseThenSignal(signal)
            } else if target.is_some() {
                SequentialSignal::Advance(target.unwrap().to_string())
            } else {
                SequentialSignal::ReturnFailure
            }
        }
        Some("report_success") => {
            let transition = stage_def.and_then(|s| s.on_success());
            let explicit_target = transition.and_then(|t| t.next.as_ref());
            let should_pause = transition.map_or(false, |t| t.pause);

            let advance_target = if let Some(target) = explicit_target {
                Some(target.to_string())
            } else {
                workflow
                    .pipeline(pipeline_name)
                    .and_then(|p| p.next_stage(stage_name))
                    .map(|(next, _)| next.to_string())
            };

            if should_pause {
                let signal = match advance_target {
                    Some(next) => Signal::go(next),
                    None => Signal::Return,
                };
                SequentialSignal::PauseThenSignal(signal)
            } else {
                match advance_target {
                    Some(next) => SequentialSignal::Advance(next),
                    None => SequentialSignal::Return,
                }
            }
        }
        _ => SequentialSignal::Pause,
    }
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

/// Handle a stage that calls another pipeline instead of running an agent.
///
/// Pushes the current pipeline onto the stack with the appropriate return signal
/// (advance to next stage or return), then emits a `call_<pipeline>` signal.
async fn handle_call_stage(
    zbobr: &Arc<ZbobrDispatcher>,
    task_id: u64,
    pipeline_name: &Pipeline,
    stage_name: &str,
    call_pipeline: &Pipeline,
) -> anyhow::Result<()> {
    let task_session = zbobr.task_session(task_id);

    // Determine what signal to emit when the called pipeline returns.
    let stage_def = zbobr.workflow().stage(pipeline_name, stage_name);
    let return_signal = if let Some(target) = stage_def
        .and_then(|s| s.on_success())
        .and_then(|t| t.next.as_ref())
    {
        Signal::go(target.as_str())
    } else {
        match zbobr
            .workflow()
            .pipeline(pipeline_name)
            .and_then(|p| p.next_stage(stage_name))
        {
            Some((next, _)) => Signal::go(next),
            None => Signal::Return,
        }
    };

    // Push stack so we return to the right place.
    task_session
        .push_stack(pipeline_name.clone(), return_signal.clone())
        .await?;
    task_session.allocate_pipeline_run_id().await?;
    let call_signal = Signal::call(call_pipeline.clone());
    task_session.set_signal(Some(call_signal.clone())).await?;
    task_session
        .set_state(State::pending(pipeline_name.clone()))
        .await?;

    tracing::info!(
        "Task #{task_id}: stage {pipeline_name}/{stage_name} calling pipeline '{call_pipeline}' (return → {return_signal})"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Centralized pause / ready handlers
// ---------------------------------------------------------------------------

/// Centralized pause handler.  Called before dispatching any task.
///
/// When `task.pause` is true the handler atomically:
///   1. pushes `(pipeline, signal)` onto the task stack,
///   2. sets state to `State::Pause`,
///   3. clears signal and pause flag.
///
/// Returns `Ok(true)` when the task was paused (caller should skip it).
async fn apply_pause_to_state(zbobr: &Arc<ZbobrDispatcher>, task: &Task) -> anyhow::Result<bool> {
    if !task.pause {
        return Ok(false);
    }

    if task.stack.len() > 100 {
        anyhow::bail!(
            "Task #{}: stack overflow (depth {})",
            task.id,
            task.stack.len()
        );
    }

    let pipeline = match task.state.pipeline() {
        Some(p) => p.clone(),
        None => {
            tracing::warn!(
                "Task #{}: pause flag set but state is '{}', expected Pending — using default pipeline",
                task.id,
                task.state
            );
            zbobr.workflow().default_pipeline()
        }
    };

    let signal = match &task.signal {
        Some(s) => s.clone(),
        None => {
            tracing::warn!(
                "Task #{}: pause flag set but no signal — defaulting to pipeline start",
                task.id
            );
            let first_stage = zbobr
                .workflow()
                .start_stage_for_pipeline(&pipeline)
                .map(|(name, _)| name.to_string())
                .unwrap_or_default();
            Signal::go(first_stage)
        }
    };

    let task_session = zbobr.task_session(task.id);
    task_session
        .modify_task(move |mut t| {
            t.stack.push(StackEntry {
                pipeline,
                signal,
                pipeline_run_id: t.pipeline_run_id,
            });
            t.state = State::Pause;
            t.signal = None;
            t.pause = false;
            t
        })
        .await?;

    tracing::info!("Task #{}: pause applied — state set to PAUSE", task.id);
    Ok(true)
}

/// Handle READY state by popping resume context from the stack.
///
/// If the task is READY with a non-empty stack, pops the top entry and
/// sets state to `Pending(pipeline)` with the saved signal.
///
/// If the stack is empty, returns `false` — the existing
/// `resolve_next_action` handles READY with empty stack by starting
/// from the default pipeline's first stage.
///
/// Returns `Ok(true)` when the task state was updated.
async fn apply_ready_from_state(zbobr: &Arc<ZbobrDispatcher>, task: &Task) -> anyhow::Result<bool> {
    if !task.state.is_ready() {
        return Ok(false);
    }
    if task.stack.is_empty() {
        return Ok(false);
    }

    let task_session = zbobr.task_session(task.id);
    let entry = task_session.pop_stack().await?;

    if let Some(entry) = entry {
        task_session.set_signal(Some(entry.signal.clone())).await?;
        task_session
            .set_state(State::pending(entry.pipeline.clone()))
            .await?;
        tracing::info!(
            "Task #{}: READY with stack — restored pipeline '{}' signal '{}'",
            task.id,
            entry.pipeline,
            entry.signal
        );
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Process a task according to its current state and signal (single-step).
pub async fn process_task(
    zbobr: &Arc<ZbobrDispatcher>,
    task: &Task,
    mcp_tester_override: Option<&ZbobrExecutorMcpTesterConfig>,
) -> anyhow::Result<()> {
    if task.state.is_done() {
        tracing::info!("Task #{} is DONE — nothing to process", task.id);
        return Ok(());
    }

    // Centralized pause handler: convert pause flag → PAUSE state + stack push
    if apply_pause_to_state(zbobr, task).await? {
        tracing::info!("Task #{} paused — state set to PAUSE", task.id);
        return Ok(());
    }

    if task.state.is_pause() {
        tracing::info!("Task #{} is paused — skipped", task.id);
        return Ok(());
    }

    // Handle READY with stack (resume from pause)
    let task = if apply_ready_from_state(zbobr, task).await? {
        zbobr
            .task_backend()
            .get_task(task.id)
            .await?
            .snapshot(false)
            .await?
    } else {
        task.clone()
    };

    // Use state machine to determine what to do
    let action = zbobr.workflow().resolve_next_action(&task)?;
    match action {
        crate::workflow::StateAction::RunStage(pipeline_name, stage_name, stage_def) => {
            if let Some(call_target) = stage_def.call_pipeline() {
                tracing::info!(
                    "Task #{}: entering call stage {}/{} → pipeline '{}'",
                    task.id,
                    pipeline_name,
                    stage_name,
                    call_target
                );
                handle_call_stage(zbobr, task.id, pipeline_name, stage_name, call_target).await?;
            } else {
                tracing::info!(
                    "Task #{}: running stage {}/{} (role={:?})",
                    task.id,
                    pipeline_name,
                    stage_name,
                    stage_def.role_name()
                );
                let runner = CliStageRunner::new(
                    zbobr,
                    task.id,
                    pipeline_name,
                    stage_name,
                    stage_def,
                    mcp_tester_override,
                );
                runner.run().await?;
            }
        }
        crate::workflow::StateAction::Done => {
            let task_session = zbobr.task_session(task.id);
            let is_failure = task.signal.as_ref() == Some(&Signal::ReturnFailure);
            if is_failure {
                // Pipeline failed — return to caller or pause at root
                if let Some(entry) = task_session.pop_stack().await? {
                    task_session.set_signal(Some(Signal::ReturnFailure)).await?;
                    task_session
                        .set_state(State::pending(entry.pipeline.clone()))
                        .await?;
                    tracing::info!(
                        "Task #{}: pipeline failed — returning failure to pipeline '{}'",
                        task.id,
                        entry.pipeline
                    );
                } else {
                    let pipeline = task
                        .state
                        .pipeline()
                        .cloned()
                        .unwrap_or_else(|| zbobr.workflow().default_pipeline());
                    let first_stage = zbobr
                        .workflow()
                        .start_stage_for_pipeline(&pipeline)
                        .map(|(name, _)| name.to_string())
                        .unwrap_or_default();
                    task_session
                        .modify_task(move |mut t| {
                            t.pause = true;
                            t.signal = Some(Signal::go(first_stage));
                            t
                        })
                        .await?;
                    tracing::info!("Task #{}: pipeline failed at root — paused", task.id);
                }
            } else if let Some(entry) = task_session.pop_stack().await? {
                // Success return from sub-pipeline — re-run calling stage
                task_session.set_signal(Some(entry.signal.clone())).await?;
                task_session
                    .set_state(State::pending(entry.pipeline.clone()))
                    .await?;
                tracing::info!(
                    "Task #{}: returning to pipeline '{}' with signal '{}'",
                    task.id,
                    entry.pipeline,
                    entry.signal
                );
            } else {
                task_session.finish().await?;
                tracing::info!("Task #{}: completed", task.id);
            }
        }
        crate::workflow::StateAction::Paused => {
            tracing::info!("Task #{}: paused — skipped", task.id);
        }
        crate::workflow::StateAction::Idle => {
            tracing::info!(
                "Task #{}: idle (state={}, signal={:?}) — skipped",
                task.id,
                task.state,
                task.signal
            );
        }
    }
    Ok(())
}

/// Main manager loop: polls for tasks and dispatches role sessions.
pub async fn run_manager_loop(
    zbobr: &Arc<ZbobrDispatcher>,
    interval_secs: u64,
    cleanup_interval_secs: u64,
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
    let workflow = zbobr.workflow();
    for (pipeline_name, stage_name, stage_def) in workflow.all_stages() {
        if let Some(target) = stage_def.call_pipeline() {
            tracing::info!("Stage {}/{}: call={}", pipeline_name, stage_name, target,);
        } else {
            let tool = zbobr
                .config()
                .tool_for_stage(stage_def, zbobr.workflow().config());
            let model = zbobr
                .config()
                .model_for_stage(stage_def, zbobr.workflow().config());
            tracing::info!(
                "Stage {}/{}: role={:?}, tool={:?}, model={:?}, prompts={:?}",
                pipeline_name,
                stage_name,
                stage_def.role_name().unwrap_or("<none>"),
                tool,
                model,
                stage_def.role_prompt
            );
        }
    }

    let mut last_cleanup = std::time::Instant::now();

    loop {
        let loop_start = std::time::Instant::now();

        if last_cleanup.elapsed().as_secs() >= cleanup_interval_secs {
            tracing::info!("Running workspaces cleanup...");
            if let Err(e) = zbobr.cleanup_closed_tasks(false).await {
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
            match w.snapshot(false).await {
                Ok(t) => all_tasks.push(t),
                Err(e) => tracing::warn!("Failed to snapshot task: {e}"),
            }
        }

        let mut session_run = false;
        for task in &all_tasks {
            if task.state.is_done() {
                continue;
            }

            // Centralized pause handler: convert pause flag → PAUSE state + stack push
            match apply_pause_to_state(zbobr, task).await {
                Ok(true) => {
                    tracing::info!("Task #{} paused — state set to PAUSE", task.id);
                    continue;
                }
                Err(e) => {
                    tracing::error!("Failed to apply pause for task #{}: {e}", task.id);
                    continue;
                }
                _ => {}
            }

            if task.state.is_pause() {
                continue;
            }

            // Handle READY with stack (resume from pause)
            match apply_ready_from_state(zbobr, task).await {
                Ok(true) => continue, // will be processed next poll cycle
                Err(e) => {
                    tracing::error!(
                        "Failed to apply ready-from-state for task #{}: {e}",
                        task.id
                    );
                    continue;
                }
                _ => {}
            }

            let action = match workflow.resolve_next_action(task) {
                Ok(a) => a,
                Err(e) => {
                    tracing::warn!("State machine error for task #{}: {e}", task.id);
                    continue;
                }
            };

            match action {
                crate::workflow::StateAction::RunStage(pipeline_name, stage_name, stage_def) => {
                    tracing::info!(
                        "Processing task #{} (state={}, signal={:?}) — running stage {}/{}",
                        task.id,
                        task.state,
                        task.signal,
                        pipeline_name,
                        stage_name,
                    );
                    if let Some(call_target) = stage_def.call_pipeline() {
                        if let Err(e) = handle_call_stage(
                            zbobr,
                            task.id,
                            pipeline_name,
                            stage_name,
                            call_target,
                        )
                        .await
                        {
                            tracing::error!(
                                "Call stage {}/{} failed for task #{}: {e}",
                                pipeline_name,
                                stage_name,
                                task.id
                            );
                        }
                        // Don't break — call stages are instant, continue processing
                        continue;
                    }
                    let runner = CliStageRunner::new(
                        zbobr,
                        task.id,
                        pipeline_name,
                        stage_name,
                        stage_def,
                        None,
                    );
                    if let Err(e) = runner.run().await {
                        let msg = format!(
                            "Stage {}/{} failed for task #{}: {e}",
                            pipeline_name, stage_name, task.id
                        );
                        tracing::error!("{msg}");
                        let hostname = get_hostname();
                        if let Err(post_err) = zbobr
                            .task_session(task.id)
                            .post_comment("error", &hostname, None, None, &msg, "", 0, None, None)
                            .await
                        {
                            tracing::warn!("Failed to post error to task discussion: {post_err}");
                        }
                    }
                    session_run = true;
                    break;
                }
                crate::workflow::StateAction::Done => {
                    let task_session = zbobr.task_session(task.id);
                    let is_failure = task.signal.as_ref() == Some(&Signal::ReturnFailure);
                    if is_failure {
                        // Pipeline failed — return to caller or pause at root
                        match task_session.pop_stack().await {
                            Ok(Some(entry)) => {
                                tracing::info!(
                                    "Task #{}: pipeline failed — returning failure to pipeline '{}'",
                                    task.id,
                                    entry.pipeline
                                );
                                if let Err(e) =
                                    task_session.set_signal(Some(Signal::ReturnFailure)).await
                                {
                                    tracing::error!(
                                        "Failed to set return_failure signal for task #{}: {e}",
                                        task.id
                                    );
                                }
                                if let Err(e) = task_session
                                    .set_state(State::pending(entry.pipeline.clone()))
                                    .await
                                {
                                    tracing::error!(
                                        "Failed to set return state for task #{}: {e}",
                                        task.id
                                    );
                                }
                            }
                            Ok(None) => {
                                tracing::info!(
                                    "Task #{}: pipeline failed at root — paused",
                                    task.id
                                );
                                let pipeline = task
                                    .state
                                    .pipeline()
                                    .cloned()
                                    .unwrap_or_else(|| zbobr.workflow().default_pipeline());
                                let first_stage = zbobr
                                    .workflow()
                                    .start_stage_for_pipeline(&pipeline)
                                    .map(|(name, _)| name.to_string())
                                    .unwrap_or_default();
                                if let Err(e) = task_session
                                    .modify_task(move |mut t| {
                                        t.pause = true;
                                        t.signal = Some(Signal::go(first_stage));
                                        t
                                    })
                                    .await
                                {
                                    tracing::error!("Failed to pause task #{}: {e}", task.id);
                                }
                            }
                            Err(e) => {
                                tracing::error!("Failed to pop stack for task #{}: {e}", task.id)
                            }
                        }
                    } else {
                        match task_session.pop_stack().await {
                            Ok(Some(entry)) => {
                                // Success return from sub-pipeline — re-run calling stage
                                tracing::info!(
                                    "Task #{}: returning to pipeline '{}' with signal '{}'",
                                    task.id,
                                    entry.pipeline,
                                    entry.signal
                                );
                                if let Err(e) =
                                    task_session.set_signal(Some(entry.signal.clone())).await
                                {
                                    tracing::error!(
                                        "Failed to set return signal for task #{}: {e}",
                                        task.id
                                    );
                                }
                                if let Err(e) = task_session
                                    .set_state(State::pending(entry.pipeline.clone()))
                                    .await
                                {
                                    tracing::error!(
                                        "Failed to set return state for task #{}: {e}",
                                        task.id
                                    );
                                }
                            }
                            Ok(None) => {
                                tracing::info!("Task #{}: completed", task.id);
                                if let Err(e) = task_session.finish().await {
                                    tracing::error!("Failed to finish task #{}: {e}", task.id);
                                }
                            }
                            Err(e) => {
                                tracing::error!("Failed to pop stack for task #{}: {e}", task.id);
                            }
                        }
                    }
                }
                crate::workflow::StateAction::Paused | crate::workflow::StateAction::Idle => {}
            }
        }

        if session_run {
            continue;
        }

        // Task statistics
        let mut state_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for task in &all_tasks {
            *state_counts.entry(task.state.to_string()).or_default() += 1;
        }
        let stats: Vec<String> = state_counts
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect();
        tracing::info!("Task statistics: {}", stats.join(", "));

        let elapsed = loop_start.elapsed();
        let interval_dur = std::time::Duration::from_secs(interval_secs);
        let min_idle_sleep = std::time::Duration::from_secs(1);
        let sleep_dur = interval_dur.saturating_sub(elapsed).max(min_idle_sleep);

        tracing::info!("No processable tasks. Sleeping {}s...", sleep_dur.as_secs());
        tokio::select! {
            _ = tokio::time::sleep(sleep_dur) => {}
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("Received shutdown signal, exiting...");
                break;
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
/// worktree, and attempts to merge upstream. If identity is undefined, returns
/// the task directory as the working directory (prompt template validation will
/// catch missing placeholders). If a merge conflict is detected, dispatches to
/// the configured merge handler.
async fn detect_and_handle_worktree(
    zbobr: &Arc<ZbobrDispatcher>,
    task_id: u64,
    pipeline_name: &Pipeline,
    stage_name: &str,
    task_dir: &Path,
) -> anyhow::Result<WorktreeResult> {
    let task_backend = zbobr.task_backend();
    let task = task_backend
        .get_task(task_id)
        .await?
        .snapshot(false)
        .await?;

    // 1. Check if identity is defined
    let identity = match task.identity() {
        Some(id) => id,
        None => {
            // Identity not yet configured — proceed with task_dir.
            // If the stage's prompt uses {destination_branch} or {work_branch},
            // template rendering will catch the error before the agent starts.
            return Ok(WorktreeResult::Ready(task_dir.to_path_buf()));
        }
    };

    // 2. Update worktree
    let is_uptodate = match zbobr.update_worktree(&identity).await {
        Ok(up) => up,
        Err(e) => {
            let msg = format!("Failed to prepare workspace for task #{task_id}: {e:#}");
            tracing::error!("{msg}");
            let hostname = get_hostname();
            if let Err(post_err) = zbobr
                .task_session(task_id)
                .post_comment("error", &hostname, None, None, &msg, "", 0, None, None)
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
    let is_conflict_handler = pipeline_name.as_str() == Pipeline::MERGE;

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

    handle_merge_conflict(zbobr, task_id, pipeline_name, stage_name).await
}

/// Dispatch to the merge conflict handler pipeline.
///
/// Pushes the current stage onto the stack and calls the merge pipeline.
/// If already inside the merge pipeline, pauses the task.
async fn handle_merge_conflict(
    zbobr: &Arc<ZbobrDispatcher>,
    task_id: u64,
    pipeline_name: &Pipeline,
    stage_name: &str,
) -> anyhow::Result<WorktreeResult> {
    let task_session = zbobr.task_session(task_id);
    let pending_state = State::pending(pipeline_name.clone());

    // Recursion guard: if already inside the merge pipeline, pause
    if pipeline_name.as_str() == Pipeline::MERGE {
        tracing::error!("Task #{task_id}: merge conflict inside merge pipeline — pausing");
        let hostname = get_hostname();
        let msg = "Merge conflict inside merge pipeline. Manual intervention required.".to_string();
        task_session
            .post_comment("error", &hostname, None, None, &msg, "", 0, None, None)
            .await
            .ok();
        let stage = stage_name.to_string();
        task_session
            .modify_task(move |mut t| {
                t.pause = true;
                t.signal = Some(Signal::go(stage));
                t
            })
            .await?;
        task_session.set_state(pending_state.clone()).await?;
        return Ok(WorktreeResult::Paused);
    }

    // Push stack: re-run the interrupted stage upon return
    task_session
        .push_stack(pipeline_name.clone(), Signal::go(stage_name))
        .await?;
    task_session.allocate_pipeline_run_id().await?;
    task_session
        .set_signal(Some(Signal::call(Pipeline::MERGE)))
        .await?;
    task_session.set_state(pending_state).await?;

    tracing::info!("Task #{task_id}: merge conflict — calling merge pipeline");
    Ok(WorktreeResult::HandlerCalled)
}

async fn ensure_pr_url(zbobr: &Arc<ZbobrDispatcher>, task_id: u64) -> anyhow::Result<()> {
    let role_session = zbobr.role_session(task_id);
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
    match zbobr.repo_backend().update_pr(&identity).await {
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
            let task_session = zbobr.task_session(task_id);
            if let Err(post_err) = task_session
                .post_comment("error", &hostname, None, None, &msg, "", 0, None, None)
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

async fn start_mcp_server(
    zbobr: Arc<ZbobrDispatcher>,
    role_name: &str,
    task_id: u64,
    tool: Tool,
    model: Model,
    stage_name: String,
    allowed_tools: std::collections::HashSet<McpTool>,
    tool_tracker: Arc<std::sync::Mutex<Option<String>>>,
    pipeline_name: String,
    pipeline_run_id: u64,
    prompt_holder: Arc<std::sync::Mutex<Option<String>>>,
) -> anyhow::Result<(u16, tokio::task::JoinHandle<()>)> {
    let (port_tx, port_rx) = tokio::sync::oneshot::channel();
    let role_name = role_name.to_string();
    let server_handle = tokio::spawn(async move {
        match crate::mcp::run_role_mcp_server(
            zbobr,
            &role_name,
            task_id,
            tool,
            model,
            stage_name,
            allowed_tools,
            tool_tracker,
            pipeline_name,
            pipeline_run_id,
            prompt_holder,
        )
        .await
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
    zbobr: &Arc<ZbobrDispatcher>,
    task_id: u64,
    pipeline_name: &Pipeline,
    stage_name: &str,
    work_dir: &Path,
    outcome: SessionOutcome,
    last_mapped_tool: Option<&str>,
) -> anyhow::Result<Option<anyhow::Error>> {
    let task_session = zbobr.task_session(task_id);
    let pending_state = State::pending(pipeline_name.clone());

    if outcome.execution_interrupted {
        if let Err(e) = perform_stash_and_push(zbobr, task_id, work_dir, stage_name).await {
            tracing::warn!("Stash/push failed during interruption for task #{task_id}: {e}");
        }
        task_session.set_state(pending_state.clone()).await?;
        tracing::info!("Session interrupted for task #{task_id}, moved to {pending_state}");
        return Ok(None);
    }

    if let Some(e) = outcome.execution_error.as_ref() {
        if let Err(e) = perform_stash_and_push(zbobr, task_id, work_dir, stage_name).await {
            tracing::warn!("Stash/push failed during error handling for task #{task_id}: {e}");
        }
        let error_msg = format!("Execution failed: {e}");
        let hostname = get_hostname();
        if let Err(post_err) = task_session
            .post_comment(
                "error", &hostname, None, None, &error_msg, "", 0, None, None,
            )
            .await
        {
            tracing::error!("Failed to post error to task #{task_id}: {post_err}");
        }
        let stage = stage_name.to_string();
        if let Err(pause_err) = task_session
            .modify_task(move |mut task| {
                task.pause = true;
                task.signal = Some(Signal::go(stage));
                task
            })
            .await
        {
            tracing::error!("Failed to set pause for task #{task_id}: {pause_err}");
        }
        task_session.set_state(pending_state.clone()).await?;
        tracing::info!("Session failed for task #{task_id}, moved to {pending_state} with pause");
        return Ok(outcome.execution_error);
    }

    tracing::info!("Session complete for task #{task_id}");

    if let Err(e) = perform_stash_and_push(zbobr, task_id, work_dir, stage_name).await {
        tracing::error!("Stash/push failed for task #{task_id}: {e}");
        let hostname = get_hostname();
        let msg = format!("Stash/push failed: {e}");
        if let Err(post_err) = task_session
            .post_comment("error", &hostname, None, None, &msg, "", 0, None, None)
            .await
        {
            tracing::error!("Failed to post stash/push error for task #{task_id}: {post_err}");
        }
        let stage = stage_name.to_string();
        if let Err(pause_err) = task_session
            .modify_task(move |mut task| {
                task.pause = true;
                task.signal = Some(Signal::go(stage));
                task
            })
            .await
        {
            tracing::error!(
                "Failed to pause task #{task_id} after stash/push failure: {pause_err}"
            );
        }
        task_session.set_state(pending_state.clone()).await?;
        return Ok(None);
    }

    // Compute post-stage signal using the sequential pipeline model.
    // If the agent already set a signal during the session (e.g. stop_with_error),
    // that signal takes priority.
    let current_task = zbobr
        .task_backend()
        .get_task(task_id)
        .await?
        .snapshot(false)
        .await?;
    if !current_task.pause && current_task.signal.is_none() {
        let stage_def = zbobr.workflow().stage(pipeline_name, stage_name);
        let seq_signal = compute_sequential_signal(
            pipeline_name,
            stage_name,
            stage_def,
            zbobr.workflow(),
            last_mapped_tool,
        );
        match seq_signal {
            SequentialSignal::ReturnFailure => {
                task_session.set_signal(Some(Signal::ReturnFailure)).await?;
            }
            SequentialSignal::Advance(next) => {
                task_session.set_signal(Some(Signal::go(next))).await?;
            }
            SequentialSignal::Return => {
                task_session.set_signal(Some(Signal::Return)).await?;
            }
            SequentialSignal::Pause => {
                let stage = stage_name.to_string();
                task_session
                    .modify_task(move |mut t| {
                        t.pause = true;
                        t.signal = Some(Signal::go(stage));
                        t
                    })
                    .await?;
            }
            SequentialSignal::PauseThenSignal(signal) => {
                task_session
                    .modify_task(move |mut t| {
                        t.pause = true;
                        t.signal = Some(signal);
                        t
                    })
                    .await?;
            }
        }
    }
    // If pause was set by MCP tool (e.g. stop_with_error) but no signal, set
    // signal to re-run the current stage on resume.
    if current_task.pause && current_task.signal.is_none() {
        task_session
            .set_signal(Some(Signal::go(stage_name)))
            .await?;
    }
    task_session.set_state(pending_state).await?;

    Ok(None)
}

async fn perform_stash_and_push(
    zbobr: &Arc<ZbobrDispatcher>,
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
                match git(
                    work_dir,
                    &["stash", "push", "--include-untracked", "-m", &stash_msg],
                )
                .await
                {
                    Ok(_) => tracing::info!("Git stash successful"),
                    Err(e) => tracing::warn!("Git stash failed: {e}"),
                }
            } else {
                tracing::info!("No uncommitted changes found");
            }
        }
        Err(e) => tracing::warn!("Failed to check git status for stash: {e}"),
    }

    let task = task_backend
        .get_task(task_id)
        .await?
        .snapshot(false)
        .await?;
    if let Some(identity) = task.identity() {
        if let Err(e) = repo_backend.update_pr(&identity).await {
            tracing::warn!("Could not push branch commits for task #{task_id}: {e}");
        }
        let config = zbobr.config();
        if config.overwrite_author {
            let dest_branch = identity.destination_branch.clone();
            zbobr_utility::rewrite_authors_on_worktree(
                work_dir,
                &dest_branch,
                &config.git_user_name,
                &config.git_user_email,
            )
            .await?;
            // Push rewritten commits
            if let Err(e) = repo_backend.update_pr(&identity).await {
                tracing::warn!("Could not push rewritten commits for task #{task_id}: {e}");
            }
        }
    } else {
        tracing::warn!("Task #{task_id} missing routing parameters — skipping push");
    }

    Ok(())
}
