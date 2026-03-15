use anyhow::Context;
use clap::Parser;
use zbobr_dispatcher::backend::{TaskBackend as _, WorktreeBackend as _};
use zbobr_dispatcher::config::{
    Config as _, PromptsArgs, PromptsConfig, PromptsToml, ZbobrDispatcherArgs,
    ZbobrDispatcherConfig, ZbobrDispatcherToml, ZbobrExecutorArgs, ZbobrExecutorConfig,
    ZbobrExecutorToml,
};
use zbobr_dispatcher::{Command, ConfigFileArg};
use zbobr_utility::config_struct;

use zbobr_repo_backend_fs::{
    ZbobrRepoBackendFs, ZbobrRepoBackendFsArgs, ZbobrRepoBackendFsConfig, ZbobrRepoBackendFsToml,
};
use zbobr_task_backend_fs::{
    ArcTaskBackendFs, ZbobrTaskBackendFs, ZbobrTaskBackendFsArgs, ZbobrTaskBackendFsConfig,
    ZbobrTaskBackendFsToml,
};

#[derive(Clone, Default)]
#[config_struct]
struct RootConfig {
    #[config(nested)]
    dispatcher: ZbobrDispatcherConfig,
    #[config(nested)]
    tasks: ZbobrTaskBackendFsConfig,
    #[config(nested)]
    repo: ZbobrRepoBackendFsConfig,
    #[config(nested)]
    executor: ZbobrExecutorConfig,
    #[config(nested)]
    prompts: PromptsConfig,
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
        "zbobr-fs",
        "Filesystem-backed AI-powered task dispatcher",
        "Filesystem-backed AI-powered task dispatcher that manages tasks through automated stages.\n\n\
        Tasks are stored in YAML files and work is done on local git clones.\n\
        Tasks flow through: PENDING -> PREPARING -> PLANNING -> WORKING -> REVIEWING -> DONE.\n\
        Merge conflicts are handled by MERGING sessions when the conflict flag is set.\n\n\
        Ideal for testing, local development, and offline scenarios.\n\n\
        Default config file: zbobr-fs.toml in current directory.",
    );

    let location =
        zbobr_dispatcher::resolve_config_location(&cli.config_file.path, "zbobr-fs.toml")?;

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

    let command = cli.command;

    let zbobr = zbobr_dispatcher::ZbobrDispatcher::new_with_executors(
        config.dispatcher,
        config.executor.claude,
        config.executor.copilot,
        config.executor.mcp_tester,
    );

    let task_backend = ArcTaskBackendFs::new(ZbobrTaskBackendFs::from_config(config.tasks)?);
    let repo_backend = ZbobrRepoBackendFs::from_config(config.repo)?;
    task_backend.validate_connectivity().await?;
    repo_backend.validate_connectivity().await?;

    let prompt_builder = zbobr_dispatcher::ConfiguredPromptBuilder::from(config.prompts);
    prompt_builder.validate()?;

    zbobr_dispatcher::run_command(
        zbobr,
        task_backend,
        repo_backend,
        command,
        prompt_builder,
    )
    .await
}
