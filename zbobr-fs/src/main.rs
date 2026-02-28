use anyhow::Context;
use clap::Parser;
use zbobr_dispatcher::{
    GenericConfigArgs,
    cli::{Command, ConfigFileArg, parse_cli},
};
use zbobr_repo_backend_fs::ZbobrRepoBackendFsConfig;
use zbobr_task_backend_fs::ZbobrTaskBackendFsConfig;

type ConfigArgs = GenericConfigArgs<
    <ZbobrTaskBackendFsConfig as zbobr_dispatcher::BackendConfig>::Args,
    <ZbobrRepoBackendFsConfig as zbobr_dispatcher::BackendConfig>::Args,
>;

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
    #[command(
        flatten,
        next_help_heading = "[config] Meta options and config file overrides"
    )]
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

    zbobr_dispatcher::cli::run_zbobr(
        cli.config_file.path,
        cli.settings,
        cli.command,
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
