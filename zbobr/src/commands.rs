#![allow(clippy::needless_borrows_for_generic_args)]

use std::{path::PathBuf, sync::Arc};

use clap::Subcommand;
use zbobr_api::{Comment, Pipeline, Stage, State, Task, config::WorkflowConfig};
use zbobr_dispatcher::{
    ConfiguredPromptBuilder, TaskDir, Workflow, ZbobrDispatcher,
    config::{ZbobrDispatcherConfig, ZbobrExecutorConfig},
    print_task,
};
use zbobr_executor_claude::ClaudeExecutor;
use zbobr_executor_copilot::CopilotExecutor;
use zbobr_executor_mcp_tester::{McpTesterExecutor, ZbobrExecutorMcpTesterConfig};
use zbobr_repo_backend_github::{ZbobrRepoBackendGithub, ZbobrRepoBackendGithubConfig};
use zbobr_task_backend_github::{TaskBackendGithub, ZbobrTaskBackendGithubConfig};
use zbobr_utility::{git, git_output};

// ---------------------------------------------------------------------------
// CLI types
// ---------------------------------------------------------------------------

/// Top-level commands.
#[derive(Subcommand)]
pub enum Command {
    /// Initialize a new zbobr workspace with config, prompts, and directories
    Init {
        /// Destination directory for the new workspace
        directory: PathBuf,
    },
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
    /// Manage tasks (create, show, update, delete)
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
    /// Show a task by ID (or list all tasks if no ID given)
    Show {
        /// Task ID
        id: Option<u64>,
    },
    /// Update fields of an existing task
    Update {
        /// Task ID
        id: Option<u64>,
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
        id: Option<u64>,
    },
    /// Process a task according to its current stage (single-step)
    Process {
        /// Task ID
        task: Option<u64>,
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
    /// Show the resolved prompt for a task stage
    Prompt {
        /// Task ID (if omitted, placeholders are used instead of real task data)
        id: Option<u64>,
        /// Stage name to show the prompt for
        #[arg(long, value_parser = |s: &str| -> Result<Stage, std::convert::Infallible> { Ok(Stage::from(s)) })]
        stage: Option<Stage>,
        /// Role name to show the prompt for
        #[arg(long)]
        role: Option<String>,
        /// Pipeline name (required when stage name exists in multiple pipelines)
        #[arg(long)]
        pipeline: Option<Pipeline>,
    },
    /// Rewrite commit authors on the task's PR branch and push back
    OverwriteAuthor {
        /// Task ID
        id: Option<u64>,
        /// Skip confirmation and force execution
        #[arg(long)]
        force: bool,
        /// Show what would be done without making changes
        #[arg(long)]
        dry_run: bool,
    },
}

// ---------------------------------------------------------------------------
// Command dispatch
// ---------------------------------------------------------------------------

impl Command {
    /// Whether this command requires backend connectivity (GitHub token, etc.).
    fn needs_backends(&self) -> bool {
        match self {
            Command::Init { .. } => false,
            Command::Task {
                subcommand: TaskSubcommand::Prompt { id: None, .. },
            } => false,
            Command::Task {
                subcommand: TaskSubcommand::Show { id: None },
            } => false,
            _ => true,
        }
    }
}

/// Main entry point for command dispatch. Creates backends only when needed.
pub async fn run(
    dispatcher_config: ZbobrDispatcherConfig,
    tasks_config: ZbobrTaskBackendGithubConfig,
    repo_config: ZbobrRepoBackendGithubConfig,
    executor_config: ZbobrExecutorConfig,
    workflow_config: WorkflowConfig,
    config_dir: PathBuf,
    command: Command,
) -> anyhow::Result<()> {
    let workflow = Workflow::new(workflow_config)?;
    let prompt_builder = ConfiguredPromptBuilder::new(
        Some(config_dir),
        Arc::new(workflow.clone()),
    );

    if !command.needs_backends() {
        return run_without_backends(command, &prompt_builder);
    }

    let task_backend = TaskBackendGithub::new(tasks_config).await?;
    let repo_backend = ZbobrRepoBackendGithub::new(repo_config).await?;

    let claude = ClaudeExecutor::new(executor_config.claude);
    let copilot = CopilotExecutor::new(executor_config.copilot);
    let mcp_tester = McpTesterExecutor::new(executor_config.mcp_tester);

    let dispatcher = zbobr_dispatcher::ZbobrDispatcherBuilder::new()
        .with_config(dispatcher_config)
        .with_workflow(workflow)
        .with_task_backend(task_backend)
        .with_repo_backend(repo_backend)
        .with_claude(claude)
        .with_copilot(copilot)
        .with_mcp_tester(mcp_tester)
        .with_prompt_builder(prompt_builder)
        .build()
        .validated()?;

    run_with_dispatcher(dispatcher, command).await
}

/// Handle commands that don't need backends.
fn run_without_backends(
    command: Command,
    prompt_builder: &ConfiguredPromptBuilder,
) -> anyhow::Result<()> {
    match command {
        Command::Init { .. } => {
            unreachable!("Init is handled before config loading in main()")
        }
        Command::Task {
            subcommand:
                TaskSubcommand::Prompt {
                    id: None,
                    stage,
                    role,
                    pipeline,
                },
        } => {
            let workflow = prompt_builder.workflow_config();
            let stage_def = resolve_stage_def(workflow, &stage, &role, &pipeline)?;
            let (task, comments) = dummy_task_and_comments();
            let prompt =
                prompt_builder.build_for_stage_with_task(stage_def, &task, &comments)?;
            println!("{}", prompt);
            Ok(())
        }
        Command::Task {
            subcommand: TaskSubcommand::Show { id: None },
        } => {
            let (task, comments) = dummy_task_and_comments();
            print_task(&task, &comments);
            Ok(())
        }
        _ => unreachable!("needs_backends() returned false for unexpected command"),
    }
}

fn dummy_task_and_comments() -> (Task, Vec<Comment>) {
    let task = Task {
        id: 0,
        title: "{TITLE}".to_string(),
        description: "{DESCRIPTION}".to_string(),
        state: State::Ready,
        destination_repository: Some("{DESTINATION_REPOSITORY}".to_string()),
        destination_branch: Some("{DESTINATION_BRANCH}".to_string()),
        work_branch: Some("{WORK_BRANCH}".to_string()),
        pr_url: None,
        checklist: vec![],
        signal: None,
        stack: vec![],
        pause: false,
        confirm: false,
        pipeline_run_id: 0,
        stage_count: 0,
        etag: None,
    };
    let comments = vec![
        Comment {
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            stage: "planning".to_string(),
            hostname: "dummy".to_string(),
            tool: None,
            model: None,
            text: "{USER_REQUEST}".to_string(),
            pipeline: "main".to_string(),
            pipeline_run_id: 1,
            caller_pipeline: None,
            caller_pipeline_run_id: None,
            report_name: None,
            prompt_name: None,
        },
        Comment {
            timestamp: "2025-01-01T01:00:00Z".to_string(),
            stage: "planning".to_string(),
            hostname: "dummy".to_string(),
            tool: None,
            model: None,
            text: "[report_success]\n{LAST_REPORT}".to_string(),
            pipeline: "main".to_string(),
            pipeline_run_id: 1,
            caller_pipeline: None,
            caller_pipeline_run_id: None,
            report_name: None,
            prompt_name: None,
        },
        Comment {
            timestamp: "2025-01-01T02:00:00Z".to_string(),
            stage: "working".to_string(),
            hostname: "dummy".to_string(),
            tool: None,
            model: None,
            text: "[report_failure]\n{LAST_FAILURE}".to_string(),
            pipeline: "main".to_string(),
            pipeline_run_id: 1,
            caller_pipeline: None,
            caller_pipeline_run_id: None,
            report_name: None,
            prompt_name: None,
        },
    ];
    (task, comments)
}

/// Handle commands that need the full dispatcher.
async fn run_with_dispatcher(zbobr: ZbobrDispatcher, command: Command) -> anyhow::Result<()> {
    let zbobr = Arc::new(zbobr);
    match command {
        Command::Init { .. } => unreachable!(),
        Command::Setup { force } => {
            zbobr.setup(force).await?;
        }
        Command::Cleanup { dry_run } => {
            zbobr.cleanup_closed_tasks(dry_run).await?;
        }
        Command::Task { subcommand } => {
            run_task_subcommand(&zbobr, subcommand).await?;
        }
        Command::Loop {
            interval,
            cleanup_interval,
            ..
        } => {
            zbobr_dispatcher::run_manager_loop(&zbobr, interval, cleanup_interval).await?;
        }
    }
    Ok(())
}

fn require_task_id(id: Option<u64>, command: &str) -> anyhow::Result<u64> {
    id.ok_or_else(|| anyhow::anyhow!("Task ID is required for '{command}'"))
}

async fn run_task_subcommand(
    zbobr: &Arc<ZbobrDispatcher>,
    subcommand: TaskSubcommand,
) -> anyhow::Result<()> {
    let task_backend = zbobr.task_backend();
    match subcommand {
        TaskSubcommand::Create {
            title,
            description,
            state,
            dest_repo,
            dest_branch,
            confirm,
        } => {
            let parsed_state = state.parse::<zbobr_api::State>()?;
            let id = zbobr
                .create_task(&title, &description, parsed_state, dest_repo, dest_branch)
                .await?;
            if confirm {
                zbobr.task_session(id).set_confirm(true).await?;
            }
            println!("Created task #{}", id);
        }
        TaskSubcommand::List { state } => {
            let state_filter = state
                .as_deref()
                .map(str::parse::<zbobr_api::State>)
                .transpose()?;
            let weak_tasks = task_backend.list_tasks().await?;
            let mut tasks = Vec::new();
            for w in &weak_tasks {
                let task = w.snapshot(false).await?;
                if let Some(ref filter) = state_filter {
                    if task.state != *filter {
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
            if let Some(id) = id {
                let weak = task_backend.get_task(id).await?;
                let task = weak.snapshot(false).await?;
                let discussion = weak.get_comments().await?;
                print_task(&task, &discussion);
            } else {
                let weak_tasks = task_backend.list_tasks().await?;
                let mut tasks = Vec::new();
                for w in &weak_tasks {
                    tasks.push(w.snapshot(false).await?);
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
            let id = require_task_id(id, "update")?;
            let parsed_state = state.map(|s| s.parse::<zbobr_api::State>()).transpose()?;
            let parsed_signal = signal.map(|s| s.parse::<zbobr_api::Signal>()).transpose()?;
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
                    if let Some(s) = parsed_state {
                        if task.confirm && task.state != s {
                            task.pause = true;
                        }
                        task.state = s;
                    }
                    if let Some(s) = parsed_signal {
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
            let id = require_task_id(id, "delete")?;
            let weak = task_backend.get_task(id).await?;
            let mutable = weak.upgrade().await?;
            mutable.close().await?;
            println!("Deleted task #{}", id);
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
            let task = require_task_id(task, "process")?;
            let task_obj = task_backend.get_task(task).await?.snapshot(false).await?;
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
                    scenarios: Default::default(),
                })
            } else {
                None
            };
            zbobr_dispatcher::process_task(zbobr, &task_obj, mcp_tester_config_override.as_ref())
                .await?;
        }
        TaskSubcommand::Prompt {
            id,
            stage,
            role,
            pipeline,
        } => {
            let workflow = zbobr.workflow().config();
            let stage_def = resolve_stage_def(workflow, &stage, &role, &pipeline)?;
            let prompt = if let Some(task_id) = id {
                zbobr
                    .prompt_builder()
                    .build_for_stage(stage_def, task_id, zbobr.task_backend())
                    .await?
            } else {
                let (task, comments) = dummy_task_and_comments();
                zbobr
                    .prompt_builder()
                    .build_for_stage_with_task(stage_def, &task, &comments)?
            };
            println!("{}", prompt);
        }
        TaskSubcommand::OverwriteAuthor { id, force, dry_run } => {
            let id = require_task_id(id, "overwrite-author")?;
            overwrite_author(zbobr, id, force, dry_run).await?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn resolve_stage_def<'a>(
    workflow: &'a WorkflowConfig,
    stage: &Option<Stage>,
    role: &Option<String>,
    pipeline: &Option<Pipeline>,
) -> anyhow::Result<&'a zbobr_api::config::StageDefinition> {
    match (stage, role) {
        (None, None) => {
            anyhow::bail!("Either --stage or --role must be specified");
        }
        (Some(_), Some(_)) => {
            anyhow::bail!("Only one of --stage or --role may be specified, not both");
        }
        (Some(stage_name), None) => {
            if let Some(p) = pipeline {
                workflow
                    .stage(p.clone(), stage_name.as_str())
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "Stage '{}' not found in pipeline '{}'",
                            stage_name,
                            p
                        )
                    })
            } else {
                let matches: Vec<_> = workflow
                    .all_stages()
                    .into_iter()
                    .filter(|(_, sname, _)| *sname == stage_name.as_str())
                    .collect();
                match matches.len() {
                    0 => anyhow::bail!("Stage '{}' not found in any pipeline", stage_name),
                    1 => Ok(matches[0].2),
                    _ => {
                        let pipelines: Vec<_> =
                            matches.iter().map(|(p, _, _)| p.to_string()).collect();
                        anyhow::bail!(
                            "Stage '{}' exists in multiple pipelines: {}. Use --pipeline to disambiguate.",
                            stage_name,
                            pipelines.join(", ")
                        );
                    }
                }
            }
        }
        (None, Some(role_name)) => {
            if let Some(p) = pipeline {
                let pipeline_config = workflow
                    .pipeline(p.clone())
                    .ok_or_else(|| anyhow::anyhow!("Pipeline '{}' not found", p))?;
                pipeline_config
                    .stages
                    .values()
                    .find(|s| s.role_name() == Some(role_name.as_str()))
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "No stage with role '{}' found in pipeline '{}'",
                            role_name,
                            p
                        )
                    })
            } else {
                workflow
                    .find_stage_by_role(role_name)
                    .map(|(_, _, def)| def)
                    .ok_or_else(|| {
                        anyhow::anyhow!("No stage with role '{}' found", role_name)
                    })
            }
        }
    }
}

