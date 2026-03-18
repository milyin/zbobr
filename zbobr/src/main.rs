#![allow(clippy::needless_borrows_for_generic_args)]

mod commands;
mod init;

use std::sync::Arc;

use anyhow::Context;
use clap::Parser;
use zbobr_dispatcher::backend::{TaskBackend as _, WorktreeBackend as _};
use zbobr_dispatcher::config::{
    Config as _, ZbobrDispatcherArgs, ZbobrDispatcherConfig, ZbobrDispatcherToml,
    ZbobrExecutorArgs, ZbobrExecutorConfig, ZbobrExecutorToml,
};
use zbobr_dispatcher::{ConfigFileArg, ConfiguredPromptBuilder};
use zbobr_api::config::{PipelineConfig, PipelineArgs, PipelineToml};
use zbobr_utility::config_struct;

use zbobr_repo_backend_github::{
    ZbobrRepoBackendGithub, ZbobrRepoBackendGithubArgs, ZbobrRepoBackendGithubConfig,
    ZbobrRepoBackendGithubToml,
};
use zbobr_task_backend_github::{
    TaskBackendGithub, ZbobrTaskBackendGithubArgs, ZbobrTaskBackendGithubConfig,
    ZbobrTaskBackendGithubToml,
};

use commands::Command;

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
    pipeline: PipelineConfig,
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

    config
        .dispatcher
        .validate()
        .with_context(|| format!("Config file: {}", location.config_path.display()))?;

    config.pipeline.validate()?;

    let command = cli.command;
    let pipeline = config.pipeline;

    let task_backend = TaskBackendGithub::from_config(config.tasks)?;
    let repo_backend = ZbobrRepoBackendGithub::from_config(config.repo)?;
    task_backend.validate_connectivity().await?;
    repo_backend.validate_connectivity().await?;

    let prompt_builder = ConfiguredPromptBuilder::new(Some(location.config_dir.clone()), Arc::new(pipeline.clone()));

    let dispatcher = zbobr_dispatcher::ZbobrDispatcherBuilder::new()
        .with_config(Arc::new(config.dispatcher))
        .with_task_backend(Arc::new(task_backend) as Arc<dyn zbobr_dispatcher::backend::TaskBackend>)
        .with_repo_backend(Arc::new(repo_backend) as Arc<dyn zbobr_dispatcher::backend::WorktreeBackend>)
        .with_claude(Arc::new(config.executor.claude))
        .with_copilot(Arc::new(config.executor.copilot))
        .with_mcp_tester(Arc::new(config.executor.mcp_tester))
        .with_prompt_builder(prompt_builder)
        .build();

    commands::run_command(&dispatcher, command, &pipeline).await
}
