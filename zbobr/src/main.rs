#![allow(clippy::needless_borrows_for_generic_args)]

mod commands;
mod init;

use anyhow::Context;
use clap::Parser;
use commands::Command;
use zbobr_api::config::{WorkflowArgs, WorkflowConfig, WorkflowToml};
use zbobr_dispatcher::{
    ConfigFileArg,
    config::{
        Config as _, ZbobrDispatcherArgs, ZbobrDispatcherConfig, ZbobrDispatcherToml,
        ZbobrExecutorArgs, ZbobrExecutorConfig, ZbobrExecutorToml,
    },
};
use zbobr_repo_backend_github::{
    ZbobrRepoBackendGithubArgs, ZbobrRepoBackendGithubConfig, ZbobrRepoBackendGithubToml,
};
use zbobr_task_backend_github::{
    ZbobrTaskBackendGithubArgs, ZbobrTaskBackendGithubConfig, ZbobrTaskBackendGithubToml,
};
use zbobr_utility::config_struct;

#[derive(Clone, Default)]
#[config_struct]
struct RootConfig {
    #[config(nested)]
    dispatcher: ZbobrDispatcherConfig,
    #[config(nested)]
    tasks: ZbobrTaskBackendGithubConfig,
    #[config(nested)]
    repo: ZbobrRepoBackendGithubConfig,
    #[config(nested)]
    executor: ZbobrExecutorConfig,
    #[config(nested)]
    workflow: WorkflowConfig,
}

#[derive(Parser)]
struct Cli {
    /// Enable log output to stderr
    #[arg(long)]
    logs: bool,

    #[command(
        flatten,
        next_help_heading = "[config] Meta options and config file overrides"
    )]
    config_file: ConfigFileArg,

    #[command(flatten)]
    settings: RootConfigArgs,

    #[command(subcommand)]
    command: Command,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = rustls::crypto::ring::default_provider().install_default();

    let cli: Cli = zbobr_dispatcher::parse_cli(
        "zbobr",
        "GitHub-backed AI-powered task dispatcher",
        "GitHub-backed AI-powered task dispatcher that manages tasks through automated stages.\n\n\
        Tasks are stored in GitHub issues and work is done via pull requests.\n\
        Requires a GitHub token: set GH_TOKEN or GITHUB_TOKEN env var.\n\
        Easiest way: export GH_TOKEN=$(gh auth token)",
    );

    let filter = if cli.logs {
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into())
    } else {
        "off".into()
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    // Handle init before config loading — no existing config needed
    if let Command::Init {
        ref directory,
        force,
    } = cli.command
    {
        return init::init_workspace(directory, force).await;
    }

    let location = zbobr_dispatcher::resolve_config_location(&cli.config_file.paths, "zbobr.toml")?;

    let root_toml = {
        let mut merged: Option<RootConfigToml> = None;
        for path in &location.config_paths {
            if !path.exists() {
                // When no explicit configs given, the default file may not exist — skip it.
                continue;
            }
            let content = std::fs::read_to_string(path)
                .with_context(|| format!("Failed to read {}", path.display()))?;
            let parsed: RootConfigToml = toml::from_str(&content)
                .with_context(|| format!("Failed to parse {}", path.display()))?;
            // Resolve relative paths against the config file's own directory
            // so that paths from each config stay anchored to their source.
            let file_dir = path
                .parent()
                .expect("config file must have a parent directory");
            let resolved = parsed.resolve_paths(file_dir);
            merged = Some(match merged {
                Some(base) => base.merge_toml(resolved),
                None => resolved,
            });
        }
        merged
    };

    let config = RootConfig::build(root_toml, cli.settings, &location.config_dir);

    commands::run(
        config.dispatcher,
        config.tasks,
        config.repo,
        config.executor,
        config.workflow,
        location.config_dir,
        cli.command,
    )
    .await
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;

    // Test-only Cli struct for testing the Command parser without config parts
    #[derive(Parser)]
    struct TestCli {
        #[command(subcommand)]
        command: commands::Command,
    }

    #[test]
    fn task_process_select_flag_parses_without_task_id() {
        let cli = TestCli::try_parse_from(["zbobr", "task", "process", "--select"]).unwrap();
        match cli.command {
            commands::Command::Task { subcommand } => match subcommand {
                commands::TaskSubcommand::Process { task, select } => {
                    assert_eq!(task, None, "task should be None when --select is used");
                    assert!(select, "--select flag should be true");
                }
                _ => panic!("expected Process subcommand"),
            },
            _ => panic!("expected Task command"),
        }
    }

    #[test]
    fn task_process_explicit_id_parses_without_select() {
        let cli = TestCli::try_parse_from(["zbobr", "task", "process", "42"]).unwrap();
        match cli.command {
            commands::Command::Task { subcommand } => match subcommand {
                commands::TaskSubcommand::Process { task, select } => {
                    assert_eq!(task, Some(42), "task should be Some(42)");
                    assert!(!select, "--select flag should be false");
                }
                _ => panic!("expected Process subcommand"),
            },
            _ => panic!("expected Task command"),
        }
    }

    #[test]
    fn task_process_select_and_task_id_together_is_rejected() {
        let result = TestCli::try_parse_from(["zbobr", "task", "process", "42", "--select"]);
        assert!(
            result.is_err(),
            "should fail: task and --select are mutually exclusive"
        );
    }

    #[test]
    fn task_advance_parses_without_arguments() {
        let cli = TestCli::try_parse_from(["zbobr", "task", "advance"]).unwrap();
        match cli.command {
            commands::Command::Task { subcommand } => match subcommand {
                commands::TaskSubcommand::Advance => {}
                _ => panic!("expected Advance subcommand"),
            },
            _ => panic!("expected Task command"),
        }
    }

    #[test]
    fn logs_flag_defaults_to_false() {
        let cli = Cli::try_parse_from(["zbobr", "task", "process", "--select"]).unwrap();
        assert!(!cli.logs, "logs flag should default to false");
    }

    #[test]
    fn logs_flag_parses_when_present() {
        let cli = Cli::try_parse_from(["zbobr", "--logs", "task", "process", "--select"]).unwrap();
        assert!(cli.logs, "--logs flag should set logs to true");
    }
}
