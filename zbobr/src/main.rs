#![allow(clippy::needless_borrows_for_generic_args)]
use std::path::PathBuf;

use anyhow::Context;
use clap::{Args, CommandFactory, Parser, Subcommand};
use zbobr_dispatcher::{
    Stage, Zbobr, ZbobrConfig, ZbobrConfigArgs, ZbobrConfigToml, ZbobrExecutorConfig,
    task::{Model, Parameter, Role, Tool},
};
use zbobr_executor_mcp_tester::ZbobrExecutorMcpTesterConfig;

mod role_session;
mod prompts;

use crate::role_session::RoleSession;
use crate::prompts::{Prompts, resolve_prompts};

#[derive(Args, Clone)]
struct GlobalArgs {
    #[command(
        flatten,
        next_help_heading = "[config] Meta options and config file overrides"
    )]
    config_file: ConfigFileArg,

    #[command(flatten)]
    settings: ZbobrConfigArgs,
}

#[derive(Args, Clone)]
struct ConfigFileArg {
    /// Path to TOML configuration file (default: zbobr.toml in cwd)
    #[arg(long = "config")]
    pub path: Option<PathBuf>,
}

#[derive(Parser)]
#[command(
    name = "zbobr",
    about = "AI-powered task dispatcher",
    long_about = "AI-powered task dispatcher that manages tasks through automated stages.\n\n\
        Tasks flow through: PENDING -> PREPARING -> PLANNING -> WORKING -> REVIEWING -> DONE.\n\
        Merge conflicts are handled by MERGING sessions when the conflict flag is set.\n\
        Preparator roles set parameters, planner roles create implementation plans, worker roles implement them\n\
        by forking target repositories and creating pull requests, reviewer roles review the changes,\n\
        and merger roles resolve any merge conflicts.\n\n\
        Requires a GitHub token: set GH_TOKEN or GITHUB_TOKEN env var.\n\
        Easiest way: export GH_TOKEN=$(gh auth token)"
)]
struct Cli {
    #[command(flatten)]
    global: GlobalArgs,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Initialize a task project: create repo if needed, set up stages and labels
    Setup {
        /// Force overwrite existing labels
        #[arg(long, short = 'f')]
        force: bool,
    },
    /// Poll for tasks and run planner/worker roles automatically
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

#[derive(Subcommand)]
enum TaskSubcommand {
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
        /// Set or clear the confirm flag (true/false).  When set a stage change will
        /// automatically pause the task.
        #[arg(long)]
        confirm: Option<bool>,
    },
    /// Delete (close) a task by ID
    Delete {
        /// Task ID
        id: u64,
    },
    /// Clone the task's destination repository into its workspace using the configured repo backend
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
}



/// Print a task to stdout in a human-readable format.
fn print_task(task: &zbobr_dispatcher::Task, discussion: &[String]) {
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
    if !task.plan.is_empty() {
        println!("Plan:\n{}", task.plan);
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
        for (i, msg) in discussion.iter().enumerate() {
            println!("  [{}] {}", i + 1, msg);
        }
    }
}

/// Load root TOML config based on CLI args.
/// If --config is specified, load that file (error if missing).
/// Otherwise, try zbobr.toml in cwd (silently skip if missing).
fn load_root_toml(cli: &Cli) -> anyhow::Result<Option<ZbobrConfigToml>> {
    if let Some(ref path) = cli.global.config_file.path {
        let root = ZbobrConfigToml::load(path)?
            .ok_or_else(|| anyhow::anyhow!("Config file not found: {}", path.display()))?;
        Ok(Some(root))
    } else {
        let default_path = std::env::current_dir()?.join("zbobr.toml");
        ZbobrConfigToml::load(&default_path)
    }
}

