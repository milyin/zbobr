use std::sync::Arc;

use anyhow::Context;
use zbobr_repo_backend_fs::{ZbobrRepoBackendFs, ZbobrRepoBackendFsConfig};
use zbobr_task_backend_fs::{ArcTaskBackendFs, ZbobrTaskBackendFs, ZbobrTaskBackendFsConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = rustls::crypto::ring::default_provider().install_default();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let cli: zbobr_dispatcher::cli::GenericCli<
        <ZbobrTaskBackendFsConfig as zbobr_dispatcher::Config>::Args,
        <ZbobrRepoBackendFsConfig as zbobr_dispatcher::Config>::Args,
    > = zbobr_dispatcher::parse_cli(
        "zbobr-fs",
        "Filesystem-backed AI-powered task dispatcher",
        "Filesystem-backed AI-powered task dispatcher that manages tasks through automated stages.\n\n\
        Tasks are stored in YAML files and work is done on local git clones.\n\
        Tasks flow through: PENDING -> PREPARING -> PLANNING -> WORKING -> REVIEWING -> DONE.\n\
        Merge conflicts are handled by MERGING sessions when the conflict flag is set.\n\n\
        Ideal for testing, local development, and offline scenarios.\n\n\
        Default config file: zbobr-fs.toml in current directory.",
    );

    let loc = zbobr_dispatcher::resolve_config_location(&cli.config_file.path, "zbobr-fs.toml")?;
    let config = zbobr_dispatcher::load_config::<ZbobrTaskBackendFsConfig, ZbobrRepoBackendFsConfig>(
        &loc,
        cli.settings.clone(),
    )
    .with_context(|| format!("Config file: {}", loc.config_path.display()))?;

    let executor_config = config.executor.clone();

    let task_backend: Arc<dyn zbobr_dispatcher::backend::TaskBackend> =
        Arc::new(ArcTaskBackendFs::new(ZbobrTaskBackendFs::from_config(config.tasks)?));
    let repo_backend: Arc<dyn zbobr_dispatcher::backend::WorktreeBackend> =
        Arc::new(ZbobrRepoBackendFs::from_config(config.repo)?);

    let zbobr = zbobr_dispatcher::ZbobrDispatcher::new(config.dispatcher);
    task_backend.validate_connectivity().await?;
    repo_backend.validate_connectivity().await?;

    let prompts =
        zbobr_dispatcher::resolve_prompts(&cli.settings.dispatcher, zbobr.config());
    zbobr_dispatcher::prompts::validate_prompts(&prompts)?;

    zbobr_dispatcher::run_command(
        zbobr,
        task_backend,
        repo_backend,
        cli.command,
        &prompts,
        &executor_config,
    )
    .await
}
