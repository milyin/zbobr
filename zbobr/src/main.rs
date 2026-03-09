#![allow(clippy::needless_borrows_for_generic_args)]

use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = rustls::crypto::ring::default_provider().install_default();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    zbobr_dispatcher::cli::run_zbobr(
        "zbobr",
        "GitHub-backed AI-powered task dispatcher",
        "GitHub-backed AI-powered task dispatcher that manages tasks through automated stages.\n\n\
        Tasks are stored in GitHub issues and work is done via pull requests.\n\
        Tasks flow through: PENDING -> PREPARING -> PLANNING -> WORKING -> REVIEWING -> DONE.\n\
        Merge conflicts are handled by MERGING sessions when the conflict flag is set.\n\n\
        Requires a GitHub token: set GH_TOKEN or GITHUB_TOKEN env var.\n\
        Easiest way: export GH_TOKEN=$(gh auth token)",
        "zbobr.toml",
        |tc, rc, dispatcher| {
            use zbobr_dispatcher::BackendConfig;
            let task_backend: Arc<dyn zbobr_dispatcher::backend::TaskBackend> =
                Arc::new(tc.github.build_backend(dispatcher)?);
            let repo_backend: Arc<dyn zbobr_dispatcher::backend::WorktreeBackend> =
                Arc::new(rc.github.build_backend(dispatcher)?);
            Ok((task_backend, repo_backend))
        },
    )
    .await?;

    Ok(())
}
