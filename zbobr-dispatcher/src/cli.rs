#![allow(clippy::needless_borrows_for_generic_args)]

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Context;
use clap::{Args, Parser};
use zbobr_api::{
    Pipeline, Signal, StackEntry, State,
    config::StageDefinition,
    config_tools::McpTool,
    task::{Stage, StageContext, StageInfo},
};
use zbobr_executor_mcp_tester::ZbobrExecutorMcpTesterConfig;
// bring in the generic git helpers from utility crate
use zbobr_utility::{git, git_check, git_output};

use crate::{
    Comment, Task, TaskDir, ToolExecutor, Workflow, ZbobrDispatcher,
    task::{Model, Executor},
    workflow::SequentialSignal,
};
use zbobr_api::tool_executor::ExecutorOutput;

// ---------------------------------------------------------------------------
// CLI types
// ---------------------------------------------------------------------------

/// Configuration file path argument.
///
/// Multiple config files can be specified with repeated `-c` / `--config` flags.
/// When one or more configs are given, the default `zbobr.toml` is ignored.
/// Configs are applied in order: later files override earlier ones.
#[derive(Args, Clone)]
pub struct ConfigFileArg {
    /// Path to TOML configuration file (repeatable; later files override earlier ones)
    #[arg(short = 'c', long = "config")]
    pub paths: Vec<PathBuf>,
}

/// Resolved config file location.
pub struct ConfigLocation {
    /// Config file paths to load (in order). May contain a single default path
    /// that doesn't necessarily exist on disk.
    pub config_paths: Vec<PathBuf>,
    pub config_dir: PathBuf,
}

/// Resolve config file paths and the base directory for relative path resolution.
///
/// When `cli_paths` is non-empty, each file must exist and `config_dir` is
/// derived from the **last** file's parent directory.
/// When empty, `default_config_name` in the current directory is used and
/// `config_dir` is `std::env::current_dir()`.
pub fn resolve_config_location(
    cli_paths: &[PathBuf],
    default_config_name: &str,
) -> anyhow::Result<ConfigLocation> {
    if cli_paths.is_empty() {
        let config_dir = std::env::current_dir()?;
        let config_paths = vec![config_dir.join(default_config_name)];
        return Ok(ConfigLocation {
            config_paths,
            config_dir,
        });
    }

    let mut config_paths = Vec::with_capacity(cli_paths.len());
    let mut config_dir = std::env::current_dir()?;

    for path in cli_paths {
        let canonical = std::fs::canonicalize(path)
            .with_context(|| format!("Cannot resolve config path: {}", path.display()))?;
        config_dir = canonical
            .parent()
            .expect("config file must have a parent directory")
            .to_path_buf();
        config_paths.push(canonical);
    }

    Ok(ConfigLocation {
        config_paths,
        config_dir,
    })
}

