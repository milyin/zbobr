#![allow(clippy::needless_borrows_for_generic_args)]
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use clap::{Args, Parser, Subcommand};
use zbobr_dispatcher::{
    ZbobrDispatcher, ZbobrConfig, ZbobrConfigArgs, ZbobrConfigToml,
};
use zbobr_task_backend_fs::ZbobrTaskBackendFs;
use zbobr_repo_backend_fs::ZbobrRepoBackendFs;

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
    /// Path to TOML configuration file (default: zbobr-fs.toml in cwd)
    #[arg(long = "config")]
    pub path: Option<PathBuf>,
}

#[derive(Parser)]
#[command(
    name = "zbobr-fs",
    about = "Filesystem-backed AI-powered task dispatcher",
    long_about = "Filesystem-backed AI-powered task dispatcher that manages tasks through automated stages.\n\n\
        Tasks are stored in YAML files and work is done on local clones.\n\
        Tasks flow through: PENDING -> PREPARING -> PLANNING -> WORKING -> REVIEWING -> DONE.\n\
        Merge conflicts are handled by MERGING sessions when the conflict flag is set.\n\n\
        Ideal for testing, local development, and offline scenarios."
)]
struct Cli {
    #[command(flatten)]
    global: GlobalArgs,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Initialize a task project
    Setup {
        /// Force overwrite existing configuration
        #[arg(long, short = 'f')]
        force: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    // Resolve config file path (default to zbobr-fs.toml)
    let config_path = cli.global.config_file.path.clone().unwrap_or_else(|| {
        PathBuf::from("zbobr-fs.toml")
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

    // Create filesystem-backed dispatcher
    let task_backend = Arc::new(
        ZbobrTaskBackendFs::from_config(config.tasks.fs)
            .context("Failed to initialize filesystem task backend")?
    );
    let repo_backend = Arc::new(
        ZbobrRepoBackendFs::from_config(config.repo.fs)
            .context("Failed to initialize filesystem repo backend")?
    );

    let dispatcher = ZbobrDispatcher::new_with_backends(
        config.dispatcher.clone(),
        task_backend,
        repo_backend,
    );

    // Validate backends
    dispatcher.validate_connectivity().await?;

    match cli.command {
        Command::Setup { force: _ } => {
            println!("Filesystem backend initialized successfully");
        }
    }

    Ok(())
}
