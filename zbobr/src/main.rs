#![allow(clippy::needless_borrows_for_generic_args)]

use std::sync::Arc;

use anyhow::Context;
use zbobr_repo_backend_github::{ZbobrRepoBackendGithub, ZbobrRepoBackendGithubConfig};
use zbobr_task_backend_github::{
    ArcTaskBackendGithub, ZbobrTaskBackendGithub, ZbobrTaskBackendGithubConfig,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = rustls::crypto::ring::default_provider().install_default();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let cli: zbobr_dispatcher::cli::GenericCli<
        <ZbobrTaskBackendGithubConfig as zbobr_dispatcher::Config>::Args,
        <ZbobrRepoBackendGithubConfig as zbobr_dispatcher::Config>::Args,
    > = zbobr_dispatcher::parse_cli(
        "zbobr",
        "GitHub-backed AI-powered task dispatcher",
        "GitHub-backed AI-powered task dispatcher that manages tasks through automated stages.\n\n\
        Tasks are stored in GitHub issues and work is done via pull requests.\n\
        Tasks flow through: PENDING -> PREPARING -> PLANNING -> WORKING -> REVIEWING -> DONE.\n\
        Merge conflicts are handled by MERGING sessions when the conflict flag is set.\n\n\
        Requires a GitHub token: set GH_TOKEN or GITHUB_TOKEN env var.\n\
        Easiest way: export GH_TOKEN=$(gh auth token)",
    );

    let loc = zbobr_dispatcher::resolve_config_location(&cli.config_file.path, "zbobr.toml")?;
    let config = zbobr_dispatcher::load_config::<ZbobrTaskBackendGithubConfig, ZbobrRepoBackendGithubConfig>(
        &loc,
        cli.settings.clone(),
    )
    .with_context(|| format!("Config file: {}", loc.config_path.display()))?;

    let executor_config = config.executor.clone();

    let task_backend: Arc<dyn zbobr_dispatcher::backend::TaskBackend> = Arc::new(
        ArcTaskBackendGithub::new(ZbobrTaskBackendGithub::from_config(config.tasks)?),
    );
    let repo_backend: Arc<dyn zbobr_dispatcher::backend::WorktreeBackend> =
        Arc::new(ZbobrRepoBackendGithub::from_config(config.repo)?);

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
