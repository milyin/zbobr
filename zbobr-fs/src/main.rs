mod config;

use std::sync::Arc;

use anyhow::Context;
use clap::Parser;
use zbobr_dispatcher::{
    ZbobrDispatcher,
    cli::{Command, ConfigFileArg, parse_cli},
    prompts::resolve_prompts,
};
use zbobr_task_backend_fs::ZbobrTaskBackendFs;
use zbobr_repo_backend_fs::ZbobrRepoBackendFs;

use config::{Config, ConfigArgs, ConfigToml};

#[derive(Parser)]
#[command(
    name = "zbobr-fs",
    about = "Filesystem-backed AI-powered task dispatcher",
    long_about = "Filesystem-backed AI-powered task dispatcher that manages tasks through automated stages.\n\n\
        Tasks are stored in YAML files and work is done on local git clones.\n\
        Tasks flow through: PENDING -> PREPARING -> PLANNING -> WORKING -> REVIEWING -> DONE.\n\
        Merge conflicts are handled by MERGING sessions when the conflict flag is set.\n\n\
        Ideal for testing, local development, and offline scenarios.\n\n\
        Default config file: zbobr-fs.toml in current directory."
)]
struct Cli {
    #[command(flatten, next_help_heading = "[config] Meta options and config file overrides")]
    config_file: ConfigFileArg,

    #[command(flatten)]
    settings: ConfigArgs,

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

    let cli: Cli = parse_cli();

    let config_path = cli.config_file.path.clone()
        .unwrap_or_else(|| "zbobr-fs.toml".into());

    let config_dir = if cli.config_file.path.is_some() {
        std::fs::canonicalize(&config_path)
            .with_context(|| format!("Cannot resolve config path: {}", config_path.display()))?
            .parent()
            .expect("config file must have a parent directory")
            .to_path_buf()
    } else {
        std::env::current_dir()?
    };

    let root_toml = ConfigToml::load(&config_path)?;
    let config = Config::build(root_toml, cli.settings.clone(), &config_dir)?;
    config.dispatcher.validate()?;
    let executor_config = config.executor.clone();

    let task_backend: Arc<dyn zbobr_dispatcher::backend::TaskBackend> = Arc::new(
        ZbobrTaskBackendFs::from_config(config.tasks)
            .context("Failed to initialize filesystem task backend")?,
    );
    let repo_backend: Arc<dyn zbobr_dispatcher::backend::RepoBackend> = Arc::new(
        ZbobrRepoBackendFs::from_config(config.repo)
            .context("Failed to initialize filesystem repo backend")?,
    );

    let zbobr = ZbobrDispatcher::new_with_backends(config.dispatcher.clone(), task_backend, repo_backend);
    zbobr.validate_connectivity().await?;

    let prompts = resolve_prompts(&cli.settings.dispatcher, zbobr.config());

    zbobr_dispatcher::cli::run_command(zbobr, cli.command, &prompts, &executor_config).await
}
