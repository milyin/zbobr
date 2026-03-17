#![allow(clippy::needless_borrows_for_generic_args)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Context;
use clap::{Args, Parser, Subcommand};
use zbobr_executor_mcp_tester::ZbobrExecutorMcpTesterConfig;

// bring in the generic git helpers from utility crate
use zbobr_utility::{git, git_check, git_output};

use crate::{
    Comment, CommentType, Task, TaskDir, ToolExecutor, ZbobrDispatcher,
    mcp::common::get_hostname,
    task::{Model, Role, Tool},
};
use zbobr_api::config::{PipelineConfig, StageDefinition};

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

/// Top-level commands.
#[derive(Subcommand)]
pub enum Command {
    /// Initialize a task project: create repo if needed, set up stages and labels
    Setup {
        /// Force overwrite existing labels
        #[arg(long, short = 'f')]
        force: bool,
    },
    /// Poll for tasks and run roles automatically
    Loop {
        /// How often to check for new tasks, in seconds
        #[arg(long, default_value = "60")]
        interval: u64,
        /// How often to clean up workspaces for closed tasks, in seconds
        #[arg(long, default_value = "600")]
        cleanup_interval: u64,
    },
    /// Remove workspace directories for tasks that have been closed
    Cleanup {
        /// Show what would be removed without actually deleting
        #[arg(long, short = 'n')]
        dry_run: bool,
    },
    /// Manage tasks (create, show, update, delete) and run role sessions
    Task {
        #[command(subcommand)]
        subcommand: TaskSubcommand,
    },
}

