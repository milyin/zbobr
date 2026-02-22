#![allow(clippy::needless_borrows_for_generic_args)]
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Context;
use clap::{Args, CommandFactory, Parser, Subcommand};
use zbobr_config::{ZbobrConfigArgs, ZbobrConfigToml};
use zbobr_dispatcher::{
    Stage, ToolExecutor, Zbobr, ZbobrDispatcherConfig,
    task::{Model, Role, Tool},
};
use zbobr_executor_claude::{ClaudeExecutor, ZbobrExecutorClaudeConfig};
use zbobr_executor_copilot::{CopilotExecutor, ZbobrExecutorCopilotConfig};
use zbobr_executor_mcp_tester::{McpTesterExecutor, ZbobrExecutorMcpTesterConfig};
use zbobr_repo_backend_fs::FilesystemRepoBackend;
use zbobr_repo_backend_github::GitHubRepoBackend;
use zbobr_task_backend_fs::FilesystemTaskBackend;
use zbobr_task_backend_github::GitHubTaskBackend;

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
        Tasks flow through: PENDING -> GO_PREPARATION -> PREPARATION -> GO_PLANNING -> PLANNING -> GO_WORKING -> WORKING -> GO_REVIEWING -> REVIEWING -> GO_MERGING -> MERGING.\n\
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
        /// Port for the MCP server that roles connect to
        #[arg(long, default_value = "3000")]
        port: u16,
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
        /// AI model to use (e.g. "gpt-5-mini", "claude-3-5-sonnet")
        #[arg(long)]
        model: Option<String>,
        /// Port for the MCP server that the agent connects to
        #[arg(long, default_value = "3000")]
        port: u16,
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
        /// Port for the MCP server that the agent connects to
        #[arg(long, default_value = "3000")]
        port: u16,
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
        /// Port for the MCP server that the agent connects to
        #[arg(long, default_value = "3000")]
        port: u16,
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
        /// Port for the MCP server that the agent connects to
        #[arg(long, default_value = "3000")]
        port: u16,
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
        /// Port for the MCP server that the agent connects to
        #[arg(long, default_value = "3000")]
        port: u16,
        /// Show the prompt that would be sent to the model instead of running
        #[arg(long)]
        show_prompt: bool,
    },
}

/// Resolved prompt file paths for planner, worker, and merger.
struct Prompts {
    base_path: Option<PathBuf>,
    preparator: Vec<PathBuf>,
    planner: Vec<PathBuf>,
    worker: Vec<PathBuf>,
    reviewer: Vec<PathBuf>,
    merger: Vec<PathBuf>,
}

/// Resolve prompt paths: CLI arg > config values.
/// Paths are resolved relative to prompts_path if provided, otherwise relative to current directory.
fn resolve_prompts(cli: &Cli, config: &ZbobrDispatcherConfig) -> anyhow::Result<Prompts> {
    // Use CLI args if provided, otherwise use config (which came from TOML/env/defaults)
    let planner = cli
        .global
        .settings
        .dispatcher
        .planner_prompts
        .clone()
        .unwrap_or_else(|| config.planner_prompts.clone());

    let preparator = cli
        .global
        .settings
        .dispatcher
        .preparator_prompts
        .clone()
        .unwrap_or_else(|| config.preparator_prompts.clone());

    let worker = cli
        .global
        .settings
        .dispatcher
        .worker_prompts
        .clone()
        .unwrap_or_else(|| config.worker_prompts.clone());

    let reviewer = cli
        .global
        .settings
        .dispatcher
        .reviewer_prompts
        .clone()
        .unwrap_or_else(|| config.reviewer_prompts.clone());

    let merger = config.merger_prompts.clone();

    // CLI prompts_path > config.prompts_path (which came from TOML/env)
    let base_path = cli
        .global
        .settings
        .dispatcher
        .prompts_path
        .clone()
        .or_else(|| config.prompts_path.clone());

    Ok(Prompts {
        base_path,
        preparator,
        planner,
        worker,
        reviewer,
        merger,
    })
}

