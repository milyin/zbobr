use std::path::Path;

use anyhow::Context;
use zbobr_api::config::{BackendConfig, Config};
use zbobr_executor_claude::ZbobrExecutorClaudeConfig;
use zbobr_executor_copilot::ZbobrExecutorCopilotConfig;
use zbobr_executor_mcp_tester::ZbobrExecutorMcpTesterConfig;

use crate::config::{
    ZbobrDispatcherArgs, ZbobrDispatcherConfig, ZbobrDispatcherToml, ZbobrExecutorArgs,
    ZbobrExecutorConfig, ZbobrExecutorToml,
};

// ---------------------------------------------------------------------------
// GenericConfigToml
// ---------------------------------------------------------------------------

/// Root TOML config parametrized by task and repo backend config types.
///
/// Sections:
/// - `[dispatcher]` — dispatcher runtime settings
/// - `[tasks]`      — task backend settings (type `TC::Toml`)
/// - `[repo]`       — repo backend settings (type `RC::Toml`)
/// - `[executor]`   — executor (AI tool) settings
#[derive(serde::Deserialize)]
#[serde(
    default,
    deny_unknown_fields,
    bound(
        deserialize = "TC::Toml: serde::de::DeserializeOwned, RC::Toml: serde::de::DeserializeOwned"
    )
)]
pub struct GenericConfigToml<TC: Config, RC: Config> {
    pub dispatcher: Option<ZbobrDispatcherToml>,
    pub tasks: Option<TC::Toml>,
    pub repo: Option<RC::Toml>,
    pub executor: Option<ZbobrExecutorToml>,
}

impl<TC: Config, RC: Config> Default for GenericConfigToml<TC, RC> {
    fn default() -> Self {
        Self {
            dispatcher: None,
            tasks: None,
            repo: None,
            executor: None,
        }
    }
}

impl<TC: Config, RC: Config> GenericConfigToml<TC, RC> {
    /// Load from a TOML file. Returns `Ok(None)` if the file does not exist.
    pub fn load(path: &Path) -> anyhow::Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        let config: Self = toml::from_str(&content)
            .with_context(|| format!("Failed to parse {}", path.display()))?;
        Ok(Some(config))
    }
}

// ---------------------------------------------------------------------------
// GenericConfigArgs
// ---------------------------------------------------------------------------

/// CLI args parametrized by task and repo backend arg types.
///
/// The dispatcher and executor args are always included. Task and repo backend
/// args are provided by the caller via `TA` and `RA` type parameters, which are
/// the `Args` associated types of the respective `Config` implementations.
///
/// Help headings: `[tasks]` is applied to `TA` args, `[repo]` to `RA` args.
#[derive(clap::Args, Clone, Default, Debug)]
pub struct GenericConfigArgs<TA, RA>
where
    TA: clap::Args + Default + Clone + std::fmt::Debug,
    RA: clap::Args + Default + Clone + std::fmt::Debug,
{
    #[command(flatten)]
    pub dispatcher: ZbobrDispatcherArgs,

    #[command(flatten, next_help_heading = "[tasks]")]
    pub tasks: TA,

    #[command(flatten, next_help_heading = "[repo]")]
    pub repo: RA,

    #[command(flatten)]
    pub executor: ZbobrExecutorArgs,
}

// ---------------------------------------------------------------------------
// GenericConfig
// ---------------------------------------------------------------------------

/// Resolved configuration parametrized by task and repo backend config types.
pub struct GenericConfig<TC: Config + BackendConfig, RC: Config + BackendConfig> {
    pub dispatcher: ZbobrDispatcherConfig,
    pub tasks: TC,
    pub repo: RC,
    pub executor: ZbobrExecutorConfig,
}

impl<TC: Config + BackendConfig, RC: Config + BackendConfig> GenericConfig<TC, RC>
where
    TC::Args: std::fmt::Debug,
    RC::Args: std::fmt::Debug,
{
    pub fn build(
        toml: Option<GenericConfigToml<TC, RC>>,
        args: GenericConfigArgs<TC::Args, RC::Args>,
        config_dir: &Path,
    ) -> anyhow::Result<Self> {
        let toml = toml.unwrap_or_default();

        let dispatcher =
            ZbobrDispatcherConfig::build(toml.dispatcher, args.dispatcher, config_dir);

        let tasks = TC::build(toml.tasks, args.tasks, config_dir);
        let repo = RC::build(toml.repo, args.repo, config_dir);

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