/// Task management subcommands.
#[derive(Subcommand)]
pub enum TaskSubcommand {
    /// Create a new task
    Create {
        /// Task title
        title: String,
        /// Task description
        #[arg(long, default_value = "")]
        description: String,
        /// Initial state (READY, DONE, etc.; default: READY)
        #[arg(long, default_value = "READY")]
        state: String,

        /// Destination repository in owner/repo format
        #[arg(long)]
        dest_repo: Option<String>,
        /// Destination branch
        #[arg(long)]
        dest_branch: Option<String>,
        /// When set the task will be paused automatically on every state change
        #[arg(long, action = clap::ArgAction::SetTrue)]
        confirm: bool,
    },
    /// List existing tasks (optionally filter by state)
    List {
        /// Only show tasks in this state
        #[arg(long)]
        state: Option<String>,
    },
    /// Show a task by ID
    Show {
        /// Task ID
        id: u64,
    },
    /// Update fields of an existing task
    Update {
        /// Task ID
        id: u64,
        /// New title
        #[arg(long)]
        title: Option<String>,
        /// New description
        #[arg(long)]
        description: Option<String>,
        /// New state (READY, DONE, etc.)
        #[arg(long)]
        state: Option<String>,

        /// New destination repository in owner/repo format.
        /// Pass `--dest-repo` without a value to delete the parameter.
        #[arg(long, num_args = 0..=1)]
        dest_repo: Option<Option<String>>,
        /// New destination branch.
        /// Pass `--dest-branch` without a value to delete the parameter.
        #[arg(long, num_args = 0..=1)]
        dest_branch: Option<Option<String>>,
        /// New work branch.
        /// Pass `--work-branch` without a value to delete the parameter.
        #[arg(long, num_args = 0..=1)]
        work_branch: Option<Option<String>>,
        /// New signal (go_preparation, go_planning, etc.)
        #[arg(long)]
        signal: Option<String>,
        /// Set or clear the confirm flag (true/false).
        #[arg(long)]
        confirm: Option<bool>,
    },
    /// Delete (close) a task by ID
    Delete {
        /// Task ID
        id: u64,
    },
    /// Run preparator role for a specific task (sets destination repository and branches)
    Prepare {
        /// Task ID
        task: u64,
        /// Show the prompt that would be sent to the model instead of running
        #[arg(long)]
        show_prompt: bool,
    },
    /// Run planner role for a specific task (creates implementation plan)
    Plan {
        /// Task ID
        task: u64,
        /// Show the prompt that would be sent to the model instead of running
        #[arg(long)]
        show_prompt: bool,
    },
    /// Run worker role for a specific task (implements the plan, creates PR)
    Work {
        /// Task ID
        task: u64,
        /// Show the prompt that would be sent to the model instead of running
        #[arg(long)]
        show_prompt: bool,
    },
    /// Run reviewer role for a specific task (reviews the implementation)
    Review {
        /// Task ID
        task: u64,
        /// Show the prompt that would be sent to the model instead of running
        #[arg(long)]
        show_prompt: bool,
    },
    /// Run merger role for a specific task (resolves merge conflicts)
    Merge {
        /// Task ID
        task: u64,
        /// Show the prompt that would be sent to the model instead of running
        #[arg(long)]
        show_prompt: bool,
    },
    /// Process a task according to its current stage (single-step)
    Process {
        /// Task ID
        task: u64,
        /// MCP tester scenario file for preparation role
        #[arg(long)]
        executor_mcp_tester_preparation: Option<PathBuf>,
        /// MCP tester scenario file for planning role
        #[arg(long)]
        executor_mcp_tester_planning: Option<PathBuf>,
        /// MCP tester scenario file for working role
        #[arg(long)]
        executor_mcp_tester_working: Option<PathBuf>,
        /// MCP tester scenario file for reviewing role
        #[arg(long)]
        executor_mcp_tester_reviewing: Option<PathBuf>,
        /// MCP tester scenario file for testing role
        #[arg(long)]
        executor_mcp_tester_testing: Option<PathBuf>,
        /// MCP tester scenario file for merging role
        #[arg(long)]
        executor_mcp_tester_merging: Option<PathBuf>,
    },
    /// Rewrite commit authors on the task's PR branch and push back
    OverwriteAuthor {
        /// Task ID
        id: u64,
        /// Skip confirmation and force execution
        #[arg(long)]
        force: bool,
        /// Show what would be done without making changes
        #[arg(long)]
        dry_run: bool,
    },
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
    // show latest plan comment if present (old tasks used to store this in
    // `task.plan` so we try to mimic that behaviour for convenience)
    if !discussion.is_empty()
        && let Some(plan_comment) = discussion
            .iter()
            .rev()
            .find(|c| c.comment_type == CommentType::Plan)
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
            let tag = CommentTag {
                comment_type: c.comment_type,
                role: c.role,
                hostname: c.hostname.clone(),
                tool: c.tool,
                model: c.model.clone(),
            };
            println!("  [{}] {}\n{}", i + 1, tag, c.text);
        }
    }
}

// ---------------------------------------------------------------------------
// Command dispatch
// ---------------------------------------------------------------------------

impl ZbobrDispatcher {
    /// Run the given command against the dispatcher.
    pub async fn run_command(&self, command: Command, pipeline: &PipelineConfig) -> anyhow::Result<()> {
        match command {
            Command::Setup { force } => {
                self.setup(&**self.task_backend(), force).await?;
            }
            Command::Cleanup { dry_run } => {
                self.cleanup_closed_tasks(&**self.task_backend(), dry_run).await?;
            }
            Command::Task { subcommand } => {
                run_task_subcommand(self, subcommand, pipeline).await?;
            }
            Command::Loop {
                interval,
                cleanup_interval,
                ..
            } => {
                run_manager_loop(self, interval, cleanup_interval, pipeline).await?;
            }
        }
        Ok(())
    }
}

