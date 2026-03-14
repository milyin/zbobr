use std::sync::Arc;

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

    let init = zbobr_dispatcher::init_config::<ZbobrTaskBackendFsConfig, ZbobrRepoBackendFsConfig>(
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

    let executor_config = init.config.executor.clone();

    let task_backend: Arc<dyn zbobr_dispatcher::backend::TaskBackend> =
        Arc::new(ArcTaskBackendFs::new(ZbobrTaskBackendFs::from_config(init.config.tasks)?));
    let repo_backend: Arc<dyn zbobr_dispatcher::backend::WorktreeBackend> =
        Arc::new(ZbobrRepoBackendFs::from_config(init.config.repo)?);

    let zbobr = zbobr_dispatcher::ZbobrDispatcher::new(init.config.dispatcher);
    task_backend.validate_connectivity().await?;
    repo_backend.validate_connectivity().await?;

    let prompts =
        zbobr_dispatcher::resolve_prompts(&init.dispatcher_args, zbobr.config());
    zbobr_dispatcher::prompts::validate_prompts(&prompts)?;

    zbobr_dispatcher::run_command(
        zbobr,
        task_backend,
        repo_backend,
        init.command,
        &prompts,
        &executor_config,
    )
    .await
}
