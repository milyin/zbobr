use zbobr_dispatcher::backend::{TaskBackend as _, WorktreeBackend as _};

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

    let (config, command) = zbobr_dispatcher::init_config::<
        ZbobrTaskBackendFsConfig,
        ZbobrRepoBackendFsConfig,
    >(
        "zbobr-fs",
        "Filesystem-backed AI-powered task dispatcher",
        "Filesystem-backed AI-powered task dispatcher that manages tasks through automated stages.\n\n\
        Tasks are stored in YAML files and work is done on local git clones.\n\
        Tasks flow through: PENDING -> PREPARING -> PLANNING -> WORKING -> REVIEWING -> DONE.\n\
        Merge conflicts are handled by MERGING sessions when the conflict flag is set.\n\n\
        Ideal for testing, local development, and offline scenarios.\n\n\
        Default config file: zbobr-fs.toml in current directory.",
        "zbobr-fs.toml",
    )?;

    let executor_config = config.executor.clone();

    let task_backend = ArcTaskBackendFs::new(ZbobrTaskBackendFs::from_config(config.tasks)?);
    let repo_backend = ZbobrRepoBackendFs::from_config(config.repo)?;

    let zbobr = zbobr_dispatcher::ZbobrDispatcher::new(config.dispatcher);
    task_backend.validate_connectivity().await?;
    repo_backend.validate_connectivity().await?;

    let prompt_builder = zbobr_dispatcher::ConfiguredPromptBuilder::from(config.prompts);
    prompt_builder.validate()?;

    zbobr_dispatcher::run_command(
        zbobr,
        task_backend,
        repo_backend,
        command,
        &prompt_builder,
        &executor_config,
    )
    .await
}
