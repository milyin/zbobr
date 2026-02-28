#![allow(clippy::needless_borrows_for_generic_args)]
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use clap::{Args, Parser, Subcommand};
use zbobr_dispatcher::{
    ZbobrDispatcher, ZbobrConfig, ZbobrConfigArgs, ZbobrConfigToml,
};
use zbobr_task_backend_github::ZbobrTaskBackendGithub;
use zbobr_repo_backend_github::ZbobrRepoBackendGithub;

#[derive(Args, Clone)]
struct GlobalArgs {
    #[command(
        flatten,
        next_help_heading = "[config] Meta options and config file overrides"
    )]
    config_file: ConfigFileArg,

    #[command(flatten)]
    settings: ZbobrConfigArgs,
}

#[derive(Args, Clone)]
struct ConfigFileArg {
    /// Path to TOML configuration file (default: zbobr-github.toml in cwd)
    #[arg(long = "config")]
    pub path: Option<PathBuf>,
}

#[derive(Parser)]
#[command(
    name = "zbobr-github",
    about = "GitHub-backed AI-powered task dispatcher",
    long_about = "GitHub-backed AI-powered task dispatcher that manages tasks through automated stages.\n\n\
        Tasks are stored in GitHub issues and work is done via pull requests.\n\
        Tasks flow through: PENDING -> PREPARING -> PLANNING -> WORKING -> REVIEWING -> DONE.\n\
        Merge conflicts are handled by MERGING sessions when the conflict flag is set.\n\n\
        Requires a GitHub token: set GH_TOKEN or GITHUB_TOKEN env var.\n\
        Easiest way: export GH_TOKEN=$(gh auth token)"
)]
struct Cli {
    #[command(flatten)]
    global: GlobalArgs,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Initialize a task project: create repo if needed, set up stages and labels
    Setup {
        /// Force overwrite existing labels
        #[arg(long, short = 'f')]
        force: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    // Resolve config file path (default to zbobr-github.toml)
    let config_path = cli.global.config_file.path.clone().unwrap_or_else(|| {
        PathBuf::from("zbobr-github.toml")
    });

    let config_dir = config_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .to_path_buf();

    // Load configuration
    let toml = ZbobrConfigToml::load(&config_path)?;
    let config = ZbobrConfig::build(toml, cli.global.settings, &config_dir)?;

    // Validate config
    config.dispatcher.validate()?;

    // Create GitHub-backed dispatcher
    let task_backend = Arc::new(
        ZbobrTaskBackendGithub::from_config(config.tasks.github)
            .context("Failed to initialize GitHub task backend")?
    );
    let repo_backend = Arc::new(
        ZbobrRepoBackendGithub::from_config(
            config.repo.github,
            config.dispatcher.git_user_name.clone(),
            config.dispatcher.git_user_email.clone(),
        ).context("Failed to initialize GitHub repo backend")?
    );

    let dispatcher = ZbobrDispatcher::new_with_backends(
        config.dispatcher.clone(),
        task_backend,
        repo_backend,
    );

    // Validate backends
    dispatcher.validate_connectivity().await?;

    match cli.command {
        Command::Setup { force } => {
            dispatcher.setup_repository(force).await?;
            println!("Repository setup completed successfully");
        }
    }

    Ok(())
}