/// Load and concatenate multiple prompt files (additional user context).
/// If base_path is provided, relative paths are resolved relative to it.
/// Otherwise, relative paths are resolved relative to the current directory.
/// Missing files are silently skipped (they are optional additional context).
fn load_prompts(paths: &[PathBuf], base_path: Option<&PathBuf>) -> anyhow::Result<String> {
    let mut combined = String::new();
    for path in paths.iter() {
        // Resolve path relative to base_path if provided and path is relative
        let resolved_path = if let Some(base) = base_path {
            if path.is_relative() {
                base.join(path)
            } else {
                path.clone()
            }
        } else if path.is_relative() {
            std::env::current_dir()?.join(path)
        } else {
            path.clone()
        };

        let content = match std::fs::read_to_string(&resolved_path) {
            Ok(c) => c,
            Err(_) => {
                tracing::debug!(
                    "Prompt file not found, skipping: {}",
                    resolved_path.display()
                );
                continue;
            }
        };

        let trimmed = content.trim();
        if trimmed.is_empty() {
            continue;
        }

        if !combined.is_empty() {
            combined.push_str("\n\n");
        }
        combined.push_str(trimmed);
    }
    Ok(combined)
}

/// Build full prompt: hardcoded instructions + user context files + auto-generated API docs.
fn build_full_prompt(user_context: &str, role: Role) -> String {
    let hardcoded = match role {
        Role::Preparator => zbobr_dispatcher::preparator_instructions(),
        Role::Planner => zbobr_dispatcher::planner_instructions(),
        Role::Worker => zbobr_dispatcher::worker_instructions(),
        Role::Reviewer => zbobr_dispatcher::reviewer_instructions(),
        Role::Merger => zbobr_dispatcher::merger_instructions(),
    };

    let api_docs = match role {
        Role::Preparator => zbobr_dispatcher::PreparatorMcp::generate_api_docs(),
        Role::Planner => zbobr_dispatcher::PlannerMcp::generate_api_docs(),
        Role::Worker => zbobr_dispatcher::WorkerMcp::generate_api_docs(),
        Role::Reviewer => zbobr_dispatcher::ReviewerMcp::generate_api_docs(),
        Role::Merger => zbobr_dispatcher::MergerMcp::generate_api_docs(),
    };

    if user_context.is_empty() {
        format!("{}\n\n---\n\n{}", hardcoded, api_docs)
    } else {
        format!(
            "{}\n\n---\n\n{}\n\n---\n\n{}",
            hardcoded, user_context, api_docs
        )
    }
}