async fn overwrite_author(
    zbobr: &Arc<ZbobrDispatcher>,
    id: u64,
    force: bool,
    dry_run: bool,
) -> anyhow::Result<()> {
    let task_backend = zbobr.task_backend();
    let repo_backend = zbobr.repo_backend();
    let task = task_backend.get_task(id).await?.snapshot(false).await?;
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

    let repo_name = std::path::Path::new(dest_repo)
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow::anyhow!("Cannot extract repo name from: {}", dest_repo))?;
    let repo_dir = task_dir.path().join(repo_name);

    if !repo_dir.exists() {
        return Err(anyhow::anyhow!(
            "Task repo not found at {}. Run 'zbobr task clone {}' first.",
            repo_dir.display(),
            id
        ));
    }

    git(&repo_dir, &["fetch", "origin", dest_branch]).await?;

    if !dry_run {
        let config = zbobr.config();
        zbobr_utility::rewrite_authors_on_worktree(
            &repo_dir,
            dest_branch,
            &config.git_user_name,
            &config.git_user_email,
        )
        .await?;
        // Push rewritten commits
        if let Err(e) = repo_backend.update_pr(&identity).await {
            tracing::warn!("Could not push rewritten commits for task #{}: {e}", id);
        }
        println!("Successfully rewrote commit authors and pushed");
    } else {
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

    Ok(())
}