/// Global arguments that should be hoisted before subcommands.
/// This includes only dispatcher and executor config, not backend-specific settings.
#[derive(Args, Clone)]
pub struct GlobalArgs {
    /// Enable log output to stderr
    #[arg(long)]
    pub logs: bool,

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
        .flat_map(|a| {
            let takes_value = !matches!(
                a.get_action(),
                clap::ArgAction::SetTrue | clap::ArgAction::SetFalse | clap::ArgAction::Count
            );
            let long_entry = a.get_long().map(|long| (format!("--{long}"), takes_value));
            let short_entry = a
                .get_short()
                .map(|short| (format!("-{short}"), takes_value));
            long_entry.into_iter().chain(short_entry)
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
        // Check for exact match (e.g. `-c`, `--config`, `--config=val`)
        let matched = global_flags.get(base).copied();
        // Also check for attached short-value form (e.g. `-cshared.toml`):
        // a short flag like `-c` that takes a value may appear as `-c<value>`
        // without a `=` separator.
        let matched = matched.or_else(|| {
            if arg.starts_with('-') && !arg.starts_with("--") && arg.len() > 2 {
                let short_key = &arg[..2]; // e.g. "-c"
                global_flags
                    .get(short_key)
                    .copied()
                    .filter(|&takes_value| takes_value)
            } else {
                None
            }
        });
        if let Some(takes_value) = matched {
            if arg.contains('=')
                || (arg.starts_with('-') && !arg.starts_with("--") && arg.len() > 2)
            {
                // Attached value: --config=val or -cval
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
// Branch name helpers
// ---------------------------------------------------------------------------

/// Sanitize a task title into a valid git branch postfix.
/// Lowercases, replaces non-alphanumeric characters with '-', collapses consecutive
/// dashes, and trims leading/trailing dashes. Truncates to 50 characters.
fn sanitize_branch_postfix(title: &str) -> String {
    let sanitized: String = title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();

    let mut result = String::new();
    let mut last_dash = true; // treat start as dash to trim leading dashes
    for c in sanitized.chars() {
        if c == '-' {
            if !last_dash {
                result.push(c);
                last_dash = true;
            }
        } else {
            result.push(c);
            last_dash = false;
        }
    }
    // trim trailing dash
    let result = result.trim_end_matches('-').to_string();
    // truncate to 50 chars (char-based to avoid panicking on multi-byte Unicode)
    if result.chars().count() > 50 {
        result
            .chars()
            .take(50)
            .collect::<String>()
            .trim_end_matches('-')
            .to_string()
    } else {
        result
    }
}

/// Ensure the task has a work_branch set. If not, derive one from the task title.
async fn ensure_work_branch(zbobr: &Arc<ZbobrDispatcher>, task_id: u64) -> anyhow::Result<()> {
    let task = zbobr
        .task_backend()
        .get_task(task_id)
        .await?
        .snapshot(false)
        .await?;

    if task.work_branch.is_some() {
        return Ok(());
    }

    let postfix = sanitize_branch_postfix(&task.title);
    let postfix = if postfix.is_empty() {
        "task".to_string()
    } else {
        postfix
    };

    let prefix = &zbobr.config().work_branch_prefix;
    let work_branch = format!("{}-{}-{}", prefix, task_id, postfix);

    tracing::info!(
        "Task #{task_id}: auto-deriving work branch '{}' from title '{}'",
        work_branch,
        task.title
    );

    let weak = zbobr.task_backend().get_task(task_id).await?;
    let mutable = weak.upgrade().await?;
    mutable
        .modify_task(Box::new(move |mut task| {
            task.work_branch = Some(work_branch);
            task
        }))
        .await?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Task list entry
// ---------------------------------------------------------------------------

/// Compact projection of a [`Task`] for one-line list display and JSON output.
#[derive(Debug, serde::Serialize)]
pub struct TaskListEntry {
    pub id: u64,
    pub stage_count: u64,
    pub state: State,
    pub title: String,
}

impl From<&Task> for TaskListEntry {
    fn from(task: &Task) -> Self {
        Self {
            id: task.id,
            stage_count: task.stage_count,
            state: task.state.clone(),
            title: task.title.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// Ready-task selection
// ---------------------------------------------------------------------------

/// Priority key for task scheduling: higher value → processed first.
///
/// This is the single source of truth for task priority ordering used by both
/// [`select_runnable_task`] and [`run_manager_loop`].
fn task_priority(task: &Task) -> u64 {
    task.stage_count
}

/// Return the highest-priority task that the workflow is ready to run (non-call [`StateAction::RunStage`]).
///
/// Uses full workflow resolution via [`Workflow::resolve_next_action`] so that the predicate
/// matches exactly the tasks that [`run_manager_loop`] would schedule in Phase 2.
///
/// Tasks in `READY` state with a non-empty stack are excluded because the loop normalises them
/// via `apply_ready_from_state` in Phase 1 and defers them to the next cycle — they are never
/// present in Phase 2 `runstage_candidates`.  Calling `resolve_next_action` on such tasks
/// would use the wrong pipeline (default instead of the saved stack pipeline).
///
/// Returns `None` if no runnable task exists.
pub fn select_runnable_task<'a>(workflow: &Workflow, tasks: &'a [Task]) -> Option<&'a Task> {
    tasks
        .iter()
        .filter(|t| {
            // READY-with-stack tasks are normalised in loop Phase 1 and deferred; skip them.
            let ready_with_stack = t.state.is_ready() && !t.stack.is_empty();
            !t.pause
                && !ready_with_stack
                && matches!(
                    workflow.resolve_next_action(t),
                    Ok(crate::workflow::StateAction::RunStage(_, _, def))
                        if def.call_pipeline().is_none()
                )
        })
        .max_by(|a, b| {
            task_priority(a)
                .cmp(&task_priority(b))
                .then_with(|| b.id.cmp(&a.id))
        })
}

// ---------------------------------------------------------------------------
// Task display
// ---------------------------------------------------------------------------

/// Print a task to stdout in a human-readable format.
pub fn print_task(task: &Task, discussion: &[Comment]) {
    println!("ID:          {}", task.id);
    println!("Title:       {}", task.title);
    println!("State:       {:?}", task.state);
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
    if let Some(ref branch) = task.work_branch {
        println!("Work Branch: {}", branch);
    }
    if let Some(ref url) = task.pr_url {
        println!("PR URL: {}", url);
    }
    if !task.description.is_empty() {
        println!("Description:\n{}", task.description);
    }
    if !discussion.is_empty() {
        println!("Discussion ({} comment(s)):", discussion.len());
        for (i, c) in discussion.iter().enumerate() {
            println!("  [{}] {}\n{}", i + 1, c.username, c.body);
        }
    }
}

// ---------------------------------------------------------------------------
// CliStageRunner — stage execution
// ---------------------------------------------------------------------------

struct CliStageRunner<'a> {
    zbobr: &'a Arc<ZbobrDispatcher>,
    task_id: u64,
    pipeline: &'a Pipeline,
    stage: &'a Stage,
    stage_def: &'a StageDefinition,
    mcp_tester_override: Option<&'a ZbobrExecutorMcpTesterConfig>,
}

impl<'a> CliStageRunner<'a> {
    fn new(
        zbobr: &'a Arc<ZbobrDispatcher>,
        task_id: u64,
        pipeline: &'a Pipeline,
        stage: &'a Stage,
        stage_def: &'a StageDefinition,
        mcp_tester_override: Option<&'a ZbobrExecutorMcpTesterConfig>,
    ) -> Self {
        Self {
            zbobr,
            task_id,
            pipeline,
            stage,
            stage_def,
            mcp_tester_override,
        }
    }

    fn running_state(&self) -> State {
        State::running(self.pipeline.clone(), self.stage.clone())
    }

    async fn prompt(&self, _pipeline_run_id: u64) -> anyhow::Result<String> {
        self.zbobr
            .prompt_builder()
            .build_for_stage(self.stage_def, self.task_id, self.zbobr.task_backend())
            .await
    }

    async fn run(&self) -> anyhow::Result<()> {
        let role = self
            .stage_def
            .role()
            .expect("role stage must have role");
        let tool = self
            .zbobr
            .config()
            .resolve_tool(self.stage_def, self.zbobr.workflow().config())?;

        // Set state to running
        self.zbobr
            .task_session(self.task_id)
            .set_state(self.running_state())
            .await?;

        let task_dir = TaskDir::new(self.zbobr.config().workspaces.as_path(), self.task_id);
        tokio::fs::create_dir_all(task_dir.path()).await?;

        // Auto-derive work branch if not yet set
        ensure_work_branch(self.zbobr, self.task_id).await?;

        // Unified worktree detection and problem handling
        let work_dir = match detect_and_handle_worktree(
            self.zbobr,
            self.task_id,
            self.pipeline,
            self.stage,
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

        // Auto-pause if stage count limit reached (before incrementing to avoid wasted increment).
        {
            let task_session = self.zbobr.task_session(self.task_id);
            let task = task_session.get_task().await?;
            if task.max_stage_count > 0 && task.stage_count >= task.max_stage_count {
                tracing::warn!(
                    "Task #{}: stage_count ({}) reached max_stage_count ({}) — auto-pausing",
                    self.task_id,
                    task.stage_count,
                    task.max_stage_count
                );
                let status = format_error_status(
                    self.zbobr,
                    &format!(
                        "Stage count limit ({}) reached - auto-paused",
                        task.max_stage_count
                    ),
                );
                task_session.set_pause_with_status(status).await?;
                return Ok(());
            }
        }

        // Increment the stage counter.
        {
            let task_session = self.zbobr.task_session(self.task_id);
            task_session.increment_stage_count().await?;
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
            .and_then(|d| d.mcp.as_ref())
            .map(|tools| tools.iter().copied().collect())
            .unwrap_or_default();

        // Read current pipeline_run_id for this session.
        let task_snap = self
            .zbobr
            .task_backend()
            .get_task(self.task_id)
            .await?
            .snapshot(false)
            .await?;
        let pipeline_run_id = task_snap.pipeline_run_id;

        let prompt_text = self.prompt(pipeline_run_id).await?;

        // Store the prompt once; the link is reused for each provider attempt's StageContext entry.
        let prompt_link = {
            let role_session = self.zbobr.role_session(self.task_id);
            let base_name = format!(
                "prompt_{}_{}_{}_start",
                self.pipeline, pipeline_run_id, self.stage
            );
            role_session.store_report(&base_name, &prompt_text).await?
        };

        let agent_token_owned = self.zbobr.config().agent_github_token.as_ref().to_owned();

        // Provider retry loop: on any failed execution attempt, retry with the next provider/model
        // selected by the existing priority + round-robin logic.
        let mut cycle_excluded_providers: HashSet<String> = HashSet::new();
        loop {
            let (resolved_provider, model) = self
                .zbobr
                .select_provider_excluding(&tool, &cycle_excluded_providers)?;
            let plan_mode = resolved_provider.plan_mode;

            // Add a new StageContext to the task's context for this attempt.
            {
                let instance = self.zbobr.config().instance.clone();
                let pipeline_name = self.pipeline.clone();
                let stage_name = self.stage.clone();
                let tool_val = Some(resolved_provider.provider.as_str().to_string());
                let model_val = Some(model.clone());
                let timestamp =
                    chrono::Utc::now().with_timezone(&self.zbobr.config().fixed_offset());
                let prompt_link_val = Some(prompt_link.clone());
                let role_session = self.zbobr.role_session(self.task_id);
                role_session
                    .modify_task(move |mut task| {
                        task.context.stages.push(StageContext {
                            info: StageInfo {
                                instance,
                                pipeline: pipeline_name,
                                run_id: pipeline_run_id,
                                stage: stage_name,
                                tool: tool_val,
                                model: model_val,
                                prompt_link: prompt_link_val,
                                output_link: None,
                                timestamp,
                            },
                            records: Vec::new(),
                        });
                        task
                    })
                    .await?;
            }

            let tool_tracker = Arc::new(std::sync::Mutex::new(None::<McpTool>));
            let (assigned_port, server_handle) = start_mcp_server(
                Arc::clone(self.zbobr),
                role,
                self.task_id,
                resolved_provider.executor.clone(),
                model.clone(),
                self.stage.to_string(),
                allowed_tools.clone(),
                Arc::clone(&tool_tracker),
                self.pipeline.to_string(),
                pipeline_run_id,
            )
            .await?;

            let mcp_url = format!(
                "http://127.0.0.1:{}/{}/{}",
                assigned_port, role, self.task_id,
            );

            let executor = self
                .zbobr
                .build_executor(&resolved_provider, self.mcp_tester_override)?;
            let copilot_token_owned = if resolved_provider.executor == Executor::copilot() {
                self.zbobr.copilot_github_token().to_owned()
            } else {
                String::new()
            };

            let outcome = execute_tool(
                executor,
                &copilot_token_owned,
                &agent_token_owned,
                self.task_id,
                role,
                model.as_str(),
                assigned_port,
                &prompt_text,
                &work_dir,
                &mcp_url,
                plan_mode,
            )
            .await;

            // Store the captured output and link it to the stage context entry.
            if let Some(ref output) = outcome.execution_output {
                let role_session = self.zbobr.role_session(self.task_id);
                let base_name = format!(
                    "output_{}_{}_{}_end",
                    self.pipeline, pipeline_run_id, self.stage
                );
                match role_session.store_report(&base_name, output).await {
                    Ok(output_link) => {
                        if let Err(e) = role_session
                            .modify_task(move |mut task| {
                                if let Some(stage) = task.context.stages.last_mut() {
                                    stage.info.output_link = Some(output_link);
                                }
                                task
                            })
                            .await
                        {
                            tracing::warn!(
                                "Failed to set output_link for task #{}: {e}",
                                self.task_id
                            );
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to store output report for task #{}: {e}",
                            self.task_id
                        );
                    }
                }
            }

            // Read the last mapped tool from the shared tracker.
            let last_mapped_tool = *tool_tracker.lock().unwrap();

            if outcome.execution_failed {
                cycle_excluded_providers.insert(resolved_provider.provider.as_str().to_string());
                let attempts_remaining = self.zbobr.available_provider_model_count_excluding(
                    &tool,
                    &cycle_excluded_providers,
                )?;
                let excluded = self.zbobr.record_provider_failure(resolved_provider.provider.as_str());
                server_handle.abort();
                if attempts_remaining > 0 {
                    let exclusion_hint = if excluded {
                        " (provider temporarily excluded)"
                    } else {
                        ""
                    };
                    tracing::warn!(
                        "Provider '{}' failed for tool '{}' — retrying with next available provider{}",
                        resolved_provider.provider.as_str(),
                        tool,
                        exclusion_hint,
                    );
                    continue;
                }
                tracing::warn!(
                    "Provider/model attempts exhausted for tool '{}' after a full cycle",
                    tool
                );
            } else {
                self.zbobr.record_provider_success(resolved_provider.provider.as_str());
            }

            if let Some(e) = finalize_stage_session(
                self.zbobr,
                self.task_id,
                self.pipeline,
                self.stage,
                &work_dir,
                outcome,
                last_mapped_tool,
            )
            .await?
            {
                server_handle.abort();
                return Err(e);
            }

            server_handle.abort();

            return Ok(());
        }
    }
}

/// Result of computing the post-stage signal in the sequential pipeline model.
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
    stage: &Stage,
    call_pipeline: &Pipeline,
) -> anyhow::Result<()> {
    let task_session = zbobr.task_session(task_id);

    // Determine what signal to emit when the called pipeline returns.
    let stage_def = zbobr.workflow().stage(pipeline_name, stage);
    let return_signal = if let Some(target) = stage_def
        .and_then(|s| s.on_success())
        .and_then(|t| t.next.as_ref())
    {
        Signal::go(target.as_str())
    } else {
        match zbobr
            .workflow()
            .pipeline(pipeline_name)
            .and_then(|p| p.next_stage(stage))
        {
            Some((next, _)) => Signal::go(next.as_str()),
            None => Signal::Return,
        }
    };

    task_session.allocate_pipeline_run_id().await?;
    task_session.increment_stage_count().await?;

    // Auto-pause if stage count limit reached (before push_stack to prevent stack duplication on resume).
    {
        let task = task_session.get_task().await?;
        if task.max_stage_count > 0 && task.stage_count >= task.max_stage_count {
            tracing::warn!(
                "Task #{task_id}: stage_count ({}) reached max_stage_count ({}) — auto-pausing",
                task.stage_count,
                task.max_stage_count
            );
            let status = format_error_status(
                zbobr,
                &format!(
                    "Stage count limit ({}) reached - auto-paused",
                    task.max_stage_count
                ),
            );
            task_session.set_pause_with_status(status).await?;
            return Ok(());
        }
    }

    // Push stack so we return to the right place.
    task_session
        .push_stack(pipeline_name.clone(), return_signal.clone())
        .await?;

    let call_signal = Signal::call(call_pipeline.clone());
    task_session.set_signal(Some(call_signal.clone())).await?;
    task_session
        .set_state(State::pending(pipeline_name.clone()))
        .await?;

    tracing::info!(
        "Task #{task_id}: stage {pipeline_name}/{} calling pipeline '{call_pipeline}' (return → {return_signal})",
        stage.as_str()
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
                "Task #{}: pause flag set but state is '{:?}', expected Pending — using default pipeline",
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
                handle_call_stage(
                    zbobr,
                    task.id,
                    pipeline_name,
                    stage_name,
                    call_target,
                )
                .await?;
            } else {
                tracing::info!(
                    "Task #{}: running stage {}/{} (role={:?})",
                    task.id,
                    pipeline_name,
                    stage_name,
                    stage_def.role()
                );
                let runner = CliStageRunner::new(
                    zbobr,
                    task.id,
                    pipeline_name,
                    stage_name,
                    stage_def,
                    mcp_tester_override,
                );
                if let Err(e) = runner.run().await {
                    let msg = format!(
                        "Stage {}/{} failed for task #{}: {e}",
                        pipeline_name, stage_name, task.id
                    );
                    tracing::error!("{msg}");
                    let task_session = zbobr.task_session(task.id);
                    let status = format_error_status(zbobr, &msg);
                    if let Err(pause_err) = task_session
                        .set_pause_with_status_and_signal(status, Signal::go(stage_name.as_str()))
                        .await
                    {
                        tracing::error!(
                            "Failed to pause task #{} after stage error: {pause_err}",
                            task.id
                        );
                    }
                }
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
                    let status = format_error_status(
                        zbobr,
                        "Pipeline failed at root — manual intervention required",
                    );
                    task_session
                        .set_pause_with_status_and_signal(status, Signal::go(first_stage))
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
                "Task #{}: idle (state={:?}, signal={:?}) — skipped",
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
    tracing::info!(
        "Configured providers: {:?}",
        zbobr.config().providers.keys().collect::<Vec<_>>()
    );
    if let Some(base) = prompt_builder.base_path() {
        tracing::info!("Prompts base path: {}", base.display());
    }

    // Dump stage-specific settings for visibility
    let workflow = zbobr.workflow();
    for (pipeline_name, stage_name, stage_def) in workflow.all_stages() {
        if let Some(target) = stage_def.call_pipeline() {
            tracing::info!("Stage {}/{}: call={}", pipeline_name, stage_name, target,);
        } else {
            let tool_name = zbobr
                .config()
                .resolve_tool(stage_def, zbobr.workflow().config());
            tracing::info!(
                "Stage {}/{}: role={:?}, tool={:?}, prompts={:?}",
                pipeline_name,
                stage_name,
                stage_def.role().map_or("<none>", |r| r.as_str()),
                tool_name,
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
        // Sort by task_priority descending so tasks closest to completion are processed first.
        all_tasks.sort_by_key(|b| std::cmp::Reverse(task_priority(b)));

        let mut session_run = false;

        // Phase 1: apply transitions and handle Done / instant call-stage actions for all
        // tasks eagerly.  Non-call RunStage tasks are collected for priority-based selection
        // in Phase 2 below.
        let mut runstage_candidates: Vec<Task> = Vec::new();
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
                    if let Some(call_target) = stage_def.call_pipeline() {
                        // Call stages are instant — process eagerly without consuming the slot
                        tracing::info!(
                            "Processing task #{} (state={:?}, signal={:?}) — running stage {}/{}",
                            task.id,
                            task.state,
                            task.signal,
                            pipeline_name,
                            stage_name,
                        );
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
                    } else {
                        // Non-call RunStage — collect for priority-based selection in Phase 2
                        runstage_candidates.push(task.clone());
                    }
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
                                let status = format_error_status(
                                    zbobr,
                                    "Pipeline failed at root — manual intervention required",
                                );
                                if let Err(e) = task_session
                                    .set_pause_with_status_and_signal(
                                        status,
                                        Signal::go(first_stage),
                                    )
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

        // Phase 2: use select_runnable_task to pick the highest-priority RunStage candidate
        // and run its stage.  This shares the exact same ready-task selection logic as the
        // `task list --select` CLI flag.
        if let Some(task) = select_runnable_task(workflow, &runstage_candidates) {
            let action = match workflow.resolve_next_action(task) {
                Ok(a) => a,
                Err(e) => {
                    tracing::warn!("State machine error for task #{}: {e}", task.id);
                    continue;
                }
            };
            if let crate::workflow::StateAction::RunStage(pipeline_name, stage_name, stage_def) =
                action
            {
                tracing::info!(
                    "Processing task #{} (state={:?}, signal={:?}) — running stage {}/{}",
                    task.id,
                    task.state,
                    task.signal,
                    pipeline_name,
                    stage_name,
                );
                let runner =
                    CliStageRunner::new(zbobr, task.id, pipeline_name, stage_name, stage_def, None);
                if let Err(e) = runner.run().await {
                    let msg = format!(
                        "Stage {}/{} failed for task #{}: {e}",
                        pipeline_name, stage_name, task.id
                    );
                    tracing::error!("{msg}");
                    let task_session = zbobr.task_session(task.id);
                    let status = format_error_status(zbobr, &msg);
                    if let Err(pause_err) = task_session
                        .set_pause_with_status_and_signal(status, Signal::go(stage_name.as_str()))
                        .await
                    {
                        tracing::error!(
                            "Failed to pause task #{} after stage error: {pause_err}",
                            task.id
                        );
                    }
                }
                session_run = true;
            }
        }

        if session_run {
            continue;
        }

        // Task statistics
        let mut state_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for task in &all_tasks {
            *state_counts.entry(format!("{:?}", task.state)).or_default() += 1;
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
/// Sets up the worktree for the task and attempts to merge upstream.
/// If a merge conflict is detected, dispatches to the configured merge handler.
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

    // 1. Check if identity is defined (work_branch set)
    let identity = match task.identity() {
        Some(id) => id,
        None => {
            // work_branch not set — proceed with task_dir.
            // This should only happen if no stages have run yet.
            return Ok(WorktreeResult::Ready(task_dir.to_path_buf()));
        }
    };

    // 2. Update worktree
    let is_uptodate = match zbobr.update_worktree(&identity).await {
        Ok(up) => up,
        Err(e) => {
            let msg = format!("Failed to prepare workspace for task #{task_id}: {e:#}");
            tracing::error!("{msg}");
            set_task_status_with_log(zbobr, task_id, "workspace preparation", &msg).await;
            return Err(anyhow::anyhow!(msg));
        }
    };

    // 3. Compute work_dir from backend repo name
    let repo_name = zbobr.repo_backend().repo_name();
    let work_dir = TaskDir::new(zbobr.config().workspaces.as_path(), task_id)
        .path()
        .join(repo_name);

    // 4. If up-to-date, no merge needed
    if is_uptodate {
        return Ok(WorktreeResult::Ready(work_dir));
    }

    // 5. Attempt merge with base branch from backend config
    let base_branch = zbobr.repo_backend().branch();

    // If we ARE the conflict handler, start the merge but don't abort on failure —
    // the agent needs to see conflict markers in the working tree.
    let is_conflict_handler = pipeline_name.as_str() == Pipeline::MERGE;

    let merged_ok = git_check(
        &work_dir,
        &["merge", &format!("origin/{}", base_branch), "--no-edit"],
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
        let msg = "Merge conflict inside merge pipeline. Manual intervention required.";
        let status = format_error_status(zbobr, msg);
        let stage = stage_name.to_string();
        task_session
            .set_pause_with_status_and_signal(status, Signal::go(stage))
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

fn format_error_status(zbobr: &ZbobrDispatcher, message: &str) -> String {
    let ts = chrono::Utc::now().with_timezone(&zbobr.config().fixed_offset());
    zbobr_api::format_status(zbobr_api::ERROR_PREFIX, &ts, message)
}

async fn set_task_status_with_log(
    zbobr: &Arc<ZbobrDispatcher>,
    task_id: u64,
    context: &str,
    message: &str,
) {
    let role_session = zbobr.role_session(task_id);
    let status = format_error_status(zbobr, message);
    if let Err(set_err) = role_session.set_status(Some(status)).await {
        tracing::warn!("Failed to set task status for task #{task_id} ({context}): {set_err}");
    }
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
            let msg = format!("Task #{task_id} is missing work_branch — cannot ensure PR URL");
            tracing::error!("{msg}");
            return Err(anyhow::anyhow!(msg));
        }
    };
    let issue_body = zbobr.task_backend().task_repo_name().map(|repo_name| {
        format!(
            "Resolves https://github.com/{}/issues/{}",
            repo_name, task_id
        )
    });
    match zbobr
        .repo_backend()
        .ensure_pr_url(&identity, issue_body.as_deref())
        .await
    {
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
            set_task_status_with_log(zbobr, task_id, "ensure PR URL", &msg).await;
            Err(anyhow::anyhow!(msg))
        }
    }
}

/// Pre-populate task parameters from dispatcher config defaults.
/// Only sets a parameter if it is not already present, so a previously
/// prepared task keeps its values unchanged. Called unconditionally at
/// the start of every stage run.
#[allow(clippy::too_many_arguments)]
async fn start_mcp_server(
    zbobr: Arc<ZbobrDispatcher>,
    role_name: &str,
    task_id: u64,
    tool: Executor,
    model: Model,
    stage_name: String,
    allowed_tools: std::collections::HashSet<McpTool>,
    tool_tracker: Arc<std::sync::Mutex<Option<McpTool>>>,
    pipeline_name: String,
    pipeline_run_id: u64,
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
    execution_output: Option<String>,
    /// True when execution failed and the stage should try another provider/model.
    execution_failed: bool,
}

#[allow(clippy::too_many_arguments)]
async fn execute_tool(
    executor: Box<dyn ToolExecutor>,
    copilot_token: &str,
    agent_token: &str,
    task_id: u64,
    role: &str,
    model: &str,
    assigned_port: u16,
    prompt: &str,
    work_dir: &Path,
    mcp_url: &str,
    plan_mode: bool,
) -> SessionOutcome {
    tokio::select! {
        result = executor.execute(task_id, role, model, assigned_port, prompt, work_dir, mcp_url, plan_mode, agent_token, copilot_token) => {
            match result {
                Ok(ExecutorOutput { output, exit_ok: true, .. }) => SessionOutcome {
                    execution_interrupted: false,
                    execution_error: None,
                    execution_output: Some(output),
                    execution_failed: false,
                },
                Ok(ExecutorOutput { output, exit_ok: false, .. }) => {
                    let e = anyhow::anyhow!("Tool exited with non-zero status");
                    tracing::error!("Tool execution failed: {e}");
                    SessionOutcome {
                        execution_interrupted: false,
                        execution_error: Some(e),
                        execution_output: Some(output),
                        execution_failed: true,
                    }
                }
                Err(e) => {
                    tracing::error!("Tool execution failed: {e}");
                    SessionOutcome {
                        execution_interrupted: false,
                        execution_error: Some(e),
                        execution_output: None,
                        execution_failed: true,
                    }
                }
            }
        }
        _ = tokio::signal::ctrl_c() => {
            tracing::warn!("Received shutdown signal during execution");
            SessionOutcome {
                execution_interrupted: true,
                execution_error: None,
                execution_output: None,
                execution_failed: false,
            }
        }
    }
}

async fn finalize_stage_session(
    zbobr: &Arc<ZbobrDispatcher>,
    task_id: u64,
    pipeline: &Pipeline,
    stage: &Stage,
    work_dir: &Path,
    outcome: SessionOutcome,
    last_mapped_tool: Option<McpTool>,
) -> anyhow::Result<Option<anyhow::Error>> {
    let task_session = zbobr.task_session(task_id);
    let pending_state = State::pending(pipeline.clone());

    if outcome.execution_interrupted {
        if let Err(e) =
            perform_stash_and_push(zbobr, task_id, work_dir, stage.as_str(), pipeline).await
        {
            tracing::warn!("Stash/push failed during interruption for task #{task_id}: {e}");
        }
        task_session.set_state(pending_state.clone()).await?;
        tracing::info!("Session interrupted for task #{task_id}, moved to {pending_state:?}");
        return Ok(None);
    }

    if let Some(e) = outcome.execution_error.as_ref() {
        if let Err(e) =
            perform_stash_and_push(zbobr, task_id, work_dir, stage.as_str(), pipeline).await
        {
            tracing::warn!("Stash/push failed during error handling for task #{task_id}: {e}");
        }
        let error_msg = format!("Execution failed: {e}");
        let status = format_error_status(zbobr, &error_msg);
        let stage = stage.to_string();
        if let Err(pause_err) = task_session
            .set_pause_with_status_and_signal(status, Signal::go(stage.as_str()))
            .await
        {
            tracing::error!("Failed to set pause for task #{task_id}: {pause_err}");
        }
        task_session.set_state(pending_state.clone()).await?;
        tracing::info!("Session failed for task #{task_id}, moved to {pending_state:?} with pause");
        return Ok(outcome.execution_error);
    }

    tracing::info!("Session complete for task #{task_id}");

    if let Err(e) =
        perform_stash_and_push(zbobr, task_id, work_dir, stage.as_str(), pipeline).await
    {
        tracing::error!("Stash/push failed for task #{task_id}: {e}");
        let msg = format!("Stash/push failed: {e}");
        let status = format_error_status(zbobr, &msg);
        let stage = stage.to_string();
        if let Err(pause_err) = task_session
            .set_pause_with_status_and_signal(status, Signal::go(stage.as_str()))
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
        let current_stage = stage.clone();
        let stage_def = zbobr.workflow().stage(pipeline, &current_stage);
        let seq_signal = zbobr.workflow().sequential_signal(
            pipeline,
            &current_stage,
            stage_def,
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
            SequentialSignal::PauseThenSignal(signal) => {
                let status = format_error_status(zbobr, "Auto-pause: stage completed");
                task_session
                    .set_pause_with_status_and_signal(status, signal)
                    .await?;
            }
        }
    }
    // If pause was set by MCP tool (e.g. stop_with_error) but no signal, set
    // signal to re-run the current stage on resume.
    if current_task.pause && current_task.signal.is_none() {
        task_session
            .set_signal(Some(Signal::go(stage.as_str())))
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
    pipeline_name: &Pipeline,
) -> anyhow::Result<()> {
    let task_backend = zbobr.task_backend();

    // Stash uncommitted changes if work_dir is a git repository.
    // The work_dir may not yet be a git repo on the first run,
    // so we skip stash but still proceed to update_worktree below.
    let is_git_repo = git_output(work_dir, &["rev-parse", "--is-inside-work-tree"])
        .await
        .is_ok();

    if is_git_repo {
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
    } else {
        tracing::info!(
            "Skipping stash for task #{task_id}: {} is not a git repository",
            work_dir.display()
        );
    }

    let task = task_backend
        .get_task(task_id)
        .await?
        .snapshot(false)
        .await?;
    if let Some(identity) = task.identity() {
        let is_conflict_handler = pipeline_name.as_str() == Pipeline::MERGE;
        let is_uptodate = zbobr.update_worktree(&identity).await?;
        if !is_uptodate && !is_conflict_handler {
            anyhow::bail!("Merge conflict while syncing work branch for task #{task_id}");
        }
        let config = zbobr.config();
        if config.overwrite_author && is_uptodate && is_git_repo {
            let base_branch = zbobr.repo_backend().branch().to_string();
            zbobr_utility::rewrite_authors_on_worktree(
                work_dir,
                &base_branch,
                &config.git_user_name,
                &config.git_user_email,
            )
            .await?;
            // Push rewritten commits
            let is_uptodate = zbobr.update_worktree(&identity).await?;
            if !is_uptodate {
                anyhow::bail!("Merge conflict while pushing rewritten commits for task #{task_id}");
            }
        }
    } else {
        tracing::warn!("Task #{task_id} missing routing parameters — skipping push");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;
    use zbobr_api::config::StageDefinition;
    use zbobr_api::config::{PipelineConfig, WorkflowConfig};
    use zbobr_api::{Pipeline, StackEntry};

    // -- Test Helpers --

    /// Build a minimal Workflow with a single "main" pipeline containing one "working" stage.
    fn make_workflow() -> Workflow {
        let role_stage = StageDefinition {
            role: Some("worker".to_string().into()),
            ..Default::default()
        };
        let main_pipeline = PipelineConfig {
            stages: IndexMap::from([("working".into(), role_stage)]),
        };
        let config = WorkflowConfig {
            pipelines: [("main".into(), main_pipeline)].into(),
            ..Default::default()
        };
        Workflow::from_config(config)
    }

    /// Construct a Task with the given fields, defaults for the rest.
    fn make_task(
        id: u64,
        state: State,
        stage_count: u64,
        pause: bool,
        stack: Vec<StackEntry>,
    ) -> Task {
        Task {
            id,
            title: format!("task {}", id),
            description: "test description".to_string(),
            state,
            work_branch: None,
            pr_url: None,
            context: Default::default(),
            signal: None,
            stack,
            status: None,
            pause,
            confirm: false,
            pipeline_run_id: 0,
            stage_count,
            max_stage_count: 0,
            closed: false,
            etag: None,
        }
    }

    // -- Tests for select_runnable_task --

    #[test]
    fn select_runnable_task_selects_highest_stage_count() {
        let wf = make_workflow();
        let tasks = [
            make_task(1, State::Ready, 2, false, vec![]),
            make_task(2, State::Ready, 5, false, vec![]),
            make_task(3, State::Ready, 3, false, vec![]),
        ];

        let selected = select_runnable_task(&wf, &tasks);
        assert!(selected.is_some());
        assert_eq!(selected.unwrap().id, 2);
    }

    #[test]
    fn select_runnable_task_deterministic_tie_break() {
        let wf = make_workflow();
        // Two tasks with same stage_count but different IDs
        let tasks1 = [
            make_task(5, State::Ready, 10, false, vec![]),
            make_task(3, State::Ready, 10, false, vec![]),
        ];
        let tasks2 = [
            make_task(3, State::Ready, 10, false, vec![]),
            make_task(5, State::Ready, 10, false, vec![]),
        ];

        let selected1 = select_runnable_task(&wf, &tasks1);
        let selected2 = select_runnable_task(&wf, &tasks2);

        // Both should select the same task regardless of input order (deterministic)
        // The implementation uses b.id.cmp(&a.id) which selects the lower ID on ties
        assert_eq!(selected1.map(|t| t.id), Some(3));
        assert_eq!(selected2.map(|t| t.id), Some(3));
    }

    #[test]
    fn select_runnable_task_excludes_paused_tasks() {
        let wf = make_workflow();
        let tasks = [make_task(1, State::Ready, 10, true, vec![])];

        let selected = select_runnable_task(&wf, &tasks);
        assert!(selected.is_none());
    }

    #[test]
    fn select_runnable_task_excludes_ready_with_stack() {
        let wf = make_workflow();
        let stack = vec![StackEntry {
            pipeline: Pipeline::Main,
            signal: Signal::go("working"),
            pipeline_run_id: 1,
        }];
        let tasks = [make_task(1, State::Ready, 10, false, stack)];

        let selected = select_runnable_task(&wf, &tasks);
        // READY-with-stack tasks are excluded (Phase 1 normalization)
        assert!(selected.is_none());
    }

    #[test]
    fn select_runnable_task_excludes_done_tasks() {
        let wf = make_workflow();
        let tasks = [make_task(1, State::Done, 10, false, vec![])];

        let selected = select_runnable_task(&wf, &tasks);
        assert!(selected.is_none());
    }

    #[test]
    fn select_runnable_task_returns_none_on_empty_input() {
        let wf = make_workflow();
        let tasks: [Task; 0] = [];

        let selected = select_runnable_task(&wf, &tasks);
        assert!(selected.is_none());
    }

    #[test]
    fn select_runnable_task_returns_none_when_all_filtered() {
        let wf = make_workflow();
        let stack = vec![StackEntry {
            pipeline: Pipeline::Main,
            signal: Signal::go("working"),
            pipeline_run_id: 1,
        }];
        let tasks = [
            make_task(1, State::Ready, 10, true, vec![]), // paused
            make_task(2, State::Done, 5, false, vec![]),  // done
            make_task(3, State::Ready, 15, false, stack), // ready with stack
        ];

        let selected = select_runnable_task(&wf, &tasks);
        assert!(selected.is_none());
    }

    // -- Tests for TaskListEntry --

    #[test]
    fn task_list_entry_from_task_projects_correct_fields() {
        let task = make_task(42, State::Ready, 7, false, vec![]);
        let entry = TaskListEntry::from(&task);

        assert_eq!(entry.id, 42);
        assert_eq!(entry.stage_count, 7);
        assert_eq!(entry.state, State::Ready);
        assert_eq!(entry.title, "task 42");
    }

    #[test]
    fn task_list_entry_json_serialization_has_expected_keys() {
        let task = make_task(99, State::Pending(Pipeline::Main), 3, false, vec![]);
        let entry = TaskListEntry::from(&task);
        let json_str = serde_json::to_string(&entry).expect("failed to serialize");
        let json_obj: serde_json::Value =
            serde_json::from_str(&json_str).expect("failed to parse json");

        // Check that all expected keys are present
        assert!(json_obj.is_object());
        let obj = json_obj.as_object().unwrap();
        assert!(obj.contains_key("id"));
        assert!(obj.contains_key("stage_count"));
        assert!(obj.contains_key("state"));
        assert!(obj.contains_key("title"));

        // Verify values
        assert_eq!(obj["id"].as_u64(), Some(99));
        assert_eq!(obj["stage_count"].as_u64(), Some(3));
        assert_eq!(obj["title"].as_str(), Some("task 99"));
    }

    // -- Existing tests for sanitize_branch_postfix --

    #[test]
    fn sanitize_branch_postfix_basic() {
        assert_eq!(sanitize_branch_postfix("Hello World"), "hello-world");
    }

    #[test]
    fn sanitize_branch_postfix_special_chars() {
        assert_eq!(
            sanitize_branch_postfix("fix: handle special chars!@#$%"),
            "fix-handle-special-chars"
        );
    }

    #[test]
    fn sanitize_branch_postfix_consecutive_dashes() {
        assert_eq!(sanitize_branch_postfix("a---b"), "a-b");
    }

    #[test]
    fn sanitize_branch_postfix_leading_trailing_dashes() {
        assert_eq!(sanitize_branch_postfix("--hello--"), "hello");
    }

    #[test]
    fn sanitize_branch_postfix_empty() {
        assert_eq!(sanitize_branch_postfix(""), "");
    }

    #[test]
    fn sanitize_branch_postfix_only_special_chars() {
        assert_eq!(sanitize_branch_postfix("!!!"), "");
    }

    #[test]
    fn sanitize_branch_postfix_truncates_long_input() {
        let long_title = "a".repeat(60);
        let result = sanitize_branch_postfix(&long_title);
        assert!(result.len() <= 50);
        assert_eq!(result, "a".repeat(50));
    }

    #[test]
    fn sanitize_branch_postfix_truncation_trims_trailing_dash() {
        // Create input that will have a dash at position 50 after truncation
        let mut title = "a".repeat(49);
        title.push(' '); // becomes dash
        title.push_str(&"b".repeat(10));
        let result = sanitize_branch_postfix(&title);
        assert!(result.len() <= 50);
        assert!(!result.ends_with('-'));
    }

    #[test]
    fn sanitize_branch_postfix_preserves_numbers() {
        assert_eq!(sanitize_branch_postfix("task 123 fix"), "task-123-fix");
    }

    #[test]
    fn sanitize_branch_postfix_lowercases() {
        assert_eq!(sanitize_branch_postfix("FIX BUG"), "fix-bug");
    }

    #[test]
    fn sanitize_branch_postfix_unicode_no_panic() {
        // Multi-byte Unicode chars: each Japanese char is 3 bytes.
        // 20 chars × 3 bytes = 60 bytes > 50 bytes, but only 20 chars.
        // Old byte-slice would panic; char-based should not.
        let title = "タスク".repeat(20); // 60 chars
        let result = sanitize_branch_postfix(&title);
        assert!(result.chars().count() <= 50);
    }

    // -- Tests for resolve_config_location --

    #[test]
    fn resolve_config_location_default_when_empty() {
        let loc = resolve_config_location(&[], "zbobr.toml").unwrap();
        let cwd = std::env::current_dir().unwrap();
        assert_eq!(loc.config_paths.len(), 1);
        assert_eq!(loc.config_paths[0], cwd.join("zbobr.toml"));
        assert_eq!(loc.config_dir, cwd);
    }

    #[test]
    fn resolve_config_location_multiple_paths() {
        // Create temp files in different directories
        let dir_a = tempfile::tempdir().unwrap();
        let dir_b = tempfile::tempdir().unwrap();
        let file_a = dir_a.path().join("a.toml");
        let file_b = dir_b.path().join("b.toml");
        std::fs::write(&file_a, "").unwrap();
        std::fs::write(&file_b, "").unwrap();

        let loc = resolve_config_location(&[file_a, file_b.clone()], "zbobr.toml").unwrap();
        assert_eq!(loc.config_paths.len(), 2);
        // config_dir should be the last file's parent
        assert_eq!(
            loc.config_dir,
            std::fs::canonicalize(file_b.parent().unwrap()).unwrap()
        );
    }

    #[test]
    fn resolve_config_location_missing_file_errors() {
        let result = resolve_config_location(&[PathBuf::from("/nonexistent/cfg.toml")], "z.toml");
        assert!(result.is_err());
    }

    // -- Tests for ConfigFileArg short flag --

    #[test]
    fn config_file_arg_short_flag_registered() {
        let cmd = GlobalArgs::augment_args(clap::Command::new(""));
        let config_arg = cmd
            .get_arguments()
            .find(|a| a.get_long().map(|l| l == "config").unwrap_or(false));
        assert!(config_arg.is_some(), "GlobalArgs should have --config arg");
        let arg = config_arg.unwrap();
        assert_eq!(
            arg.get_short(),
            Some('c'),
            "--config should have -c short alias"
        );
    }

    #[test]
    fn global_args_includes_logs_flag() {
        let cmd = GlobalArgs::augment_args(clap::Command::new(""));
        let logs_arg = cmd
            .get_arguments()
            .find(|a| a.get_long().map(|l| l == "logs").unwrap_or(false));
        assert!(
            logs_arg.is_some(),
            "GlobalArgs should declare a --logs flag"
        );
        let action = logs_arg.unwrap().get_action();
        assert!(
            matches!(action, clap::ArgAction::SetTrue),
            "--logs should be a boolean flag (SetTrue action)"
        );
    }
}