/// Print a task to stdout in a human-readable format.
fn print_task(task: &zbobr_dispatcher::Task) {
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
    println!("Done:        {}", task.done);
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
    if !task.discussion.is_empty() {
        println!("Discussion ({} comment(s))", task.discussion.len());
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

fn load_config(
    cli: &Cli,
    root_toml: &Option<ZbobrConfigToml>,
    config_dir: &Path,
) -> anyhow::Result<ZbobrDispatcherConfig> {
    // Build dispatcher config
    let dispatcher_toml = root_toml.as_ref().and_then(|r| r.dispatcher.as_ref());
    let mut config = ZbobrDispatcherConfig::build(
        dispatcher_toml.cloned(),
        cli.global.settings.dispatcher.clone(),
        config_dir,
    )?;

    // CLI arg overrides (highest priority)
    if let Some(ref ws) = cli.global.settings.dispatcher.workspaces {
        config.workspaces = ws.clone();
    }
    if let Some(ref b) = cli.global.settings.dispatcher.backend {
        config.backend = *b;
    }
    if let Some(ref t) = cli.global.settings.dispatcher.cli_tool {
        config.cli_tool = *t;
    }
    config.validate()?;

    Ok(config)
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

    let config = load_config(&cli, &root_toml, &config_dir)?;
    let task_backend_github_toml = root_toml
        .as_ref()
        .and_then(|r| r.tasks.as_ref())
        .and_then(|t| t.github.as_ref());
    let task_backend_fs_toml = root_toml
        .as_ref()
        .and_then(|r| r.tasks.as_ref())
        .and_then(|t| t.fs.as_ref());
    let repo_backend_github_toml = root_toml
        .as_ref()
        .and_then(|r| r.repo.as_ref())
        .and_then(|r| r.github.as_ref());
    let repo_backend_fs_toml = root_toml
        .as_ref()
        .and_then(|r| r.repo.as_ref())
        .and_then(|r| r.fs.as_ref());

    let executor_toml = root_toml.as_ref().and_then(|r| r.executor.as_ref());
    let claude_executor_config = ZbobrExecutorClaudeConfig::build(
        executor_toml.and_then(|e| e.claude.clone()),
        cli.global.settings.executor.claude.clone(),
    );
    let copilot_executor_config = ZbobrExecutorCopilotConfig::build(
        executor_toml.and_then(|e| e.copilot.clone()),
        cli.global.settings.executor.copilot.clone(),
    );
    let mcp_tester_executor_config = ZbobrExecutorMcpTesterConfig::build(
        executor_toml.and_then(|e| e.mcp_tester.clone()),
        cli.global.settings.executor.mcp_tester.clone(),
        &config_dir,
    );

    let task_backend: Arc<dyn zbobr_dispatcher::backend::TaskBackend> = match config.backend {
        zbobr_dispatcher::config::BackendType::GitHub => Arc::new(
            GitHubTaskBackend::new(
                task_backend_github_toml.cloned(),
                cli.global.settings.tasks.github.clone(),
            )
            .context("Failed to create GitHub task backend")?,
        ),
        zbobr_dispatcher::config::BackendType::Filesystem => Arc::new(
            FilesystemTaskBackend::new(
                task_backend_fs_toml.cloned(),
                cli.global.settings.tasks.fs.clone(),
                &config_dir,
            )
            .context("Failed to create filesystem task backend")?,
        ),
    };
    let repo_backend: Arc<dyn zbobr_dispatcher::backend::RepoBackend> = match config.backend {
        zbobr_dispatcher::config::BackendType::GitHub => Arc::new(
            GitHubRepoBackend::new(
                repo_backend_github_toml.cloned(),
                cli.global.settings.repo.github.clone(),
            )
            .context("Failed to create GitHub repo backend")?,
        ),
        zbobr_dispatcher::config::BackendType::Filesystem => Arc::new(
            FilesystemRepoBackend::new(
                repo_backend_fs_toml.cloned(),
                cli.global.settings.repo.fs.clone(),
                &config_dir,
            )
            .context("Failed to create filesystem repo backend")?,
        ),
    };
    let zbobr = Zbobr::new(config, task_backend, repo_backend);
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
                        Stage::GoPreparation,
                        Stage::Preparation,
                        Stage::GoPlanning,
                        Stage::Planning,
                        Stage::GoWorking,
                        Stage::Working,
                        Stage::GoReviewing,
                        Stage::Reviewing,
                        Stage::GoMerging,
                        Stage::Merging,
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
                        print_task(task);
                        println!("---");
                    }
                }
            }
            TaskSubcommand::Show { id } => {
                let task = zbobr.get_task(id).await?;
                print_task(&task);
            }
            TaskSubcommand::Update {
                id,
                title,
                description,
                stage,
                tool,
                model,
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
                            if let Some(s) = stage {
                                task.stage = s;
                            }
                            if let Some(to) = tool {
                                task.tool = Some(to);
                            }
                            if let Some(m) = model {
                                task.model = Some(m);
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
            TaskSubcommand::Prepare {
                task,
                model,
                port,
                show_prompt,
            } => {
                let base_prompt = load_prompts(&prompts.preparator, prompts.base_path.as_ref())?;
                let full_prompt = build_full_prompt(&base_prompt, Role::Preparator);

                if show_prompt {
                    println!("{}", full_prompt);
                    return Ok(());
                }

                let model_enum = model
                    .map(|m| m.parse::<Model>())
                    .transpose()
                    .context("Invalid model name")?;
                run_role_session(
                    &zbobr,
                    task,
                    Role::Preparator,
                    model_enum,
                    port,
                    &full_prompt,
                    &claude_executor_config,
                    &copilot_executor_config,
                    &mcp_tester_executor_config,
                )
                .await?;
            }
            TaskSubcommand::Plan {
                task,
                model,
                port,
                show_prompt,
            } => {
                let base_prompt = load_prompts(&prompts.planner, prompts.base_path.as_ref())?;
                let full_prompt = build_full_prompt(&base_prompt, Role::Planner);

                if show_prompt {
                    println!("{}", full_prompt);
                    return Ok(());
                }

                let model_enum = model
                    .map(|m| m.parse::<Model>())
                    .transpose()
                    .context("Invalid model name")?;
                run_role_session(
                    &zbobr,
                    task,
                    Role::Planner,
                    model_enum,
                    port,
                    &full_prompt,
                    &claude_executor_config,
                    &copilot_executor_config,
                    &mcp_tester_executor_config,
                )
                .await?;
            }
            TaskSubcommand::Work {
                task,
                model,
                port,
                show_prompt,
            } => {
                let base_prompt = load_prompts(&prompts.worker, prompts.base_path.as_ref())?;
                let full_prompt = build_full_prompt(&base_prompt, Role::Worker);

                if show_prompt {
                    println!("{}", full_prompt);
                    return Ok(());
                }

                let model_enum = model
                    .map(|m| m.parse::<Model>())
                    .transpose()
                    .context("Invalid model name")?;
                run_role_session(
                    &zbobr,
                    task,
                    Role::Worker,
                    model_enum,
                    port,
                    &full_prompt,
                    &claude_executor_config,
                    &copilot_executor_config,
                    &mcp_tester_executor_config,
                )
                .await?;
            }
            TaskSubcommand::Review {
                task,
                model,
                port,
                show_prompt,
            } => {
                let base_prompt = load_prompts(&prompts.reviewer, prompts.base_path.as_ref())?;
                let full_prompt = build_full_prompt(&base_prompt, Role::Reviewer);

                if show_prompt {
                    println!("{}", full_prompt);
                    return Ok(());
                }

                let model_enum = model
                    .map(|m| m.parse::<Model>())
                    .transpose()
                    .context("Invalid model name")?;
                run_role_session(
                    &zbobr,
                    task,
                    Role::Reviewer,
                    model_enum,
                    port,
                    &full_prompt,
                    &claude_executor_config,
                    &copilot_executor_config,
                    &mcp_tester_executor_config,
                )
                .await?;
            }
            TaskSubcommand::Merge {
                task,
                model,
                port,
                show_prompt,
            } => {
                let base_prompt = load_prompts(&prompts.merger, prompts.base_path.as_ref())?;
                let full_prompt = build_full_prompt(&base_prompt, Role::Merger);

                if show_prompt {
                    println!("{}", full_prompt);
                    return Ok(());
                }

                let model_enum = model
                    .map(|m| m.parse::<Model>())
                    .transpose()
                    .context("Invalid model name")?;
                run_role_session(
                    &zbobr,
                    task,
                    Role::Merger,
                    model_enum,
                    port,
                    &full_prompt,
                    &claude_executor_config,
                    &copilot_executor_config,
                    &mcp_tester_executor_config,
                )
                .await?;
            }
        },
        Command::Loop {
            interval,
            cleanup_interval,
            model,
            port,
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
                port,
                &prompts,
                &claude_executor_config,
                &copilot_executor_config,
                &mcp_tester_executor_config,
            )
            .await?;
        }
    }

    Ok(())
}

