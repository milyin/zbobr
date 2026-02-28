use anyhow::Context;
use zbobr_repo_backend_fs::ZbobrRepoBackendFsConfig;
use zbobr_task_backend_fs::ZbobrTaskBackendFsConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = rustls::crypto::ring::default_provider().install_default();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    zbobr_dispatcher::cli::run_zbobr::<
        zbobr_task_backend_fs::ZbobrTaskBackendFs,
        zbobr_repo_backend_fs::ZbobrRepoBackendFs,
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
        |config| {
            zbobr_task_backend_fs::ZbobrTaskBackendFs::from_config(config)
                .context("Failed to initialize filesystem task backend")
        },
        |config| {
            zbobr_repo_backend_fs::ZbobrRepoBackendFs::from_config(config)
                .context("Failed to initialize filesystem repo backend")
        },
    )
    .await
}