async fn run_task_subcommand(
    zbobr: &ZbobrDispatcher,
    subcommand: TaskSubcommand,
    pipeline: &PipelineConfig,
) -> anyhow::Result<()> {
    let task_backend = zbobr.task_backend();
    let repo_backend = zbobr.repo_backend();
    match subcommand {
        TaskSubcommand::Create {
            title,
            description,
            state,
            dest_repo,
            dest_branch,
            confirm,
        } => {
            let id = zbobr
                .create_task(
                    &**task_backend,
                    &title,
                    &description,
                    &state,
                    dest_repo,
                    dest_branch,
                )
                .await?;
            if confirm {
                zbobr
                    .task_session(Arc::clone(task_backend), Arc::clone(repo_backend), id)
                    .set_confirm(true)
                    .await?;
            }
            println!("Created task #{}", id);
        }
        TaskSubcommand::List { state } => {
            let weak_tasks = task_backend.list_tasks().await?;
            let mut tasks = Vec::new();
            for w in &weak_tasks {
                let task = w.snapshot().await?;
                if let Some(ref filter) = state {
                    if task.state != filter.to_uppercase() {
                        continue;
                    }
                }
                tasks.push(task);
            }
            tasks.sort_by_key(|t| t.id);

            if tasks.is_empty() {
                println!("No tasks found");
            } else {
                for task in &tasks {
                    print_task(task, &[]);
                    println!("---");
                }
            }
        }
        TaskSubcommand::Show { id } => {
            let weak = task_backend.get_task(id).await?;
            let task = weak.snapshot().await?;
            let discussion = weak.get_comments().await?;
            print_task(&task, &discussion);
        }
        TaskSubcommand::Update {
            id,
            title,
            description,
            state,
            dest_repo,
            dest_branch,
            work_branch,
            signal,
            confirm,
        } => {
            let weak = task_backend.get_task(id).await?;
            let mutable = weak.upgrade().await?;
            mutable
                .modify_task(Box::new(move |mut task| {
                    if let Some(t) = title {
                        task.title = t;
                    }
                    if let Some(d) = description {
                        task.description = d;
                    }
                    if let Some(c) = confirm {
                        task.confirm = c;
                    }
                    if let Some(s) = state {
                        if task.confirm && task.state != s {
                            task.pause = true;
                        }
                        task.state = s;
                    }
                    if let Some(s) = signal {
                        task.signal = Some(s);
                    }
                    if let Some(repo) = dest_repo {
                        task.destination_repository = repo;
                    }
                    if let Some(branch) = dest_branch {
                        task.destination_branch = branch;
                    }
                    if let Some(branch) = work_branch {
                        task.work_branch = branch;
                    }
                    task
                }))
                .await?;
            println!("Updated task #{}", id);
        }
        TaskSubcommand::Delete { id } => {
            let weak = task_backend.get_task(id).await?;
            let mutable = weak.upgrade().await?;
            mutable.close().await?;
            println!("Deleted task #{}", id);
        }
        TaskSubcommand::Prepare { task, show_prompt } => {
            run_role_subcommand(zbobr, task, Role::Preparator, show_prompt, pipeline).await?;
        }
        TaskSubcommand::Plan { task, show_prompt } => {
            run_role_subcommand(zbobr, task, Role::Planner, show_prompt, pipeline).await?;
        }
        TaskSubcommand::Work { task, show_prompt } => {
            run_role_subcommand(zbobr, task, Role::Worker, show_prompt, pipeline).await?;
        }
        TaskSubcommand::Review { task, show_prompt } => {
            run_role_subcommand(zbobr, task, Role::Reviewer, show_prompt, pipeline).await?;
        }
        TaskSubcommand::Merge { task, show_prompt } => {
            run_role_subcommand(zbobr, task, Role::Merger, show_prompt, pipeline).await?;
        }
        TaskSubcommand::Process {
            task,
            executor_mcp_tester_preparation,
            executor_mcp_tester_planning,
            executor_mcp_tester_working,
            executor_mcp_tester_reviewing,
            executor_mcp_tester_testing,
            executor_mcp_tester_merging,
        } => {
            let task_obj = task_backend.get_task(task).await?.snapshot().await?;
            let mcp_tester_config_override = if executor_mcp_tester_preparation.is_some()
                || executor_mcp_tester_planning.is_some()
                || executor_mcp_tester_working.is_some()
                || executor_mcp_tester_reviewing.is_some()
                || executor_mcp_tester_testing.is_some()
                || executor_mcp_tester_merging.is_some()
            {
                Some(ZbobrExecutorMcpTesterConfig {
                    preparation: executor_mcp_tester_preparation,
                    planning: executor_mcp_tester_planning,
                    working: executor_mcp_tester_working,
                    reviewing: executor_mcp_tester_reviewing,
                    testing: executor_mcp_tester_testing,
                    merging: executor_mcp_tester_merging,
                })
            } else {
                None
            };
            let effective_dispatcher = match mcp_tester_config_override {
                Some(mcp_tester) => zbobr.with_mcp_tester_config(mcp_tester),
                None => zbobr.clone(),
            };
            process_task(&effective_dispatcher, &task_obj, pipeline).await?;
        }
        TaskSubcommand::OverwriteAuthor { id, force, dry_run } => {
            let task = task_backend.get_task(id).await?.snapshot().await?;
            let identity = task
                .identity()
                .ok_or_else(|| anyhow::anyhow!("Task #{} missing routing parameters", id))?;

            let dest_repo = &identity.destination_repository;
            let dest_branch = &identity.destination_branch;

            if dry_run {
                println!(
                    "Dry run: would rewrite commit authors in repo '{}' (PR: '{}')",
                    dest_repo, task.title
                );
            } else if !force {
                println!(
                    "This will rewrite commit authors in repo '{}' (PR: '{}'). Continue? (yes/no)",
                    dest_repo, task.title
                );
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                if !input.trim().eq_ignore_ascii_case("yes") {
                    println!("Cancelled");
                    return Ok(());
                }
            }

            let task_dir = TaskDir::new(zbobr.config().workspaces.as_path(), id);

            // Derive the actual git repo directory (work_dir/<repo_name>)
            let repo_name = std::path::Path::new(dest_repo)
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| anyhow::anyhow!("Cannot extract repo name from: {}", dest_repo))?;
            let repo_dir = task_dir.path().join(repo_name);

            // Ensure workspace exists and is set up
            if !repo_dir.exists() {
                return Err(anyhow::anyhow!(
                    "Task repo not found at {}. Run 'zbobr task clone {}' first.",
                    repo_dir.display(),
                    id
                ));
            }

            // Fetch the latest from remote
            git(&repo_dir, &["fetch", "origin", dest_branch]).await?;

            if !dry_run {
                repo_backend
                    .rewrite_commit_authors(&identity, &repo_dir, dest_branch)
                    .await?;
                println!("Successfully rewrote commit authors and pushed");
            } else {
                // Show commits that would be rewritten
                if let Ok(log) = git_output(
                    &repo_dir,
                    &[
                        "log",
                        &format!("{}..HEAD", dest_branch),
                        "--format=%H %an <%ae>",
                    ],
                )
                .await
                {
                    println!("Commits that would be rewritten:");
                    for line in log.lines() {
                        println!("  {}", line);
                    }
                }
                println!("Dry run completed. No commits were modified.");
            }
        }
    }
    Ok(())
}

