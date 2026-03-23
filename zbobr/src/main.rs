#![allow(clippy::needless_borrows_for_generic_args)]

mod commands;
mod init;

use anyhow::Context;
use clap::Parser;
use commands::Command;
use zbobr_api::config::{WorkflowArgs, WorkflowConfig, WorkflowToml};
use zbobr_dispatcher::{
    ConfigFileArg,
    config::{
        Config as _, ZbobrDispatcherArgs, ZbobrDispatcherConfig, ZbobrDispatcherToml,
        ZbobrExecutorArgs, ZbobrExecutorConfig, ZbobrExecutorToml,
    },
};
use zbobr_repo_backend_github::{
    ZbobrRepoBackendGithubArgs, ZbobrRepoBackendGithubConfig, ZbobrRepoBackendGithubToml,
};
use zbobr_task_backend_github::{
    ZbobrTaskBackendGithubArgs, ZbobrTaskBackendGithubConfig, ZbobrTaskBackendGithubToml,
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

    commands::run(config.dispatcher, config.tasks, config.repo, config.executor, config.workflow, location.config_dir, cli.command).await
}