/// Start MCP server, invoke CLI tool (copilot/claude/stub), and handle stage transitions.
#[allow(clippy::too_many_arguments)]
async fn run_role_session(
    zbobr: &Zbobr,
    task_id: u64,
    role: Role,
    model: Option<Model>,
    base_port: u16,
    prompt: &str,
    claude_executor_config: &ZbobrExecutorClaudeConfig,
    copilot_executor_config: &ZbobrExecutorCopilotConfig,
    mcp_tester_executor_config: &ZbobrExecutorMcpTesterConfig,
) -> anyhow::Result<()> {
    let cli_tool = zbobr.config().cli_tool;
    let model = model.unwrap_or_else(|| match cli_tool {
        Tool::Claude => claude_executor_config.default_model.clone(),
        Tool::Copilot => copilot_executor_config.default_model.clone(),
        Tool::McpTester => Model::default(),
    });

    // Set stage
    let stage = match role {
        Role::Preparator => Stage::Preparation,
        Role::Planner => Stage::Planning,
        Role::Worker => Stage::Working,
        Role::Reviewer => Stage::Reviewing,
        Role::Merger => Stage::Merging,
    };
    zbobr.set_task_stage(task_id, stage).await?;

    // Clear any existing signal when a session starts so signal labels are removed
    // (GitHub backend will remove all "signal:*" labels when None is passed).
    if let Err(e) = zbobr.set_task_signal(task_id, None).await {
        tracing::warn!(
            "Failed to clear signal for task {} when starting session: {}",
            task_id,
            e
        );
    }

    // Create task directory within workspaces
    let task_dir = zbobr.config().workspaces.join(format!("task#{task_id}"));
    tokio::fs::create_dir_all(&task_dir).await?;

    // Create a channel to receive the actual port from the MCP server
    let (port_tx, port_rx) = std::sync::mpsc::channel();

    // Start MCP server in background, scoped to this role and task
    let server_zbobr = zbobr.clone();
    let server_role = role;
    let server_handle = tokio::spawn(async move {
        match zbobr_dispatcher::mcp::run_role_mcp_server(
            server_zbobr,
            base_port,
            server_role,
            task_id,
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

    // Receive the assigned port from the server task
    let assigned_port = port_rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .context("MCP server failed to report assigned port in time")?;

    // Execute the tool using the ToolExecutor trait with Ctrl+C handling
    let mcp_url = format!("http://127.0.0.1:{assigned_port}/{role}/{task_id}");

    let executor: Box<dyn ToolExecutor> = match cli_tool {
        Tool::Copilot => Box::new(CopilotExecutor {
            config: copilot_executor_config.clone(),
        }),
        Tool::Claude => Box::new(ClaudeExecutor {
            config: claude_executor_config.clone(),
        }),
        Tool::McpTester => Box::new(McpTesterExecutor {
            config: mcp_tester_executor_config.clone(),
        }),
    };
    let agent_token = &zbobr.config().agent_github_token;
    // copilot token belongs to the executor config now; only relevant for Copilot tool
    let copilot_token = match cli_tool {
        Tool::Copilot => &copilot_executor_config.copilot_github_token,
        _ => "",
    };
    let (execution_interrupted, execution_error) = tokio::select! {
        result = executor.execute(task_id, role, &model, assigned_port, prompt, &task_dir, &mcp_url, agent_token, copilot_token) => {
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
    };

    // Unlock task by moving back to PENDING - main loop will handle any signal-based transitions
    zbobr.set_task_stage(task_id, Stage::Pending).await?;
    if execution_interrupted {
        tracing::info!("Session interrupted for task #{task_id}, moved to PENDING");
    } else {
        tracing::info!("Session complete for task #{task_id}, moved to PENDING");
        // After a normal (non-interrupted) session finish, decide next signal
        // based on the current checklist and role. This centralizes the
        // checkbox->signal transition in the main loop rather than inside
        // individual checklist operations.
        let session = zbobr.task_session(task_id);
        match session.get_checklist().await {
            Ok(items) => {
                let has_unchecked = items.iter().any(|i| !i.checked);
                use zbobr_dispatcher::Signal;
                let next_signal = match role {
                    Role::Preparator => Signal::GoPlan,
                    Role::Worker => {
                        if has_unchecked {
                            Signal::GoWork
                        } else {
                            Signal::GoReview
                        }
                    }
                    Role::Reviewer => {
                        if has_unchecked {
                            Signal::GoWork
                        } else {
                            Signal::Done
                        }
                    }
                    Role::Merger => {
                        // After merger resolves the conflict, resume work
                        Signal::GoWork
                    }
                    _ => {
                        // Planner and other roles don't change signal here.
                        // Leave as-is.
                        Signal::GoAsk // placeholder no-op; we'll skip setting below
                    }
                };

                // Only set signal for Preparator/Worker/Reviewer/Merger logic above
                if matches!(
                    role,
                    Role::Preparator | Role::Worker | Role::Reviewer | Role::Merger
                ) && let Err(e) = session.set_signal(next_signal).await
                {
                    tracing::error!(
                        "Failed to set follow-up signal for task {} after session: {e}",
                        task_id
                    );
                }
            }
            Err(e) => tracing::error!(
                "Failed to read checklist for task {} after session: {e}",
                task_id
            ),
        }
    }

    // Shut down server
    server_handle.abort();

    // Propagate executor error after cleanup
    if let Some(e) = execution_error {
        return Err(e);
    }

    Ok(())
}

/// Main manager loop: polls for tasks and spawns sessions.
#[allow(clippy::too_many_arguments)]
async fn run_manager_loop(
    zbobr: &Zbobr,
    interval_secs: u64,
    cleanup_interval_secs: u64,
    model: Option<Model>,
    port: u16,
    prompts: &Prompts,
    claude_executor_config: &ZbobrExecutorClaudeConfig,
    copilot_executor_config: &ZbobrExecutorCopilotConfig,
    mcp_tester_executor_config: &ZbobrExecutorMcpTesterConfig,
) -> anyhow::Result<()> {
    let cli_tool = zbobr.config().cli_tool;
    let model = model.unwrap_or_else(|| match cli_tool {
        Tool::Claude => claude_executor_config.default_model.clone(),
        Tool::Copilot => copilot_executor_config.default_model.clone(),
        Tool::McpTester => Model::default(),
    });

    // Load prompts once at loop start and append API docs
    let preparator_base = load_prompts(&prompts.preparator, prompts.base_path.as_ref())?;
    let planner_base = load_prompts(&prompts.planner, prompts.base_path.as_ref())?;
    let worker_base = load_prompts(&prompts.worker, prompts.base_path.as_ref())?;
    let reviewer_base = load_prompts(&prompts.reviewer, prompts.base_path.as_ref())?;
    let merger_base = load_prompts(&prompts.merger, prompts.base_path.as_ref())?;
    let preparator_prompt = build_full_prompt(&preparator_base, Role::Preparator);
    let planner_prompt = build_full_prompt(&planner_base, Role::Planner);
    let worker_prompt = build_full_prompt(&worker_base, Role::Worker);
    let reviewer_prompt = build_full_prompt(&reviewer_base, Role::Reviewer);
    let merger_prompt = build_full_prompt(&merger_base, Role::Merger);

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
    tracing::info!("Backend: {:?}", zbobr.config().backend);

    let mut last_cleanup = std::time::Instant::now();

    loop {
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

        // First, check PENDING tasks for signals and transition them.
        // Note: Only PENDING tasks are checked - tasks already in GO_PLANNING or GO_WORKING
        // stages are locked and should not be transitioned by this logic.
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

        for task in pending_tasks {
            if let Some(signal) = task.signal {
                let target_stage = signal.target_stage();
                if target_stage != Stage::Pending {
                    tracing::info!(
                        "Task #{} has signal {:?}, transitioning from PENDING to {}",
                        task.id,
                        signal,
                        target_stage
                    );
                    if let Err(e) = zbobr.set_task_stage(task.id, target_stage).await {
                        tracing::error!("Failed to transition task #{}: {e}", task.id);
                    }
                }
            }
        }

        // Check for GO_PREPARATION tasks
        let preparation_tasks = match zbobr
            .list_tasks_by_stage(Stage::GoPreparation, Some(current_tool))
            .await
        {
            Ok(tasks) => tasks,
            Err(e) => {
                tracing::error!("Failed to check GO_PREPARATION tasks: {e}");
                vec![]
            }
        };

        if let Some(task) = preparation_tasks.first() {
            let task_model = task.model.clone().unwrap_or_else(|| model.clone());
            tracing::info!(
                "Found GO_PREPARATION task #{} (tool: {:?}, model: {}) - running preparator",
                task.id,
                task.tool,
                task_model
            );
            if let Err(e) = run_role_session(
                zbobr,
                task.id,
                Role::Preparator,
                Some(task_model),
                port,
                &preparator_prompt,
                claude_executor_config,
                copilot_executor_config,
                mcp_tester_executor_config,
            )
            .await
            {
                tracing::error!("Preparator session failed: {e}");
            }
            continue;
        }

        // Check for GO_PLANNING tasks
        let planning_tasks = match zbobr
            .list_tasks_by_stage(Stage::GoPlanning, Some(current_tool))
            .await
        {
            Ok(tasks) => tasks,
            Err(e) => {
                tracing::error!("Failed to check GO_PLANNING tasks: {e}");
                vec![]
            }
        };

        if let Some(task) = planning_tasks.first() {
            let task_model = task.model.clone().unwrap_or_else(|| model.clone());
            tracing::info!(
                "Found GO_PLANNING task #{} (tool: {:?}, model: {}) - running planner",
                task.id,
                task.tool,
                task_model
            );
            if let Err(e) = run_role_session(
                zbobr,
                task.id,
                Role::Planner,
                Some(task_model),
                port,
                &planner_prompt,
                claude_executor_config,
                copilot_executor_config,
                mcp_tester_executor_config,
            )
            .await
            {
                tracing::error!("Planner session failed: {e}");
            }
            continue;
        }

        // Check for GO_WORKING tasks
        let working_tasks = match zbobr
            .list_tasks_by_stage(Stage::GoWorking, Some(current_tool))
            .await
        {
            Ok(tasks) => tasks,
            Err(e) => {
                tracing::error!("Failed to check GO_WORKING tasks: {e}");
                vec![]
            }
        };

        if let Some(task) = working_tasks.first() {
            let task_model = task.model.clone().unwrap_or_else(|| model.clone());
            tracing::info!(
                "Found GO_WORKING task #{} (tool: {:?}, model: {}) - running worker",
                task.id,
                task.tool,
                task_model
            );
            if let Err(e) = run_role_session(
                zbobr,
                task.id,
                Role::Worker,
                Some(task_model),
                port,
                &worker_prompt,
                claude_executor_config,
                copilot_executor_config,
                mcp_tester_executor_config,
            )
            .await
            {
                tracing::error!("Worker session failed: {e}");
            }
            continue;
        }

        // Check for GO_REVIEWING tasks
        let reviewing_tasks = match zbobr
            .list_tasks_by_stage(Stage::GoReviewing, Some(current_tool))
            .await
        {
            Ok(tasks) => tasks,
            Err(e) => {
                tracing::error!("Failed to check GO_REVIEWING tasks: {e}");
                vec![]
            }
        };

        if let Some(task) = reviewing_tasks.first() {
            let task_model = task.model.clone().unwrap_or_else(|| model.clone());
            tracing::info!(
                "Found GO_REVIEWING task #{} (tool: {:?}, model: {}) - running reviewer",
                task.id,
                task.tool,
                task_model
            );
            if let Err(e) = run_role_session(
                zbobr,
                task.id,
                Role::Reviewer,
                Some(task_model),
                port,
                &reviewer_prompt,
                claude_executor_config,
                copilot_executor_config,
                mcp_tester_executor_config,
            )
            .await
            {
                tracing::error!("Reviewer session failed: {e}");
            }
            continue;
        }

        // Check for GO_MERGING tasks
        let merging_tasks = match zbobr
            .list_tasks_by_stage(Stage::GoMerging, Some(current_tool))
            .await
        {
            Ok(tasks) => tasks,
            Err(e) => {
                tracing::error!("Failed to check GO_MERGING tasks: {e}");
                vec![]
            }
        };

        if let Some(task) = merging_tasks.first() {
            let task_model = task.model.clone().unwrap_or_else(|| model.clone());
            tracing::info!(
                "Found GO_MERGING task #{} (tool: {:?}, model: {}) - running merger",
                task.id,
                task.tool,
                task_model
            );
            if let Err(e) = run_role_session(
                zbobr,
                task.id,
                Role::Merger,
                Some(task_model),
                port,
                &merger_prompt,
                claude_executor_config,
                copilot_executor_config,
                mcp_tester_executor_config,
            )
            .await
            {
                tracing::error!("Merger session failed: {e}");
            }
            continue;
        }

        // Log task statistics before sleeping
        tracing::info!(
            "Task statistics for tool {:?}: GO_PREPARATION={}, GO_PLANNING={}, GO_WORKING={}, GO_REVIEWING={}, GO_MERGING={}",
            current_tool,
            preparation_tasks.len(),
            planning_tasks.len(),
            working_tasks.len(),
            reviewing_tasks.len(),
            merging_tasks.len()
        );

        if !preparation_tasks.is_empty() {
            let summary: Vec<_> = preparation_tasks
                .iter()
                .map(|t| format!("#{} (tool: {:?}, model: {:?})", t.id, t.tool, t.model))
                .collect();
            tracing::info!("  GO_PREPARATION tasks: {}", summary.join(", "));
        }

        if !planning_tasks.is_empty() {
            let summary: Vec<_> = planning_tasks
                .iter()
                .map(|t| format!("#{} (tool: {:?}, model: {:?})", t.id, t.tool, t.model))
                .collect();
            tracing::info!("  GO_PLANNING tasks: {}", summary.join(", "));
        }

        if !working_tasks.is_empty() {
            let summary: Vec<_> = working_tasks
                .iter()
                .map(|t| format!("#{} (tool: {:?}, model: {:?})", t.id, t.tool, t.model))
                .collect();
            tracing::info!("  GO_WORKING tasks: {}", summary.join(", "));
        }

        if !reviewing_tasks.is_empty() {
            let summary: Vec<_> = reviewing_tasks
                .iter()
                .map(|t| format!("#{} (tool: {:?}, model: {:?})", t.id, t.tool, t.model))
                .collect();
            tracing::info!("  GO_REVIEWING tasks: {}", summary.join(", "));
        }

        if !merging_tasks.is_empty() {
            let summary: Vec<_> = merging_tasks
                .iter()
                .map(|t| format!("#{} (tool: {:?}, model: {:?})", t.id, t.tool, t.model))
                .collect();
            tracing::info!("  GO_MERGING tasks: {}", summary.join(", "));
        }

        tracing::info!("No processable tasks. Sleeping {interval_secs}s...");

        // Sleep with Ctrl+C handling
        tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_secs(interval_secs)) => {
                // Continue to next iteration
            }
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("Received shutdown signal, exiting...");
                break;
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
            "zbobr",
            "task",
            "list",
            "--stage",
            "pending",
            "--tool",
            "claude",
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
}