/// Parse CLI, allowing global options both before and after the subcommand.
///
/// Global options are defined without `global = true` so they only appear in
/// `zbobr --help`, not in subcommand help. To still accept them after the
/// subcommand (e.g. `zbobr setup --config foo.toml`), we reorder raw args
/// to move recognized global flags before the subcommand before parsing.
fn parse_cli() -> Cli {
    let cmd = Cli::command();

    // Collect known subcommand names
    let subcommands: std::collections::HashSet<String> = cmd
        .get_subcommands()
        .map(|s| s.get_name().to_owned())
        .collect();

    // Collect global arg long names (with -- prefix) and whether they take a value
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
    let mut before_sub = vec![raw_args[0].clone()]; // binary name
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

        // After subcommand: check if this is a global flag to hoist
        // Handle --flag=value syntax
        let base = arg.split('=').next().unwrap_or(arg);
        if let Some(&takes_value) = global_flags.get(base) {
            if arg.contains('=') {
                // --flag=value: move entire arg
                before_sub.push(arg.clone());
                i += 1;
            } else if takes_value && i + 1 < raw_args.len() {
                // --flag value: move both
                before_sub.push(arg.clone());
                before_sub.push(raw_args[i + 1].clone());
                i += 2;
            } else {
                // boolean flag or no value
                before_sub.push(arg.clone());
                i += 1;
            }
        } else {
            sub_and_after.push(arg.clone());
            i += 1;
        }
    }

    before_sub.extend(sub_and_after);
    Cli::parse_from(before_sub)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize rustls crypto provider before any TLS operations
    let _ = rustls::crypto::ring::default_provider().install_default();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let cli = parse_cli();
    let root_toml = load_root_toml(&cli)?;

    // Compute the directory containing the config file.
    // All relative paths in zbobr.toml are resolved relative to this directory.
    let config_dir = match cli.global.config_file.path {
        Some(ref path) => std::fs::canonicalize(path)
            .with_context(|| format!("Cannot resolve config path: {}", path.display()))?
            .parent()
            .expect("config file must have a parent directory")
            .to_path_buf(),
        None => std::env::current_dir()?,
    };

    let zbobr_config = ZbobrConfig::build(root_toml, cli.global.settings.clone(), &config_dir)?;
    zbobr_config.dispatcher.validate()?;
    let executor_config = zbobr_config.executor.clone();
    let zbobr = Zbobr::new(zbobr_config)?;
    zbobr.validate_connectivity().await?;
    let prompts = resolve_prompts(&cli, zbobr.config())?;

    match cli.command {
        Command::Setup { force } => {
            zbobr.setup(force).await?;
        }
        Command::Cleanup { dry_run } => {
            zbobr.cleanup_closed_tasks(dry_run).await?;
        }
        Command::Task { subcommand } => match subcommand {
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
                    .map(|t| t.parse::<zbobr_dispatcher::Tool>())
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
                let stage_filter: Option<Stage> = if let Some(s) = stage {
                    Some(
                        Stage::from_milestone_name(&s.to_uppercase())
                            .ok_or_else(|| anyhow::anyhow!("Invalid stage: {}", s))?,
                    )
                } else {
                    None
                };
                let tool_filter: Option<zbobr_dispatcher::Tool> = if let Some(t) = tool {
                    Some(t.parse::<zbobr_dispatcher::Tool>()?)
                } else {
                    None
                };

                let mut tasks = Vec::new();
                if let Some(stage) = stage_filter {
                    tasks = zbobr.list_tasks_by_stage(stage, tool_filter).await?;
                } else {
                    // iterate all stages if no specific stage provided
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
                    // sort by ID for deterministic order
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
                    .map(|t| t.parse::<zbobr_dispatcher::Tool>().context("Invalid tool"))
                    .transpose()?;
                let model = model
                    .map(|m| m.parse::<Model>().context("Invalid model"))
                    .transpose()?;
                let signal = signal
                    .map(|s| {
                        s.parse::<zbobr_dispatcher::Signal>()
                            .context("Invalid signal")
                    })
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
                                        task.parameters
                                            .insert(Parameter::DestinationBranch, branch);
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
                            if let Some(c) = confirm {
                                task.confirm = c;
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
                let model_enum = model
                    .map(|m| m.parse::<Model>())
                    .transpose()
                    .context("Invalid model name")?;
                let session = RoleSession::new(
                    &zbobr,
                    task,
                    Role::Preparator,
                    model_enum,
                    &prompts,
                    &executor_config,
                );
                if show_prompt {
                    println!("{}", session.prompt()?);
                } else {
                    session.run().await?;
                }
            }
            TaskSubcommand::Plan {
                task,
                model,
                show_prompt,
            } => {
                let model_enum = model
                    .map(|m| m.parse::<Model>())
                    .transpose()
                    .context("Invalid model name")?;
                let session = RoleSession::new(
                    &zbobr,
                    task,
                    Role::Planner,
                    model_enum,
                    &prompts,
                    &executor_config,
                );
                if show_prompt {
                    println!("{}", session.prompt()?);
                } else {
                    session.run().await?;
                }
            }
            TaskSubcommand::Work {
                task,
                model,
                show_prompt,
            } => {
                let model_enum = model
                    .map(|m| m.parse::<Model>())
                    .transpose()
                    .context("Invalid model name")?;
                let session = RoleSession::new(
                    &zbobr,
                    task,
                    Role::Worker,
                    model_enum,
                    &prompts,
                    &executor_config,
                );
                if show_prompt {
                    println!("{}", session.prompt()?);
                } else {
                    session.run().await?;
                }
            }
            TaskSubcommand::Review {
                task,
                model,
                show_prompt,
            } => {
                let model_enum = model
                    .map(|m| m.parse::<Model>())
                    .transpose()
                    .context("Invalid model name")?;
                let session = RoleSession::new(
                    &zbobr,
                    task,
                    Role::Reviewer,
                    model_enum,
                    &prompts,
                    &executor_config,
                );
                if show_prompt {
                    println!("{}", session.prompt()?);
                } else {
                    session.run().await?;
                }
            }
            TaskSubcommand::Merge {
                task,
                model,
                show_prompt,
            } => {
                let model_enum = model
                    .map(|m| m.parse::<Model>())
                    .transpose()
                    .context("Invalid model name")?;
                let session = RoleSession::new(
                    &zbobr,
                    task,
                    Role::Merger,
                    model_enum,
                    &prompts,
                    &executor_config,
                );
                if show_prompt {
                    println!("{}", session.prompt()?);
                } else {
                    session.run().await?;
                }
            }
            TaskSubcommand::Process {
                task,
                model,
                executor_mcp_tester_preparation,
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
                    || executor_mcp_tester_planning.is_some()
                    || executor_mcp_tester_working.is_some()
                    || executor_mcp_tester_reviewing.is_some()
                    || executor_mcp_tester_merging.is_some()
                {
                    Some(ZbobrExecutorMcpTesterConfig {
                        preparation: executor_mcp_tester_preparation,
                        planning: executor_mcp_tester_planning,
                        working: executor_mcp_tester_working,
                        reviewing: executor_mcp_tester_reviewing,
                        merging: executor_mcp_tester_merging,
                    })
                } else {
                    None
                };
                let effective_executor_config = match mcp_tester_config_override {
                    Some(mcp_tester) => ZbobrExecutorConfig { mcp_tester, ..executor_config.clone() },
                    None => executor_config.clone(),
                };
                process_task_by_stage(
                    &zbobr,
                    &task_obj,
                    model_enum,
                    &prompts,
                    &effective_executor_config,
                )
                .await?;
            }
        },
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
                &prompts,
                &executor_config,
            )
            .await?;
        }
    }

    Ok(())
}

