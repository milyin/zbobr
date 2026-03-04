#![allow(clippy::needless_borrows_for_generic_args)]

use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::{Args, Parser, Subcommand};
use tokio::process::Command as TokioCommand;
use zbobr_executor_claude::ClaudeExecutor;
use zbobr_executor_copilot::CopilotExecutor;
use zbobr_executor_mcp_tester::{McpTesterExecutor, ZbobrExecutorMcpTesterConfig};

use crate::{
    Comment, CommentType, Signal, Stage, Task, ToolExecutor, ZbobrDispatcherDyn,
    ZbobrExecutorConfig,
    mcp::common::get_hostname,
    prompts::Prompts,
    task::{Model, Parameter, Role, Tool},
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

/// Global arguments that should be hoisted before subcommands.
/// This includes only dispatcher and executor config, not backend-specific settings.
/// For a full CLI with backend-specific options, use GenericCli<TA, RA> instead.
#[derive(Args, Clone)]
pub struct GlobalArgs {
    #[command(
        flatten,
        next_help_heading = "[config] Meta options and config file overrides"
    )]
    pub config_file: ConfigFileArg,

    #[command(flatten, next_help_heading = "[dispatcher]")]
    pub dispatcher: crate::ZbobrDispatcherArgs,

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
        /// AI model to use (e.g. "gpt-5-mini", "claude-3-5-sonnet")
        #[arg(long)]
        model: Option<String>,
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
        /// Initial stage (PENDING, GO_PREPARATION, etc.; default: PENDING)
        #[arg(long, default_value = "PENDING")]
        stage: String,
        /// AI tool to assign (copilot, claude, mcp-tester)
        #[arg(long)]
        tool: Option<String>,
        /// AI model to assign (e.g. "claude-3-5-sonnet")
        #[arg(long)]
        model: Option<String>,
        /// Destination repository in owner/repo format
        #[arg(long)]
        dest_repo: Option<String>,
        /// Destination branch
        #[arg(long)]
        dest_branch: Option<String>,
        /// When set the task will be paused automatically on every stage change
        #[arg(long, action = clap::ArgAction::SetTrue)]
        confirm: bool,
    },
    /// List existing tasks (optionally filter by stage or tool)
    List {
        /// Only show tasks in this stage (PENDING, GO_PREPARATION, etc.)
        #[arg(long)]
        stage: Option<String>,
        /// Only show tasks assigned to this tool (copilot, claude, mcp-tester)
        #[arg(long)]
        tool: Option<String>,
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
        /// New stage (PENDING, GO_PREPARATION, etc.)
        #[arg(long)]
        stage: Option<String>,
        /// New AI tool (copilot, claude, mcp-tester)
        #[arg(long)]
        tool: Option<String>,
        /// New AI model
        #[arg(long)]
        model: Option<String>,
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
    /// Clone the task's destination repository into its workspace
    Clone {
        /// Task ID
        task: u64,
    },
    /// Run preparator role for a specific task (sets destination repository and branches)
    Prepare {
        /// Task ID
        task: u64,
        /// AI model to use (e.g. "gpt-5-mini", "claude-3-5-sonnet")
        #[arg(long)]
        model: Option<String>,
        /// Show the prompt that would be sent to the model instead of running
        #[arg(long)]
        show_prompt: bool,
    },
    /// Run analyser role for a specific task (analyses the codebase)
    Analyse {
        /// Task ID
        task: u64,
        /// AI model to use (e.g. "gpt-5-mini", "claude-3-5-sonnet")
        #[arg(long)]
        model: Option<String>,
        /// Show the prompt that would be sent to the model instead of running
        #[arg(long)]
        show_prompt: bool,
    },
    /// Run planner role for a specific task (creates implementation plan)
    Plan {
        /// Task ID
        task: u64,
        /// AI model to use (e.g. "gpt-5-mini", "claude-3-5-sonnet")
        #[arg(long)]
        model: Option<String>,
        /// Show the prompt that would be sent to the model instead of running
        #[arg(long)]
        show_prompt: bool,
    },
    /// Run worker role for a specific task (implements the plan, creates PR)
    Work {
        /// Task ID
        task: u64,
        /// AI model to use (e.g. "gpt-5-mini", "claude-3-5-sonnet")
        #[arg(long)]
        model: Option<String>,
        /// Show the prompt that would be sent to the model instead of running
        #[arg(long)]
        show_prompt: bool,
    },
    /// Run reviewer role for a specific task (reviews the implementation)
    Review {
        /// Task ID
        task: u64,
        /// AI model to use (e.g. "gpt-5-mini", "claude-3-5-sonnet")
        #[arg(long)]
        model: Option<String>,
        /// Show the prompt that would be sent to the model instead of running
        #[arg(long)]
        show_prompt: bool,
    },
    /// Run merger role for a specific task (resolves merge conflicts)
    Merge {
        /// Task ID
        task: u64,
        /// AI model to use (e.g. "gpt-5-mini", "claude-3-5-sonnet")
        #[arg(long)]
        model: Option<String>,
        /// Show the prompt that would be sent to the model instead of running
        #[arg(long)]
        show_prompt: bool,
    },
    /// Process a task according to its current stage (single-step)
    Process {
        /// Task ID
        task: u64,
        /// AI model override to use when role execution is needed
        #[arg(long)]
        model: Option<String>,
        /// MCP tester scenario file for preparation role
        #[arg(long)]
        executor_mcp_tester_preparation: Option<PathBuf>,
        /// MCP tester scenario file for analysing role
        #[arg(long)]
        executor_mcp_tester_analysing: Option<PathBuf>,
        /// MCP tester scenario file for planning role
        #[arg(long)]
        executor_mcp_tester_planning: Option<PathBuf>,
        /// MCP tester scenario file for working role
        #[arg(long)]
        executor_mcp_tester_working: Option<PathBuf>,
        /// MCP tester scenario file for reviewing role
        #[arg(long)]
        executor_mcp_tester_reviewing: Option<PathBuf>,
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

/// Standard CLI structure for Zbobr dispatcher apps
#[derive(clap::Parser)]
pub struct GenericCli<
    TTaskArgs: zbobr_utility::PrefixedArgs + Default + Clone + std::fmt::Debug,
    TRepoArgs: zbobr_utility::PrefixedArgs + Default + Clone + std::fmt::Debug,
> {
    #[command(
        flatten,
        next_help_heading = "[config] Meta options and config file overrides"
    )]
    pub config_file: ConfigFileArg,

    #[command(flatten)]
    pub settings: crate::GenericConfigArgs<TTaskArgs, TRepoArgs>,

    #[command(subcommand)]
    pub command: Command,
}

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
    println!("Stage:       {}", task.stage);
    println!(
        "Tool:        {}",
        task.tool
            .map(|t| t.to_string())
            .unwrap_or_else(|| "(none)".to_string())
    );
    println!(
        "Model:       {}",
        task.model
            .as_ref()
            .map(|m| m.to_string())
            .unwrap_or_else(|| "(none)".to_string())
    );
    println!(
        "Signal:      {}",
        task.signal
            .map(|s| s.to_string())
            .unwrap_or_else(|| "(none)".to_string())
    );
    println!("Conflict:    {}", task.conflict);
    println!("Pause:       {}", task.pause);
    if !task.parameters.is_empty() {
        println!("Parameters:");
        for (k, v) in &task.parameters {
            println!("  {}: {}", k.name(), v);
        }
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
                model: c.model.clone(),
            };
            println!("  [{}] {}\n{}", i + 1, tag, c.text);
        }
    }
}