/// Helper for convenience subcommands (Prepare, Plan, Work, Review, Merge).
/// Finds the stage definition by role in the pipeline and runs it.
async fn run_role_subcommand(
    zbobr: &ZbobrDispatcher,
    task_id: u64,
    role: Role,
    show_prompt: bool,
    pipeline: &PipelineConfig,
) -> anyhow::Result<()> {
    let stage_def = pipeline
        .find_stage_by_role(role)
        .ok_or_else(|| anyhow::anyhow!("No stage definition found for role {:?} in pipeline", role))?;
    let runner = CliStageRunner::new(zbobr, task_id, stage_def, pipeline);
    if show_prompt {
        println!("{}", runner.prompt().await?);
    } else {
        runner.run().await?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// CliStageRunner — CLI-side stage execution
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

    fn pending_state(&self) -> String {
        format!("{}_PENDING", self.stage_def.mode)
    }

    async fn prompt(&self) -> anyhow::Result<String> {
        self.zbobr
            .prompt_builder()
            .build_for_stage(self.stage_def, self.task_id, &**self.zbobr.task_backend())
            .await
    }

    async fn run(&self) -> anyhow::Result<()> {
        let role = self.stage_def.role;
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

        let (work_dir, is_uptodate) = prepare_workspace(
            self.zbobr,
            self.task_id,
            role,
            task_dir.path(),
        )
        .await?;

        if matches!(role, Role::Preparator) {
            seed_preparator_defaults(self.zbobr, self.task_id).await?;
        } else {
            ensure_pr_url(self.zbobr, self.task_id).await?;
        }

        // If the work branch has diverged and on_conflict is configured,
        // push current stage onto stack and signal the conflict mode.
        if !is_uptodate && role != Role::Merger {
            if let Some(ref conflict_mode) = self.zbobr.config().on_conflict {
                tracing::info!(
                    "Task #{} work branch diverged — calling conflict mode '{}'",
                    self.task_id,
                    conflict_mode
                );
                let task_session = self.zbobr.task_session(
                    Arc::clone(self.zbobr.task_backend()),
                    Arc::clone(self.zbobr.repo_backend()),
                    self.task_id,
                );
                task_session
                    .push_stack(&self.stage_def.mode, &self.stage_def.name)
                    .await?;
                task_session
                    .set_signal(Some(&format!("call_{}", conflict_mode)))
                    .await?;
                task_session.set_state(&self.pending_state()).await?;
                return Ok(());
            }
        }

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

        // For Merger role: try a normal git merge first.
        if role == Role::Merger {
            let task = self
                .zbobr
                .task_backend()
                .get_task(self.task_id)
                .await?
                .snapshot()
                .await?;
            let dest_branch = task
                .destination_branch
                .clone()
                .unwrap_or_else(|| "main".to_string());
            let merged_ok = git_check(&work_dir, &["merge", &dest_branch, "--no-edit"])
                .await
                .context("Failed to run git merge for Merger")?;
            if merged_ok {
                tracing::info!(
                    "Task #{}: normal merge with '{}' succeeded — skipping agent session",
                    self.task_id,
                    dest_branch
                );
                perform_auto_commit_and_push(
                    self.zbobr,
                    self.task_id,
                    &work_dir,
                    role,
                )
                .await?;
                // Compute post-stage signal from transitions
                let signal = compute_post_stage_signal(self.stage_def, None);
                let task_session = self.zbobr.task_session(
                    Arc::clone(self.zbobr.task_backend()),
                    Arc::clone(self.zbobr.repo_backend()),
                    self.task_id,
                );
                task_session.set_signal(Some(&signal)).await?;
                task_session.set_state(&self.pending_state()).await?;
                return Ok(());
            }
            tracing::info!(
                "Task #{}: normal merge with '{}' failed — invoking agent",
                self.task_id,
                dest_branch
            );
            let _ = git(&work_dir, &["merge", "--abort"]).await;
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

        let (assigned_port, server_handle) = start_mcp_server(
            self.zbobr.clone(),
            role,
            self.task_id,
            cli_tool,
            model.clone(),
            self.stage_def.name.clone(),
        )
        .await?;

        let mcp_url = format!(
            "http://127.0.0.1:{assigned_port}/{role}/{task_id}",
            assigned_port = assigned_port,
            role = role,
            task_id = self.task_id,
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

        if let Some(e) = finalize_stage_session(
            self.zbobr,
            self.task_id,
            self.stage_def,
            self.pipeline,
            &work_dir,
            outcome,
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
            task_session.finish().await?;
            println!("Task #{} completed", task.id);
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
                    if let Err(e) = task_session.finish().await {
                        tracing::error!("Failed to finish task #{}: {e}", task.id);
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

async fn prepare_workspace(
    zbobr: &ZbobrDispatcher,
    task_id: u64,
    role: Role,
    task_dir: &Path,
) -> anyhow::Result<(PathBuf, bool)> {
    let task_backend = zbobr.task_backend();
    let repo_backend = zbobr.repo_backend();
    match role {
        Role::Preparator => Ok((task_dir.to_path_buf(), true)),
        Role::Merger => {
            let task = task_backend.get_task(task_id).await?.snapshot().await?;
            let dest_repo = task
                .destination_repository
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("Task #{task_id} has no destination_repository"))?;
            let repo_name = dest_repo.rsplit('/').next().unwrap_or(dest_repo);
            Ok((task_dir.join(repo_name), true))
        }
        _ => {
            let task = task_backend.get_task(task_id).await?.snapshot().await?;
            let identity = task.identity().ok_or_else(|| {
                anyhow::anyhow!("Task #{task_id} is missing routing parameters (destination_repository, destination_branch, work_branch)")
            })?;
            match zbobr.update_worktree(&**repo_backend, &identity).await {
                Ok(is_uptodate) => {
                    let dest_repo = &identity.destination_repository;
                    let repo_name = dest_repo.rsplit('/').next().unwrap_or(dest_repo);
                    let task_dir = TaskDir::new(zbobr.config().workspaces.as_path(), task_id);
                    let path = task_dir.path().join(repo_name);
                    Ok((path, is_uptodate))
                }
                Err(e) => {
                    let msg = format!("Failed to prepare workspace for task #{task_id}: {e:#}");
                    tracing::error!("{msg}");
                    let hostname = get_hostname();
                    if let Err(post_err) = zbobr
                        .task_session(Arc::clone(task_backend), Arc::clone(repo_backend), task_id)
                        .post_comment(CommentType::Error, &msg, None, &hostname, None, None)
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
                .post_comment(CommentType::Error, &msg, None, &hostname, None, None)
                .await
            {
                tracing::warn!("Failed to post error to task discussion: {post_err}");
            }
            Err(anyhow::anyhow!(msg))
        }
    }
}

/// Pre-populate task parameters from dispatcher config defaults before the
/// preparator agent runs. Only sets a parameter if it is not already present,
/// so a previously prepared task (e.g. re-run) keeps its values unchanged.
async fn seed_preparator_defaults(
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
    role: Role,
    task_id: u64,
    tool: Tool,
    model: Model,
    stage_name: String,
) -> anyhow::Result<(u16, tokio::task::JoinHandle<()>)> {
    let (port_tx, port_rx) = tokio::sync::oneshot::channel();
    let task_backend = Arc::clone(zbobr.task_backend());
    let server_handle = tokio::spawn(async move {
        match crate::mcp::run_role_mcp_server(zbobr, task_backend, role, task_id, tool, model, stage_name).await
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
    role: Role,
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
) -> anyhow::Result<Option<anyhow::Error>> {
    let role = stage_def.role;
    let task_backend = zbobr.task_backend();
    let repo_backend = zbobr.repo_backend();
    let task_session = zbobr.task_session(Arc::clone(task_backend), Arc::clone(repo_backend), task_id);
    let pending_state = format!("{}_PENDING", stage_def.mode);

    if outcome.execution_interrupted {
        if matches!(role, Role::Worker | Role::Merger)
            && let Err(e) =
                perform_auto_commit_and_push(zbobr, task_id, work_dir, role)
                    .await
        {
            tracing::warn!("Auto-commit/push failed during interruption for task #{task_id}: {e}");
        }
        task_session.set_state(&pending_state).await?;
        tracing::info!("Session interrupted for task #{task_id}, moved to {pending_state}");
        return Ok(None);
    }

    if let Some(e) = outcome.execution_error.as_ref() {
        if matches!(role, Role::Worker | Role::Merger)
            && let Err(e) =
                perform_auto_commit_and_push(zbobr, task_id, work_dir, role)
                    .await
        {
            tracing::warn!(
                "Auto-commit/push failed during error handling for task #{task_id}: {e}"
            );
        }
        let error_msg = format!("Execution failed: {e}");
        let hostname = get_hostname();
        if let Err(post_err) = task_session
            .post_comment(CommentType::Error, &error_msg, None, &hostname, None, None)
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

    if (role == Role::Worker || role == Role::Merger)
        && let Err(e) =
            perform_auto_commit_and_push(zbobr, task_id, work_dir, role).await
    {
        tracing::error!("Auto-commit/push failed for task #{task_id}: {e}");
        let hostname = get_hostname();
        let msg = format!("Auto-commit/push failed: {e}");
        if let Err(post_err) = task_session
            .post_comment(CommentType::Error, &msg, None, &hostname, None, None)
            .await
        {
            tracing::error!(
                "Failed to post auto-commit/push error for task #{task_id}: {post_err}"
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
                "Failed to pause task #{task_id} after auto-commit/push failure: {pause_err}"
            );
        }
        task_session.set_state(&pending_state).await?;
        return Ok(None);
    }

    // Merger: verify the merge actually succeeded
    if role == Role::Merger {
        let dest_branch = task_backend
            .get_task(task_id)
            .await?
            .snapshot()
            .await?
            .destination_branch
            .clone()
            .unwrap_or_else(|| "main".to_string());
        let merged_ok = git_check(work_dir, &["merge", &dest_branch, "--no-edit"])
            .await
            .context("Failed to run git merge verification after Merger session")?;
        if !merged_ok {
            let _ = git(work_dir, &["merge", "--abort"]).await;
            let msg = format!(
                "Merger failed to resolve merge conflict with branch '{dest_branch}'. \
                 Manual intervention required."
            );
            tracing::error!("task #{task_id}: {msg}");
            let hostname = get_hostname();
            if let Err(e) = task_session
                .post_comment(CommentType::Error, &msg, None, &hostname, None, None)
                .await
            {
                tracing::warn!(
                    "Failed to post merger-failure comment for task #{task_id}: {e}"
                );
            }
            if let Err(e) = task_session
                .modify_task(|mut task| {
                    task.pause = true;
                    task
                })
                .await
            {
                tracing::warn!("Failed to pause task #{task_id} after merger failure: {e}");
            }
            task_session.set_state(&pending_state).await?;
            return Ok(None);
        }
    }

    // Compute post-stage signal from transitions map.
    // If the agent already set a signal during the session (e.g. reject),
    // that signal takes priority.
    let current_task = task_backend.get_task(task_id).await?.snapshot().await?;
    if !current_task.pause && current_task.signal.is_none() {
        let signal = compute_post_stage_signal(stage_def, None);
        task_session.set_signal(Some(&signal)).await?;
    }
    task_session.set_state(&pending_state).await?;

    Ok(None)
}

async fn perform_auto_commit_and_push(
    zbobr: &ZbobrDispatcher,
    task_id: u64,
    work_dir: &Path,
    role: Role,
) -> anyhow::Result<()> {
    let task_backend = zbobr.task_backend();
    let repo_backend = zbobr.repo_backend();
    tracing::info!("Checking for uncommitted changes in {}", work_dir.display());

    match git_output(work_dir, &["status", "--porcelain"]).await {
        Ok(status) => {
            if !status.is_empty() {
                tracing::info!("Found uncommitted changes, auto-committing...");
                if let Err(e) = git(work_dir, &["add", "."]).await {
                    tracing::warn!("Failed to stage changes for auto-commit: {e}");
                }
                let commit_msg = format!("Auto-commit by {} agent", role.as_str());
                match git(work_dir, &["commit", "-m", &commit_msg]).await {
                    Ok(_) => tracing::info!("Auto-commit successful"),
                    Err(e) => tracing::warn!("Auto-commit failed: {e}"),
                }
            } else {
                tracing::info!("No uncommitted changes found");
            }
        }
        Err(e) => tracing::warn!("Failed to check git status for auto-commit: {e}"),
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

use zbobr_api::CommentTag;
