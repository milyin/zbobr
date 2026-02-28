use std::path::{Path, PathBuf};

use anyhow::Context;
use zbobr_dispatcher::{
    ZbobrDispatcherArgs, ZbobrDispatcherConfig, ZbobrDispatcherToml,
    ZbobrExecutorConfig, ZbobrExecutorConfigArgs, ZbobrExecutorConfigToml,
};
use zbobr_executor_claude::{ZbobrExecutorClaudeConfig};
use zbobr_executor_copilot::{ZbobrExecutorCopilotConfig};
use zbobr_executor_mcp_tester::ZbobrExecutorMcpTesterConfig;
use zbobr_task_backend_fs::{ZbobrTaskBackendFsArgs, ZbobrTaskBackendFsConfig, ZbobrTaskBackendFsToml};
use zbobr_repo_backend_fs::{ZbobrRepoBackendFsArgs, ZbobrRepoBackendFsConfig, ZbobrRepoBackendFsToml};

// ---------------------------------------------------------------------------
// TOML types
// ---------------------------------------------------------------------------

/// TOML `[tasks]` section: filesystem task backend.
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TasksToml {
    /// Directory for task YAML files
    pub dir: Option<PathBuf>,
}

/// TOML `[repo]` section: filesystem repo backend.
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RepoToml {
    /// Directory for repository clones
    pub dir: Option<PathBuf>,
}

/// Root TOML config for zbobr-fs.
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ConfigToml {
    pub dispatcher: Option<ZbobrDispatcherToml>,
    pub tasks: Option<TasksToml>,
    pub repo: Option<RepoToml>,
    pub executor: Option<ZbobrExecutorConfigToml>,
}

impl ConfigToml {
    pub fn load(path: &Path) -> anyhow::Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        let config: ConfigToml = toml::from_str(&content)
            .with_context(|| format!("Failed to parse {}", path.display()))?;
        Ok(Some(config))
    }
}

// ---------------------------------------------------------------------------
// CLI args
// ---------------------------------------------------------------------------

/// CLI args for zbobr-fs: dispatcher + filesystem backend options only.
#[derive(clap::Args, Clone, Default, Debug)]
pub struct ConfigArgs {
    #[command(flatten)]
    pub dispatcher: ZbobrDispatcherArgs,

    /// Directory for task YAML files (default: ./tasks)
    #[arg(long = "tasks-dir", help_heading = "[tasks]")]
    pub tasks_dir: Option<PathBuf>,

    /// Directory for repository clones (default: ./repos)
    #[arg(long = "repo-dir", help_heading = "[repo]")]
    pub repos_dir: Option<PathBuf>,

    #[command(flatten)]
    pub executor: ZbobrExecutorConfigArgs,
}

// ---------------------------------------------------------------------------
// Resolved config
// ---------------------------------------------------------------------------

pub struct Config {
    pub dispatcher: ZbobrDispatcherConfig,
    pub tasks: ZbobrTaskBackendFsConfig,
    pub repo: ZbobrRepoBackendFsConfig,
    pub executor: ZbobrExecutorConfig,
}

impl Config {
    pub fn build(
        toml: Option<ConfigToml>,
        args: ConfigArgs,
        config_dir: &Path,
    ) -> anyhow::Result<Self> {
        let toml = toml.unwrap_or_default();

        let dispatcher =
            ZbobrDispatcherConfig::build(toml.dispatcher, args.dispatcher, config_dir)?;

        let tasks = {
            let t = toml.tasks.unwrap_or_default();
            let tasks_toml = t.dir.map(|dir| ZbobrTaskBackendFsToml { tasks_dir: Some(dir) });
            let tasks_args = ZbobrTaskBackendFsArgs { tasks_dir: args.tasks_dir };
            ZbobrTaskBackendFsConfig::build(tasks_toml, tasks_args, config_dir)
        };

        let repo = {
            let r = toml.repo.unwrap_or_default();
            let repo_toml = r.dir.map(|dir| ZbobrRepoBackendFsToml { repos_dir: Some(dir) });
            let repo_args = ZbobrRepoBackendFsArgs { repos_dir: args.repos_dir };
            ZbobrRepoBackendFsConfig::build(repo_toml, repo_args, config_dir)
        };

        let executor = {
            let t = toml.executor.unwrap_or_default();
            ZbobrExecutorConfig {
                claude: ZbobrExecutorClaudeConfig::build(t.claude, args.executor.claude),
                copilot: ZbobrExecutorCopilotConfig::build(t.copilot, args.executor.copilot),
                mcp_tester: ZbobrExecutorMcpTesterConfig::build(
                    t.mcp_tester,
                    args.executor.mcp_tester,
                    config_dir,
                ),
            }
        };

        Ok(Self {
            dispatcher,
            tasks,
            repo,
            executor,
        })
    }
}