// ---------------------------------------------------------------------------
// Command dispatch
// ---------------------------------------------------------------------------

/// Run the given command against the dispatcher.
pub async fn run_command(
    zbobr: ZbobrDispatcherDyn,
    command: Command,
    prompts: &Prompts,
    executor_config: &ZbobrExecutorConfig,
) -> anyhow::Result<()> {
    match command {
        Command::Setup { force } => {
            zbobr.setup(force).await?;
        }
        Command::Cleanup { dry_run } => {
            zbobr.cleanup_closed_tasks(dry_run).await?;
        }
        Command::Task { subcommand } => {
            run_task_subcommand(&zbobr, subcommand, prompts, executor_config).await?;
        }
        Command::Loop {
            interval,
            cleanup_interval,
            model,
            ..
        } => {
            let model_enum = model
                .map(|m| m.parse::<Model>())
                .transpose()
                .context("Invalid model name")?;
            run_manager_loop(
                &zbobr,
                interval,
                cleanup_interval,
                model_enum,
                prompts,
                executor_config,
            )
            .await?;
        }
    }
    Ok(())
}

async fn run_task_subcommand(
    zbobr: &ZbobrDispatcherDyn,
    subcommand: TaskSubcommand,
    prompts: &Prompts,
    executor_config: &ZbobrExecutorConfig,
) -> anyhow::Result<()> {
    match subcommand {
        TaskSubcommand::Create {
            title,
            description,
            stage,
            tool,
            model,
            dest_repo,
            dest_branch,
            confirm,
        } => {
            let stage = Stage::from_milestone_name(&stage.to_uppercase())
                .ok_or_else(|| anyhow::anyhow!("Invalid stage: {}", stage))?;
            let tool = tool
                .map(|t| t.parse::<Tool>())
                .transpose()
                .context("Invalid tool")?;
            let model = model
                .map(|m| m.parse::<Model>())
                .transpose()
                .context("Invalid model")?;
            let id = zbobr
                .create_task(
                    &title,
                    &description,
                    stage,
                    tool,
                    model,
                    dest_repo,
                    dest_branch,
                )
                .await?;
            if confirm {
                zbobr.task_session(id).set_confirm(true).await?;
            }
            println!("Created task #{}", id);
        }
        TaskSubcommand::List { stage, tool } => {
            let stage_filter = if let Some(s) = stage {
                Some(
                    Stage::from_milestone_name(&s.to_uppercase())
                        .ok_or_else(|| anyhow::anyhow!("Invalid stage: {}", s))?,
                )
            } else {
                None
            };
            let tool_filter = if let Some(t) = tool {
                Some(t.parse::<Tool>()?)
            } else {
                None
            };

            let mut tasks = Vec::new();
            if let Some(stage) = stage_filter {
                tasks = zbobr.list_tasks_by_stage(stage, tool_filter).await?;
            } else {
                let all_stages = [
                    Stage::Pending,
                    Stage::Preparing,
                    Stage::Planning,
                    Stage::Working,
                    Stage::Reviewing,
                    Stage::Merging,
                    Stage::Done,
                ];
                for st in all_stages {
                    let mut ts = zbobr.list_tasks_by_stage(st, tool_filter).await?;
                    tasks.append(&mut ts);
                }
                tasks.sort_by_key(|t| t.id);
            }

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
            let task = zbobr.get_task(id).await?;
            let discussion = zbobr.get_task_comments(id).await?;
            print_task(&task, &discussion);
        }
        TaskSubcommand::Update {
            id,
            title,
            description,
            stage,
            tool,
            model,
            dest_repo,
            dest_branch,
            work_branch,
            signal,
            confirm,
        } => {
            let stage = stage
                .map(|s| {
                    Stage::from_milestone_name(&s.to_uppercase())
                        .ok_or_else(|| anyhow::anyhow!("Invalid stage: {}", s))
                })
                .transpose()?;
            let tool = tool
                .map(|t| t.parse::<Tool>().context("Invalid tool"))
                .transpose()?;
            let model = model
                .map(|m| m.parse::<Model>().context("Invalid model"))
                .transpose()?;
            let signal = signal
                .map(|s| s.parse::<Signal>().context("Invalid signal"))
                .transpose()?;
            zbobr
                .modify_task(
                    id,
                    Box::new(move |mut task| {
                        if let Some(t) = title {
                            task.title = t;
                        }
                        if let Some(d) = description {
                            task.description = d;
                        }
                        if let Some(c) = confirm {
                            task.confirm = c;
                        }
                        if let Some(s) = stage {
                            if task.confirm && task.stage != s {
                                task.pause = true;
                            }
                            task.stage = s;
                        }
                        if let Some(to) = tool {
                            task.tool = Some(to);
                        }
                        if let Some(m) = model {
                            task.model = Some(m);
                        }
                        if let Some(s) = signal {
                            task.signal = Some(s);
                        }
                        if let Some(repo) = dest_repo {
                            match repo {
                                Some(repo) => {
                                    task.parameters
                                        .insert(Parameter::DestinationRepository, repo);
                                }
                                None => {
                                    task.parameters.remove(&Parameter::DestinationRepository);
                                }
                            }
                        }
                        if let Some(branch) = dest_branch {
                            match branch {
                                Some(branch) => {
                                    task.parameters.insert(Parameter::DestinationBranch, branch);
                                }
                                None => {
                                    task.parameters.remove(&Parameter::DestinationBranch);
                                }
                            }
                        }
                        if let Some(branch) = work_branch {
                            match branch {
                                Some(branch) => {
                                    task.parameters.insert(Parameter::WorkBranch, branch);
                                }
                                None => {
                                    task.parameters.remove(&Parameter::WorkBranch);
                                }
                            }
                        }
                        task
                    }),
                )
                .await?;
            println!("Updated task #{}", id);
        }
        TaskSubcommand::Delete { id } => {
            zbobr.close_task(id).await?;
            println!("Deleted task #{}", id);
        }
        TaskSubcommand::Clone { task } => {
            let t = zbobr.get_task(task).await?;
            let dest_repo = t
                .parameters
                .get(&Parameter::DestinationRepository)
                .ok_or_else(|| anyhow::anyhow!("Task #{task} has no destination repository"))?
                .clone();
            let dest_branch = t
                .parameters
                .get(&Parameter::DestinationBranch)
                .cloned()
                .unwrap_or_else(|| "main".to_string());
            let work_branch_for_clone = t
                .parameters
                .get(&Parameter::WorkBranch)
                .cloned()
                .unwrap_or_else(|| dest_branch.clone());
            let path = zbobr
                .clone_and_setup(&dest_repo, &work_branch_for_clone, &dest_branch, task)
                .await?;
            println!("Cloned to {}", path.display());
        }
        TaskSubcommand::Prepare {
            task,
            model,
            show_prompt,
        } => {
            run_role_command(
                zbobr,
                task,
                Role::Preparator,
                model,
                show_prompt,
                prompts,
                executor_config,
            )
            .await?;
        }
        TaskSubcommand::Analyse {
            task,
            model,
            show_prompt,
        } => {
            run_role_command(
                zbobr,
                task,
                Role::Analyser,
                model,
                show_prompt,
                prompts,
                executor_config,
            )
            .await?;
        }
        TaskSubcommand::Plan {
            task,
            model,
            show_prompt,
        } => {
            run_role_command(
                zbobr,
                task,
                Role::Planner,
                model,
                show_prompt,
                prompts,
                executor_config,
            )
            .await?;
        }
        TaskSubcommand::Work {
            task,
            model,
            show_prompt,
        } => {
            run_role_command(
                zbobr,
                task,
                Role::Worker,
                model,
                show_prompt,
                prompts,
                executor_config,
            )
            .await?;
        }
        TaskSubcommand::Review {
            task,
            model,
            show_prompt,
        } => {
            run_role_command(
                zbobr,
                task,
                Role::Reviewer,
                model,
                show_prompt,
                prompts,
                executor_config,
            )
            .await?;
        }
        TaskSubcommand::Merge {
            task,
            model,
            show_prompt,
        } => {
            run_role_command(
                zbobr,
                task,
                Role::Merger,
                model,
                show_prompt,
                prompts,
                executor_config,
            )
            .await?;
        }
        TaskSubcommand::Process {
            task,
            model,
            executor_mcp_tester_preparation,
            executor_mcp_tester_analysing,
            executor_mcp_tester_planning,
            executor_mcp_tester_working,
            executor_mcp_tester_reviewing,
            executor_mcp_tester_merging,
        } => {
            let model_enum = model
                .map(|m| m.parse::<Model>())
                .transpose()
                .context("Invalid model name")?;
            let task_obj = zbobr.get_task(task).await?;
            let mcp_tester_config_override = if executor_mcp_tester_preparation.is_some()
                || executor_mcp_tester_analysing.is_some()
                || executor_mcp_tester_planning.is_some()
                || executor_mcp_tester_working.is_some()
                || executor_mcp_tester_reviewing.is_some()
                || executor_mcp_tester_merging.is_some()
            {
                Some(ZbobrExecutorMcpTesterConfig {
                    preparation: executor_mcp_tester_preparation,
                    analysing: executor_mcp_tester_analysing,
                    planning: executor_mcp_tester_planning,
                    working: executor_mcp_tester_working,
                    reviewing: executor_mcp_tester_reviewing,
                    merging: executor_mcp_tester_merging,
                })
            } else {
                None
            };
            let effective_executor_config = match mcp_tester_config_override {
                Some(mcp_tester) => ZbobrExecutorConfig {
                    mcp_tester,
                    ..executor_config.clone()
                },
                None => executor_config.clone(),
            };
            process_task_by_stage(
                zbobr,
                &task_obj,
                model_enum,
                prompts,
                &effective_executor_config,
            )
            .await?;
        }
        TaskSubcommand::OverwriteAuthor { id, force, dry_run } => {
            let task = zbobr.get_task(id).await?;
            let git_user_name = &zbobr.config().git_user_name;
            let git_user_email = &zbobr.config().git_user_email;

            // Ensure task has destination repo and branch
            let dest_repo = task
                .parameters
                .get(&Parameter::DestinationRepository)
                .ok_or_else(|| anyhow::anyhow!("Task #{} has no destination repository", id))?
                .clone();

            if dry_run {
                println!(
                    "Dry run: would rewrite commit authors to '{} <{}>' in repo '{}' (PR: '{}')",
                    git_user_name, git_user_email, dest_repo, task.title
                );
            } else if !force {
                println!(
                    "This will rewrite commit authors to '{} <{}>' in repo '{}' (PR: '{}'). Continue? (yes/no)",
                    git_user_name, git_user_email, dest_repo, task.title
                );
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                if !input.trim().eq_ignore_ascii_case("yes") {
                    println!("Cancelled");
                    return Ok(());
                }
            }

            let work_dir = zbobr.config().workspaces.join(format!("task#{}", id));

            // Derive the actual git repo directory (work_dir/<repo_name>)
            let repo_name = std::path::Path::new(&dest_repo)
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| anyhow::anyhow!("Cannot extract repo name from: {}", dest_repo))?;
            let repo_dir = work_dir.join(repo_name);

            // Fetch latest to ensure we have the destination branch
            let dest_branch = task
                .parameters
                .get(&Parameter::DestinationBranch)
                .cloned()
                .unwrap_or_else(|| "main".to_string());

            // Ensure workspace exists and is set up
            if !repo_dir.exists() {
                return Err(anyhow::anyhow!(
                    "Task repo not found at {}. Run 'zbobr task clone {}' first.",
                    repo_dir.display(),
                    id
                ));
            }

            // Fetch the latest from remote
            let fetch_cmd = TokioCommand::new("git")
                .args(["fetch", "origin", &dest_branch])
                .current_dir(&repo_dir)
                .output()
                .await;

            match fetch_cmd {
                Ok(output) if !output.status.success() => {
                    return Err(anyhow::anyhow!(
                        "Failed to fetch '{}' from remote in '{}': {}",
                        dest_branch,
                        repo_dir.display(),
                        String::from_utf8_lossy(&output.stderr).trim()
                    ));
                }
                Err(e) => {
                    return Err(anyhow::anyhow!(
                        "Failed to run git fetch in '{}': {}",
                        repo_dir.display(),
                        e
                    ));
                }
                Ok(_) => {}
            }

            // Call the rewrite_commit_authors function
            rewrite_commit_authors(zbobr, id, &repo_dir, dry_run).await?;
            if dry_run {
                println!("Dry run completed. No commits were modified.");
            } else {
                println!("Successfully rewrote commit authors and pushed");
            }
        }
    }
    Ok(())
}

