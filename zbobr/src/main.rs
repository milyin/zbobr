#![allow(clippy::needless_borrows_for_generic_args)]

mod commands;
mod init;

use std::sync::Arc;

use anyhow::Context;
use clap::Parser;
use commands::Command;
use zbobr_api::config::{WorkflowArgs, WorkflowConfig, WorkflowToml};
use zbobr_dispatcher::{
    ConfigFileArg, ConfiguredPromptBuilder, Workflow,
    config::{
        Config as _, ZbobrDispatcherArgs, ZbobrDispatcherConfig, ZbobrDispatcherToml,
        ZbobrExecutorArgs, ZbobrExecutorConfig, ZbobrExecutorToml,
    },
};
use zbobr_executor_claude::ClaudeExecutor;
use zbobr_executor_copilot::CopilotExecutor;
use zbobr_executor_mcp_tester::McpTesterExecutor;
use zbobr_repo_backend_github::{
    ZbobrRepoBackendGithub, ZbobrRepoBackendGithubArgs, ZbobrRepoBackendGithubConfig,
    ZbobrRepoBackendGithubToml,
};
use zbobr_task_backend_github::{
    TaskBackendGithub, ZbobrTaskBackendGithubArgs, ZbobrTaskBackendGithubConfig,
    ZbobrTaskBackendGithubToml,
};
use zbobr_utility::config_struct;

#[derive(Clone, Default)]
#[config_struct]
struct RootConfig {
    #[config(nested)]
    dispatcher: ZbobrDispatcherConfig,
    #[config(nested)]
    tasks: ZbobrTaskBackendGithubConfig,
    #[config(nested)]
    repo: ZbobrRepoBackendGithubConfig,
    #[config(nested)]
    executor: ZbobrExecutorConfig,
    #[config(nested)]
    workflow: WorkflowConfig,
}

#[derive(Parser)]
struct Cli {
    #[command(
        flatten,
        next_help_heading = "[config] Meta options and config file overrides"
    )]
    config_file: ConfigFileArg,

    #[command(flatten)]
    settings: RootConfigArgs,

    #[command(subcommand)]
    command: Command,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = rustls::crypto::ring::default_provider().install_default();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let cli: Cli = zbobr_dispatcher::parse_cli(
        "zbobr",
        "GitHub-backed AI-powered task dispatcher",
        "GitHub-backed AI-powered task dispatcher that manages tasks through automated stages.\n\n\
        Tasks are stored in GitHub issues and work is done via pull requests.\n\
        Requires a GitHub token: set GH_TOKEN or GITHUB_TOKEN env var.\n\
        Easiest way: export GH_TOKEN=$(gh auth token)",
    );

    // Handle init before config loading — no existing config needed
    if let Command::Init { ref directory } = cli.command {
        return init::init_workspace(directory).await;
    }

    let location = zbobr_dispatcher::resolve_config_location(&cli.config_file.path, "zbobr.toml")?;

    let root_toml = if location.config_path.exists() {
        let content = std::fs::read_to_string(&location.config_path)
            .with_context(|| format!("Failed to read {}", location.config_path.display()))?;
        let parsed: RootConfigToml = toml::from_str(&content)
            .with_context(|| format!("Failed to parse {}", location.config_path.display()))?;
        Some(parsed)
    } else {
        None
    };

    let config = RootConfig::build(root_toml, cli.settings, &location.config_dir);

    let command = cli.command;
    let workflow = Workflow::new(config.workflow)?;

    let task_backend = TaskBackendGithub::new(config.tasks).await?;
    let repo_backend = ZbobrRepoBackendGithub::new(config.repo).await?;

    let mut prompt_builder = ConfiguredPromptBuilder::new(
        Some(location.config_dir.clone()),
        Arc::new(workflow.clone()),
    );
    if let Some(ref v) = config.dispatcher.default_destination_repository {
        prompt_builder = prompt_builder.with_var("default_destination_repository", v.clone());
    }
    if let Some(ref v) = config.dispatcher.default_destination_branch {
        prompt_builder = prompt_builder.with_var("default_destination_branch", v.clone());
    }

    let claude = ClaudeExecutor::new(config.executor.claude);
    let copilot = CopilotExecutor::new(config.executor.copilot);
    let mcp_tester = McpTesterExecutor::new(config.executor.mcp_tester);

    let dispatcher = zbobr_dispatcher::ZbobrDispatcherBuilder::new()
        .with_config(config.dispatcher)
        .with_workflow(workflow)
        .with_task_backend(task_backend)
        .with_repo_backend(repo_backend)
        .with_claude(claude)
        .with_copilot(copilot)
        .with_mcp_tester(mcp_tester)
        .with_prompt_builder(prompt_builder)
        .build()
        .validated()?;

    commands::run_command(dispatcher, command).await
}