/// Start MCP server, invoke CLI tool (copilot/claude/stub), and handle stage transitions.
// The role session logic used to live in this file but has been refactored
    // into `role_session.rs`.  The helpers there handle the bulk of the work;
    // `main.rs` merely constructs `RoleSession` instances and drives them while
    // keeping the CLI-focused code uncluttered.



async fn process_task_by_stage(
    zbobr: &Zbobr,
    task: &zbobr_dispatcher::Task,
    model: Option<Model>,
    prompts: &Prompts,
    executor_config: &ZbobrExecutorConfig,
) -> anyhow::Result<()> {
    match task.stage {
        Stage::Pending => {
            // Follow transitions.dot: pause check → conflict check → signal routing.
            // Conflict check must come before the signal-None check: when a Reviewer/Worker
            // detects a merge conflict it clears the signal and sets conflict=true, so the
            // Merger must be dispatched even without a signal present.
            if task.pause {
                println!("Task #{} is PENDING (paused) — skipped", task.id);
                return Ok(());
            }
            if task.conflict {
                // Conflict flag set → run merger
                let task_model = task.model.clone().or(model);
                let session = RoleSession::new(
                    zbobr,
                    task.id,
                    Role::Merger,
                    task_model,
                    &prompts,
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
                let session = RoleSession::new(
                    zbobr,
                    task.id,
                    role,
                    task_model,
                    &prompts,
                    executor_config,
                );
                session.run().await?;
            }
        }
        Stage::Preparing | Stage::Planning | Stage::Working | Stage::Reviewing | Stage::Merging => {
            // convert the current stage to the corresponding role using TryFrom;
            // the preceding `if` ensures the value is one of the mapped stages so
            // `unwrap()` is safe.
            let role = Role::try_from(task.stage).unwrap();
            let task_model = task.model.clone().or(model);
            let session = RoleSession::new(
                zbobr,
                task.id,
                role,
                task_model,
                &prompts,
                executor_config,
            );
            session.run().await?;
        }
        Stage::Done => {
            println!("Task #{} is DONE — nothing to process", task.id);
        }
    }

    Ok(())
}

/// Main manager loop: polls for tasks and spawns sessions.
async fn run_manager_loop(
    zbobr: &Zbobr,
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

    // prompt text is prepared inside the role-session module; callers simply
    // pass along the resolved prompt file paths via the `prompts` argument.

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
    tracing::info!(
        "Task backend: {:?}, Repo backend: {:?}",
        zbobr.config().task_backend,
        zbobr.config().repo_backend
    );

    let mut last_cleanup = std::time::Instant::now();

    loop {
        let loop_start = std::time::Instant::now();

        // Run cleanup if interval has passed
        if last_cleanup.elapsed().as_secs() >= cleanup_interval_secs {
            tracing::info!("Running workspaces cleanup...");
            if let Err(e) = zbobr.cleanup_closed_tasks(false).await {
                tracing::warn!("Cleanup failed: {e}");
            }
            last_cleanup = std::time::Instant::now();
        }

        // Check for processable tasks using tool-based filtering
        let current_tool = zbobr.config().cli_tool;

        // Implement transitions.dot flow for PENDING tasks:
        // 1. if pause==true or signal==None → skip
        // 2. if conflict==true → run Merger session
        // 3. if signal==GoPrepare → run Preparator session (no git pull)
        // 4. otherwise (GoPlan/GoWork/GoReview) → run session for signal's role
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
            // Step 1: skip if paused
            if task.pause {
                continue;
            }

            // Step 2: conflict flag → run merger (even without a signal,
            // since the signal is cleared when conflict is first detected)
            if task.conflict {
                let task_model = task.model.clone().unwrap_or_else(|| model.clone());
                tracing::info!(
                    "Found PENDING task #{} with conflict flag - running merger (tool: {:?}, model: {})",
                    task.id,
                    task.tool,
                    task_model
                );
                let session = RoleSession::new(
                    zbobr,
                    task.id,
                    Role::Merger,
                    Some(task_model),
                    &prompts,
                    executor_config,
                );
                if let Err(e) = session.run().await {
                    tracing::error!("Merger session failed: {e}");
                }
                session_run = true;
                break;
            }

            // Step 3/4: route by signal (skip if no signal and no conflict)
            let Some(signal) = task.signal else {
                continue;
            };
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
            let session = RoleSession::new(
                zbobr,
                task.id,
                role,
                Some(task_model),
                &prompts,
                executor_config,
            );
            if let Err(e) = session.run().await {
                tracing::error!("{:?} session failed: {e}", role);
            }
            session_run = true;
            break; // Only run one session per loop iteration
        }

        if session_run {
            continue;
        }

        // Log task statistics before sleeping
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
            active_counts[&Stage::Merging]
        );

        // Sleep for the remainder of the interval, accounting for time already spent
        // in this iteration (e.g. running a session). If elapsed >= interval, skip sleep.
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

// Unit tests below
#[cfg(test)]
mod tests {
    use super::*;

    /// Constructing the clap command panics when there are duplicate argument
    /// names.  The original bug involved multiple `default_model` fields being
    /// flattened into `GlobalArgs` (dispatcher + executor configs).
    #[test]
    fn cli_command_builds_without_duplicates() {
        // clap validation runs during command construction; any duplicates
        // will trigger a panic and therefore fail this test.
        let _ = Cli::command();
    }

    #[test]
    fn task_list_parsing() {
        let cli = Cli::parse_from([
            "zbobr", "task", "list", "--stage", "pending", "--tool", "claude",
        ]);
        if let Command::Task { subcommand } = cli.command {
            match subcommand {
                TaskSubcommand::List { stage, tool } => {
                    assert_eq!(stage.as_deref(), Some("pending"));
                    assert_eq!(tool.as_deref(), Some("claude"));
                }
                _ => panic!("expected List subcommand"),
            }
        } else {
            panic!("expected Task command");
        }
    }

    #[test]
    fn task_list_empty_filters() {
        let cli = Cli::parse_from(["zbobr", "task", "list"]);
        if let Command::Task { subcommand } = cli.command {
            match subcommand {
                TaskSubcommand::List { stage, tool } => {
                    assert!(stage.is_none());
                    assert!(tool.is_none());
                }
                _ => panic!("expected List subcommand"),
            }
        } else {
            panic!("expected Task command");
        }
    }

    #[test]
    fn task_update_parameter_set_parsing() {
        let cli = Cli::parse_from([
            "zbobr",
            "task",
            "update",
            "1",
            "--dest-repo",
            "owner/repo",
            "--dest-branch",
            "main",
            "--work-branch",
            "zbobr_fix-1-test",
        ]);

        if let Command::Task { subcommand } = cli.command {
            match subcommand {
                TaskSubcommand::Update {
                    dest_repo,
                    dest_branch,
                    work_branch,
                    ..
                } => {
                    assert_eq!(dest_repo, Some(Some("owner/repo".to_string())));
                    assert_eq!(dest_branch, Some(Some("main".to_string())));
                    assert_eq!(work_branch, Some(Some("zbobr_fix-1-test".to_string())));
                }
                _ => panic!("expected Update subcommand"),
            }
        } else {
            panic!("expected Task command");
        }
    }

    #[test]
    fn task_update_parameter_delete_parsing() {
        let cli = Cli::parse_from([
            "zbobr",
            "task",
            "update",
            "1",
            "--dest-repo",
            "--dest-branch",
            "--work-branch",
        ]);

        if let Command::Task { subcommand } = cli.command {
            match subcommand {
                TaskSubcommand::Update {
                    dest_repo,
                    dest_branch,
                    work_branch,
                    ..
                } => {
                    assert_eq!(dest_repo, Some(None));
                    assert_eq!(dest_branch, Some(None));
                    assert_eq!(work_branch, Some(None));
                }
                _ => panic!("expected Update subcommand"),
            }
        } else {
            panic!("expected Task command");
        }
    }

    #[test]
    fn task_create_confirm_parsing() {
        let cli = Cli::parse_from(["zbobr", "task", "create", "foo", "--confirm"]);
        if let Command::Task { subcommand } = cli.command {
            match subcommand {
                TaskSubcommand::Create { confirm, .. } => {
                    assert!(confirm);
                }
                _ => panic!("expected Create subcommand"),
            }
        } else {
            panic!("expected Task command");
        }
    }

    #[test]
    fn task_update_confirm_parsing() {
        let cli = Cli::parse_from(["zbobr", "task", "update", "1", "--confirm", "false"]);
        if let Command::Task { subcommand } = cli.command {
            match subcommand {
                TaskSubcommand::Update { confirm, .. } => {
                    assert_eq!(confirm, Some(false));
                }
                _ => panic!("expected Update subcommand"),
            }
        } else {
            panic!("expected Task command");
        }
    }

    #[test]
    fn task_process_parsing() {
        let cli = Cli::parse_from(["zbobr", "task", "process", "42", "--model", "gpt-5-mini"]);

        if let Command::Task { subcommand } = cli.command {
            match subcommand {
                TaskSubcommand::Process {
                    task,
                    model,
                    executor_mcp_tester_preparation: _,
                    executor_mcp_tester_planning: _,
                    executor_mcp_tester_working: _,
                    executor_mcp_tester_reviewing: _,
                    executor_mcp_tester_merging: _,
                } => {
                    assert_eq!(task, 42);
                    assert_eq!(model.as_deref(), Some("gpt-5-mini"));
                }
                _ => panic!("expected Process subcommand"),
            }
        } else {
            panic!("expected Task command");
        }
    }
}
