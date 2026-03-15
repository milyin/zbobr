#![allow(clippy::needless_borrows_for_generic_args)]

use zbobr_dispatcher::backend::{TaskBackend as _, WorktreeBackend as _};

use zbobr_repo_backend_github::{ZbobrRepoBackendGithub, ZbobrRepoBackendGithubConfig};
use zbobr_task_backend_github::{TaskBackendGithub, ZbobrTaskBackendGithubConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = rustls::crypto::ring::default_provider().install_default();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let (config, command) = zbobr_dispatcher::init_config::<
        ZbobrTaskBackendGithubConfig,
        ZbobrRepoBackendGithubConfig,
    >(
        "zbobr",
        "GitHub-backed AI-powered task dispatcher",
        "GitHub-backed AI-powered task dispatcher that manages tasks through automated stages.\n\n\
        Tasks are stored in GitHub issues and work is done via pull requests.\n\
        Tasks flow through: PENDING -> PREPARING -> PLANNING -> WORKING -> REVIEWING -> DONE.\n\
        Merge conflicts are handled by MERGING sessions when the conflict flag is set.\n\n\
        Requires a GitHub token: set GH_TOKEN or GITHUB_TOKEN env var.\n\
        Easiest way: export GH_TOKEN=$(gh auth token)",
        "zbobr.toml",
    )?;

    let dispatcher =
        zbobr_dispatcher::ZbobrDispatcher::new_with_executors(config.dispatcher, config.executor);

    let task_backend = TaskBackendGithub::from_config(config.tasks)?;
    let repo_backend = ZbobrRepoBackendGithub::from_config(config.repo)?;
    task_backend.validate_connectivity().await?;
    repo_backend.validate_connectivity().await?;

    let prompt_builder = zbobr_dispatcher::ConfiguredPromptBuilder::from(config.prompts);
    prompt_builder.validate()?;

    zbobr_dispatcher::run_command(
        dispatcher,
        task_backend,
        repo_backend,
        command,
        prompt_builder,
    )
    .await
}