async fn run_role_command(
    zbobr: &ZbobrDispatcherDyn,
    task: u64,
    role: Role,
    model: Option<String>,
    show_prompt: bool,
    prompts: &Prompts,
    executor_config: &ZbobrExecutorConfig,
) -> anyhow::Result<()> {
    let model_enum = model
        .map(|m| m.parse::<Model>())
        .transpose()
        .context("Invalid model name")?;
    let session = CliRoleRunner::new(zbobr, task, role, model_enum, prompts, executor_config);
    if show_prompt {
        println!("{}", session.prompt()?);
    } else {
        session.run().await?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// CliRoleRunner — CLI-side role execution (analogous to role_session.rs in zbobr)
// ---------------------------------------------------------------------------

struct CliRoleRunner<'a> {
    zbobr: &'a ZbobrDispatcherDyn,
    task_id: u64,
    role: Role,
    model: Option<Model>,
    prompts: &'a Prompts,
    executor_config: &'a ZbobrExecutorConfig,
}

impl<'a> CliRoleRunner<'a> {
    fn new(
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

    fn prompt(&self) -> anyhow::Result<String> {
        self.prompts.build_prompt(self.role)
    }

    async fn run(&self) -> anyhow::Result<()> {
        let cli_tool = self.zbobr.config().cli_tool;
        let model = resolve_model(cli_tool, self.model.clone(), self.executor_config);

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

        if matches!(self.role, Role::Preparator) {
            seed_preparator_defaults(self.zbobr, self.task_id).await?;
        } else if !matches!(self.role, Role::Analyser) {
            ensure_pr_url(self.zbobr, self.task_id).await?;
        }

        // Early-merge check must run BEFORE clearing the triggering condition.
        // If a conflict is detected the session exits here — the signal is left
        // intact so the Merger can return to the same stage after resolving it.
        if should_try_early_merge(self.role)
            && try_early_merge(self.zbobr, self.task_id, &work_dir).await?
        {
            return Ok(());
        }

        // Rule 1: clear the triggering condition right before the agent session
        // starts (no conflict was detected above).
        // For Merger: clear the conflict flag but NOT the signal — the signal
        //   carries the original stage to return to after merging.
        // For all other roles: clear the signal that caused entry.
        {
            let task_session = self.zbobr.task_session(self.task_id);
            if self.role == Role::Merger {
                task_session
                    .set_conflict(false)
                    .await
                    .context("Failed to clear conflict flag on entry to Merger")?;
            } else {
                task_session
                    .set_signal(None)
                    .await
                    .context("Failed to clear signal on stage entry")?;
            }
        }

        let (assigned_port, server_handle) =
            start_mcp_server(self.zbobr.clone(), self.role, self.task_id).await?;

        let mcp_url = format!(
            "http://127.0.0.1:{assigned_port}/{role}/{task_id}",
            assigned_port = assigned_port,
            role = self.role,
            task_id = self.task_id,
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
            self.zbobr,
        )
        .await;

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

// ---------------------------------------------------------------------------
// Stage processing helpers
// ---------------------------------------------------------------------------

/// Process a task according to its current stage (single-step).
pub async fn process_task_by_stage(
    zbobr: &ZbobrDispatcherDyn,
    task: &Task,
    model: Option<Model>,
    prompts: &Prompts,
    executor_config: &ZbobrExecutorConfig,
) -> anyhow::Result<()> {
    match task.stage {
        Stage::Pending => {
            if task.pause {
                println!("Task #{} is PENDING (paused) — skipped", task.id);
                return Ok(());
            }
            if task.conflict {
                let task_model = task.model.clone().or(model);
                let session = CliRoleRunner::new(
                    zbobr,
                    task.id,
                    Role::Merger,
                    task_model,
                    prompts,
                    executor_config,
                );
                session.run().await?;
            } else if task.signal.is_none() {
                println!(
                    "Task #{} is PENDING (no signal, no conflict) — skipped",
                    task.id
                );
                return Ok(());
            } else {
                let signal = task.signal.unwrap();
                let role = signal.target_role();
                let task_model = task.model.clone().or(model);
                let session =
                    CliRoleRunner::new(zbobr, task.id, role, task_model, prompts, executor_config);
                session.run().await?;
            }
        }
        Stage::Preparing
        | Stage::Analysing
        | Stage::Planning
        | Stage::Working
        | Stage::Reviewing
        | Stage::Merging => {
            let role = Role::try_from(task.stage).unwrap();
            let task_model = task.model.clone().or(model);
            let session =
                CliRoleRunner::new(zbobr, task.id, role, task_model, prompts, executor_config);
            session.run().await?;
        }
        Stage::Done => {
            println!("Task #{} is DONE — nothing to process", task.id);
        }
    }
    Ok(())
}

/// Main manager loop: polls for tasks and dispatches role sessions.
pub async fn run_manager_loop(
    zbobr: &ZbobrDispatcherDyn,
    interval_secs: u64,
    cleanup_interval_secs: u64,
    model: Option<Model>,
    prompts: &Prompts,
    executor_config: &ZbobrExecutorConfig,
) -> anyhow::Result<()> {
    let cli_tool = zbobr.config().cli_tool;
    let model = model.unwrap_or_else(|| match cli_tool {
        Tool::Claude => executor_config.claude.default_model.clone(),
        Tool::Copilot => executor_config.copilot.default_model.clone(),
        Tool::McpTester => Model::default(),
    });

    tracing::info!("Manager loop started ({})", zbobr.debug_state());
    tracing::info!("Poll interval: {interval_secs}s, Cleanup interval: {cleanup_interval_secs}s");
    tracing::info!("Default Model: {model}");
    tracing::info!("CLI Tool: {:?}", zbobr.config().cli_tool);
    if let Some(ref base) = prompts.base_path {
        tracing::info!("Prompts base path: {}", base.display());
    }
    tracing::info!(
        "Preparator prompt files: {}",
        prompts
            .preparator
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join("; ")
    );
    tracing::info!(
        "Analyser prompt files: {}",
        prompts
            .analyser
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join("; ")
    );
    tracing::info!(
        "Planner prompt files: {}",
        prompts
            .planner
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join("; ")
    );
    tracing::info!(
        "Worker prompt files: {}",
        prompts
            .worker
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join("; ")
    );
    tracing::info!(
        "Reviewer prompt files: {}",
        prompts
            .reviewer
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join("; ")
    );
    tracing::info!(
        "Merger prompt files: {}",
        prompts
            .merger
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join("; ")
    );

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

        let current_tool = zbobr.config().cli_tool;
        let pending_tasks = match zbobr
            .list_tasks_by_stage(Stage::Pending, Some(current_tool))
            .await
        {
            Ok(tasks) => tasks,
            Err(e) => {
                tracing::error!("Failed to check PENDING tasks: {e}");
                vec![]
            }
        };

        let mut session_run = false;
        for task in pending_tasks {
            if task.pause {
                continue;
            }

            if task.conflict {
                let task_model = task.model.clone().unwrap_or_else(|| model.clone());
                tracing::info!(
                    "Found PENDING task #{} with conflict flag - running merger (tool: {:?}, model: {})",
                    task.id,
                    task.tool,
                    task_model
                );
                let session = CliRoleRunner::new(
                    zbobr,
                    task.id,
                    Role::Merger,
                    Some(task_model),
                    prompts,
                    executor_config,
                );
                if let Err(e) = session.run().await {
                    tracing::error!("Merger session failed: {e}");
                }
                session_run = true;
                break;
            }

            let Some(signal) = task.signal else { continue };
            let role = signal.target_role();
            let task_model = task.model.clone().unwrap_or_else(|| model.clone());
            tracing::info!(
                "Found PENDING task #{} with signal {:?} (tool: {:?}, model: {}) - running {:?}",
                task.id,
                signal,
                task.tool,
                task_model,
                role
            );
            let session = CliRoleRunner::new(
                zbobr,
                task.id,
                role,
                Some(task_model),
                prompts,
                executor_config,
            );
            if let Err(e) = session.run().await {
                tracing::error!("{:?} session failed: {e}", role);
            }
            session_run = true;
            break;
        }

        if session_run {
            continue;
        }

        let active_stages = [
            Stage::Preparing,
            Stage::Planning,
            Stage::Working,
            Stage::Reviewing,
            Stage::Merging,
        ];
        let mut active_counts = std::collections::HashMap::new();
        for stage in &active_stages {
            let count = zbobr
                .list_tasks_by_stage(*stage, Some(current_tool))
                .await
                .unwrap_or_default()
                .len();
            active_counts.insert(stage, count);
        }
        tracing::info!(
            "Task statistics for tool {:?}: PREPARING={}, PLANNING={}, WORKING={}, REVIEWING={}, MERGING={}",
            current_tool,
            active_counts[&Stage::Preparing],
            active_counts[&Stage::Planning],
            active_counts[&Stage::Working],
            active_counts[&Stage::Reviewing],
            active_counts[&Stage::Merging],
        );

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

async fn prepare_workspace(
    zbobr: &ZbobrDispatcherDyn,
    task_id: u64,
    role: Role,
    task_dir: &Path,
) -> anyhow::Result<PathBuf> {
    match role {
        Role::Preparator => Ok(task_dir.to_path_buf()),
        Role::Merger => {
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
            let dest_branch = task
                .parameters
                .get(&Parameter::DestinationBranch)
                .cloned()
                .unwrap_or_else(|| "main".to_string());
            match zbobr
                .clone_and_setup(&dest_repo, &work_branch, &dest_branch, task_id)
                .await
            {
                Ok(path) => Ok(path),
                Err(e) => {
                    let msg = format!("Failed to prepare workspace for task #{task_id}: {e:#}");
                    tracing::error!("{msg}");
                    let hostname = get_hostname();
                    if let Err(post_err) = zbobr
                        .task_session(task_id)
                        .post_comment(CommentType::Error, &msg, None, &hostname, None)
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
        Ok(_) => Ok(()),
        Err(e) => {
            let msg = format!("Could not ensure PR URL for task #{task_id}: {e}");
            tracing::error!("{msg}");
            let hostname = get_hostname();
            let task_session = zbobr.task_session(task_id);
            if let Err(post_err) = task_session
                .post_comment(CommentType::Error, &msg, None, &hostname, None)
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
async fn seed_preparator_defaults(zbobr: &ZbobrDispatcherDyn, task_id: u64) -> anyhow::Result<()> {
    let config = zbobr.config();
    let task = zbobr.get_task(task_id).await?;
    let role_session = zbobr.role_session(task_id);

    if let Some(default_repo) = &config.default_destination_repository
        && !task
            .parameters
            .contains_key(&Parameter::DestinationRepository)
    {
        role_session
            .set_parameter(Parameter::DestinationRepository, Some(default_repo.clone()))
            .await?;
    }

    if let Some(default_branch) = &config.default_destination_branch
        && !task.parameters.contains_key(&Parameter::DestinationBranch)
    {
        role_session
            .set_parameter(Parameter::DestinationBranch, Some(default_branch.clone()))
            .await?;
    }

    Ok(())
}

fn should_try_early_merge(role: Role) -> bool {
    matches!(role, Role::Planner | Role::Worker | Role::Reviewer)
}

async fn try_early_merge(
    zbobr: &ZbobrDispatcherDyn,
    task_id: u64,
    work_dir: &Path,
) -> anyhow::Result<bool> {
    let task = zbobr.get_task(task_id).await?;
    let dest_branch = task
        .parameters
        .get(&Parameter::DestinationBranch)
        .cloned()
        .unwrap_or_else(|| "main".to_string());

    let merge = TokioCommand::new("git")
        .args(["merge", &dest_branch, "--no-edit"])
        .current_dir(work_dir)
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
    let (port_tx, port_rx) = tokio::sync::oneshot::channel();
    let server_handle = tokio::spawn(async move {
        match crate::mcp::run_role_mcp_server(zbobr, role, task_id).await {
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

#[allow(clippy::too_many_arguments)]
async fn execute_tool(
    cli_tool: Tool,
    executor_config: &ZbobrExecutorConfig,
    task_id: u64,
    role: Role,
    model: &Model,
    assigned_port: u16,
    prompt: &str,
    work_dir: &Path,
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
        if role == Role::Worker || role == Role::Merger {
            let role_session = zbobr.task_session(task_id).role_session();
            if let Err(e) = role_session.push_branch_commits().await {
                tracing::warn!("Could not push branch commits for task #{task_id}: {e}");
            }
        }
        task_session.set_stage(Stage::Pending).await?;
        tracing::info!("Session interrupted for task #{task_id}, moved to PENDING");
        return Ok(());
    }

    if let Some(e) = execution_error {
        if role == Role::Worker || role == Role::Merger {
            let role_session = zbobr.task_session(task_id).role_session();
            if let Err(e) = role_session.push_branch_commits().await {
                tracing::warn!("Could not push branch commits for task #{task_id}: {e}");
            }
        }
        let error_msg = format!("Execution failed: {e}");
        let hostname = get_hostname();
        if let Err(post_err) = task_session
            .post_comment(CommentType::Error, &error_msg, None, &hostname, None)
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

    let current_task = zbobr.get_task(task_id).await?;
    let has_unchecked = current_task.checklist.iter().any(|i| !i.checked);
    match role {
        Role::Preparator => {
            if current_task.signal.is_none() && !current_task.pause {
                task_session.set_signal(Some(Signal::GoAnalyse)).await?;
            }
            task_session.set_stage(Stage::Pending).await?;
        }
        Role::Analyser => {
            if current_task.signal.is_none() && !current_task.pause {
                task_session.set_signal(Some(Signal::GoPlan)).await?;
            }
            task_session.set_stage(Stage::Pending).await?;
        }
        Role::Planner => {
            if current_task.signal.is_none() && !current_task.pause {
                task_session.set_signal(Some(Signal::GoWork)).await?;
            }
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
            // Routing is driven by the signal set during the session:
            //   None (review_accept called)  → mark task done
            //   GoPlan (review_reject called) → route back to planner
            //   GoReview (report_error)        → preserved as-is (task paused)
            if !current_task.pause && current_task.signal.is_none() {
                task_session.mark_done().await?;
                return Ok(());
            }
            task_session.set_stage(Stage::Pending).await?;
        }
        Role::Merger => {
            // Conflict was already cleared on entry (Rule 1); just return to Pending.
            // Any signal set before or during the session is preserved per Rule 2.
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

    match TokioCommand::new("git")
        .args(["status", "--porcelain"])
        .current_dir(work_dir)
        .output()
        .await
    {
        Ok(status_output) if status_output.status.success() => {
            let uncommitted = String::from_utf8_lossy(&status_output.stdout)
                .trim()
                .to_string();
            if !uncommitted.is_empty() {
                tracing::info!("Found uncommitted changes, auto-committing...");
                let _ = TokioCommand::new("git")
                    .args(["add", "."])
                    .current_dir(work_dir)
                    .status()
                    .await;
                let commit_msg = format!("Auto-commit by {} agent", role.as_str());
                match TokioCommand::new("git")
                    .args(["commit", "-m", &commit_msg])
                    .current_dir(work_dir)
                    .status()
                    .await
                {
                    Ok(s) if s.success() => tracing::info!("Auto-commit successful"),
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
        rewrite_commit_authors(zbobr, task_id, work_dir, false).await?;
    }

    Ok(())
}

async fn rewrite_commit_authors(
    zbobr: &ZbobrDispatcherDyn,
    task_id: u64,
    work_dir: &PathBuf,
    dry_run: bool,
) -> anyhow::Result<()> {
    let task = zbobr.get_task(task_id).await?;
    let dest_branch = task
        .parameters
        .get(&Parameter::DestinationBranch)
        .cloned()
        .unwrap_or_else(|| "main".to_string());

    let git_user_name = &zbobr.config().git_user_name;
    let git_user_email = &zbobr.config().git_user_email;

    // Get absolute path to the git repository
    let git_root_cmd = TokioCommand::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(work_dir)
        .output()
        .await;

    let git_root = match git_root_cmd {
        Ok(output) => {
            if output.status.success() {
                String::from_utf8_lossy(&output.stdout).trim().to_string()
            } else {
                return Err(anyhow::anyhow!(
                    "Failed to determine git repository root: {}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
        }
        Err(e) => {
            return Err(anyhow::anyhow!("Error executing git rev-parse: {}", e));
        }
    };

    let git_root_path = std::path::PathBuf::from(&git_root);

    // Get list of commits that will be rewritten
    let log_cmd = TokioCommand::new("git")
        .args([
            "log",
            &format!("{}..HEAD", dest_branch),
            "--format=%H %an <%ae>",
        ])
        .current_dir(&git_root_path)
        .output()
        .await;

    let commits_to_rewrite = match log_cmd {
        Ok(output) => {
            if output.status.success() {
                String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
            } else {
                return Err(anyhow::anyhow!(
                    "Failed to list commits: {}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
        }
        Err(e) => {
            return Err(anyhow::anyhow!("Error executing git log: {}", e));
        }
    };

    println!("Commits to be rewritten ({}):", commits_to_rewrite.len());
    for commit in &commits_to_rewrite {
        println!("  {}", commit);
    }

    if dry_run {
        println!(
            "\n[DRY-RUN] Skipping actual rebase. These commits would be rewritten with author: {} <{}>",
            git_user_name, git_user_email
        );
        return Ok(());
    }

    let config_user = TokioCommand::new("git")
        .args(["config", "--local", "user.name", git_user_name])
        .current_dir(&git_root_path)
        .output()
        .await;
    let config_email = TokioCommand::new("git")
        .args(["config", "--local", "user.email", git_user_email])
        .current_dir(&git_root_path)
        .output()
        .await;

    if let (Ok(user_out), Ok(email_out)) = (config_user, config_email) {
        if !user_out.status.success() || !email_out.status.success() {
            return Err(anyhow::anyhow!(
                "Failed to set up git config for author rewriting"
            ));
        }

        let rebase_cmd = format!(
            "git rebase --exec 'git commit --amend --no-edit --reset-author' '{}'",
            dest_branch
        );
        let rebase_output = TokioCommand::new("sh")
            .arg("-c")
            .arg(&rebase_cmd)
            .env("GIT_AUTHOR_NAME", git_user_name)
            .env("GIT_AUTHOR_EMAIL", git_user_email)
            .env("GIT_COMMITTER_NAME", git_user_name)
            .env("GIT_COMMITTER_EMAIL", git_user_email)
            .current_dir(&git_root_path)
            .output()
            .await;

        match rebase_output {
            Ok(output) if output.status.success() => {
                println!("Successfully rewrote commit authors");

                // Show post-rebase log to verify changes
                let post_log_cmd = TokioCommand::new("git")
                    .args([
                        "log",
                        &format!("{}..HEAD", dest_branch),
                        "--format=%H %an <%ae>",
                    ])
                    .current_dir(&git_root_path)
                    .output()
                    .await;

                if let Ok(log_output) = post_log_cmd
                    && log_output.status.success()
                {
                    let updated_commits = String::from_utf8_lossy(&log_output.stdout);
                    println!("Updated commits:");
                    for commit in updated_commits.lines() {
                        println!("  {}", commit);
                    }
                }

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
                return Err(anyhow::anyhow!(
                    "Failed to rewrite commit authors: {}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "Error running git rebase for author rewriting: {}",
                    e
                ));
            }
        }
    } else {
        return Err(anyhow::anyhow!(
            "Error executing git config commands for author rewriting"
        ));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// run_zbobr
// ---------------------------------------------------------------------------

/// Standard entry point for a Zbobr CLI application, heavily parameterized
/// to allow for different backends.
use zbobr_api::{CommentTag, config::BackendConfig};

pub async fn run_zbobr<TC: BackendConfig + 'static, RC: BackendConfig + 'static>(
    app_name: &'static str,
    app_about: &'static str,
    app_long_about: &'static str,
    default_config_name: &'static str,
) -> anyhow::Result<()>
where
    TC::Backend: crate::backend::TaskBackend + 'static,
    RC::Backend: crate::backend::RepoBackend + 'static,
    TC::Args: zbobr_utility::PrefixedArgs + std::fmt::Debug + Clone,
    RC::Args: zbobr_utility::PrefixedArgs + std::fmt::Debug + Clone,
{
    let cli: GenericCli<TC::Args, RC::Args> = parse_cli(app_name, app_about, app_long_about);

    let config_path = cli
        .config_file
        .path
        .clone()
        .unwrap_or_else(|| default_config_name.into());

    let config_dir = if cli.config_file.path.is_some() {
        std::fs::canonicalize(&config_path)
            .context(format!(
                "Cannot resolve config path: {}",
                config_path.display()
            ))?
            .parent()
            .expect("config file must have a parent directory")
            .to_path_buf()
    } else {
        std::env::current_dir()?
    };

    let root_toml = crate::GenericConfigToml::<TC, RC>::load(&config_path)
        .with_context(|| format!("Config file: {}", config_path.display()))?;
    let config =
        crate::GenericConfig::<TC, RC>::build(root_toml, cli.settings.clone(), &config_dir)
            .with_context(|| format!("Config file: {}", config_path.display()))?;
    config
        .dispatcher
        .validate()
        .with_context(|| format!("Config file: {}", config_path.display()))?;
    let executor_config = config.executor.clone();

    let task_backend: std::sync::Arc<dyn crate::backend::TaskBackend> =
        std::sync::Arc::new(config.tasks.build_backend(&config.dispatcher)?);
    let repo_backend: std::sync::Arc<dyn crate::backend::RepoBackend> =
        std::sync::Arc::new(config.repo.build_backend(&config.dispatcher)?);

    let zbobr = crate::ZbobrDispatcher::new_with_backends(
        config.dispatcher.clone(),
        task_backend,
        repo_backend,
    );
    zbobr.validate_connectivity().await?;

    let prompts = crate::prompts::resolve_prompts(&cli.settings.dispatcher, zbobr.config());

    run_command(zbobr, cli.command, &prompts, &executor_config).await
}
