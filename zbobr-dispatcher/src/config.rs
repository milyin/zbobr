use std::path::{Path, PathBuf};

use zbobr_utility::config_struct;

use crate::task::Tool;

/// Backend type to use.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    Default,
    clap::ValueEnum,
)]
pub enum BackendType {
    #[serde(rename = "github")]
    #[clap(alias = "github")]
    #[default]
    GitHub,
    #[serde(rename = "fs")]
    Filesystem,
}

impl std::fmt::Display for BackendType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackendType::GitHub => write!(f, "github"),
            BackendType::Filesystem => write!(f, "fs"),
        }
    }
}

impl std::str::FromStr for BackendType {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "github" => Ok(BackendType::GitHub),
            "fs" | "filesystem" => Ok(BackendType::Filesystem),
            _ => Err(anyhow::anyhow!("Unknown backend: {}", s)),
        }
    }
}

/// TOML prompts configuration section.
#[derive(Debug, Clone, serde::Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct TomlPrompts {
    pub path: Option<PathBuf>,
    pub preparator: Option<Vec<PathBuf>>,
    pub planner: Option<Vec<PathBuf>>,
    pub worker: Option<Vec<PathBuf>>,
    pub reviewer: Option<Vec<PathBuf>>,
    pub merger: Option<Vec<PathBuf>>,
}

/// Configuration for the zbobr dispatcher.
#[derive(Clone)]
#[config_struct]
pub struct ZbobrDispatcherConfig {
    /// Workspaces directory; each task gets a separate subdirectory.
    pub workspaces: PathBuf,
    /// Read-only GitHub token passed as GH_TOKEN/GITHUB_TOKEN to agent processes.
    /// This is a security boundary: it limits what agents can do on GitHub — agents must
    /// not have write access so that erroneous or misbehaving agents cannot modify remote
    /// repositories. Use a fine-grained token with read-only scopes (or no scopes at all
    /// for fully offline/mcp-tester runs). Defaults to "not-configured" when omitted.
    pub agent_github_token: String,
    // NOTE: `copilot_github_token` has been moved to the Copilot executor
    // configuration; the dispatcher no longer tracks this value.
    /// Backend to use for task storage (github or fs).
    pub task_backend: BackendType,
    /// Backend to use for repository operations (github or fs).
    pub repo_backend: BackendType,
    /// CLI tool to use.
    pub cli_tool: Tool,
    /// Custom prompt files for preparator agent.
    pub preparator_prompts: Vec<PathBuf>,
    /// Custom prompt files for planner agent.
    pub planner_prompts: Vec<PathBuf>,
    /// Custom prompt files for worker agent.
    pub worker_prompts: Vec<PathBuf>,
    /// Custom prompt files for reviewer agent.
    pub reviewer_prompts: Vec<PathBuf>,
    /// Custom prompt files for merger agent.
    pub merger_prompts: Vec<PathBuf>,
    /// Prefix for work branches (default: "zbobr_fix").
    pub work_branch_prefix: String,
    /// Base directory for resolving prompt file paths.
    pub prompts_path: Option<PathBuf>,
    /// Git user name for commits made by the tool.
    pub git_user_name: String,
    /// Git user email for commits made by the tool.
    pub git_user_email: String,
    /// Rewrite commit authors after each stage completes to match configured git user.
    pub overwrite_author: bool,
}

impl Default for ZbobrDispatcherConfig {
    fn default() -> Self {
        Self {
            workspaces: PathBuf::from("./workspaces"),
            agent_github_token: "not-configured".to_string(),
            task_backend: BackendType::default(),
            repo_backend: BackendType::default(),
            cli_tool: Tool::default(),
            preparator_prompts: vec!["prompts/preparator.md".into(), "prompts/common.md".into()],
            planner_prompts: vec!["prompts/planner.md".into(), "prompts/common.md".into()],
            worker_prompts: vec!["prompts/worker.md".into(), "prompts/common.md".into()],
            reviewer_prompts: vec!["prompts/reviewer.md".into(), "prompts/common.md".into()],
            merger_prompts: vec!["prompts/merger.md".into(), "prompts/common.md".into()],
            work_branch_prefix: "zbobr_fix".to_string(),
            prompts_path: None,
            git_user_name: String::new(),
            git_user_email: String::new(),
            overwrite_author: false,
        }
    }
}

// Note: zbobr-specific env helpers were removed — configuration now comes
// from TOML/CLI or explicit external GH env vars. `EnvSource` provides an
// abstraction for reading environment variables in tests.

impl ZbobrDispatcherConfig {
    /// Build configuration by layering: defaults < TOML.
    /// Relative paths from TOML are resolved against `config_dir`.
    ///
    /// Priority: TOML file > hardcoded defaults. Environment variables are not
    /// consulted for zbobr-specific parameters; only external GH token env vars
    /// (`COPILOT_GITHUB_TOKEN`, `GH_TOKEN`, `GITHUB_TOKEN`) are recognized.
    pub fn build(
        toml: Option<ZbobrDispatcherToml>,
        args: ZbobrDispatcherArgs,
        config_dir: &Path,
    ) -> anyhow::Result<Self> {
        let defaults = ZbobrDispatcherConfig::default();
        let merged = toml.unwrap_or_default().merge_with_args(args);

        let workspaces = merged
            .workspaces
            .map(|p| zbobr_utility::resolve_path(p, config_dir))
            .unwrap_or(defaults.workspaces);

        let task_backend = merged.task_backend.unwrap_or(defaults.task_backend);
        let repo_backend = merged.repo_backend.unwrap_or(defaults.repo_backend);

        let cli_tool = merged.cli_tool.unwrap_or(defaults.cli_tool);

        let work_branch_prefix = merged
            .work_branch_prefix
            .unwrap_or(defaults.work_branch_prefix);

        let preparator_prompts = merged
            .preparator_prompts
            .map(|v| {
                v.into_iter()
                    .map(|p| zbobr_utility::resolve_path(p, config_dir))
                    .collect()
            })
            .unwrap_or(defaults.preparator_prompts);

        let planner_prompts = merged
            .planner_prompts
            .map(|v| {
                v.into_iter()
                    .map(|p| zbobr_utility::resolve_path(p, config_dir))
                    .collect()
            })
            .unwrap_or(defaults.planner_prompts);

        let worker_prompts = merged
            .worker_prompts
            .map(|v| {
                v.into_iter()
                    .map(|p| zbobr_utility::resolve_path(p, config_dir))
                    .collect()
            })
            .unwrap_or(defaults.worker_prompts);

        let reviewer_prompts = merged
            .reviewer_prompts
            .map(|v| {
                v.into_iter()
                    .map(|p| zbobr_utility::resolve_path(p, config_dir))
                    .collect()
            })
            .unwrap_or(defaults.reviewer_prompts);

        let merger_prompts = merged
            .merger_prompts
            .map(|v| {
                v.into_iter()
                    .map(|p| zbobr_utility::resolve_path(p, config_dir))
                    .collect()
            })
            .unwrap_or(defaults.merger_prompts);

        let prompts_path = merged
            .prompts_path
            .map(|p| zbobr_utility::resolve_path(p, config_dir));

        // Token resolution with proper priority

        // Agent token: only from TOML/CLI (do not read zbobr-specific env vars)
        let agent_github_token = merged
            .agent_github_token
            .unwrap_or_else(|| defaults.agent_github_token.clone());

        let git_user_name = merged.git_user_name.unwrap_or_default();

        let git_user_email = merged.git_user_email.unwrap_or_default();

        let overwrite_author = merged.overwrite_author.unwrap_or(defaults.overwrite_author);

        Ok(Self {
            workspaces,
            agent_github_token,
            task_backend,
            repo_backend,
            cli_tool,
            preparator_prompts,
            planner_prompts,
            worker_prompts,
            reviewer_prompts,
            merger_prompts,
            work_branch_prefix,
            prompts_path,
            git_user_name,
            git_user_email,
            overwrite_author,
        })
    }

    /// Load configuration from environment variables only (backward compat).
    pub fn from_env() -> anyhow::Result<Self> {
        let cwd = std::env::current_dir()
            .map_err(|e| anyhow::anyhow!("Cannot get current directory: {e}"))?;
        Self::build(None, ZbobrDispatcherArgs::default(), &cwd)
    }

    /// Validate that all required fields are set.
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.git_user_name.is_empty() {
            anyhow::bail!(
                "git user name not set. Use --git-user-name NAME or set git_user_name in the config file.\n  \
                 This is used for git commits made by the tool."
            );
        }
        if self.git_user_email.is_empty() {
            anyhow::bail!(
                "git user email not set. Use --git-user-email EMAIL or set git_user_email in the config file.\n  \
                 This is used for git commits made by the tool."
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    fn test_config_dir() -> PathBuf {
        PathBuf::from("/test/config")
    }

    #[test]
    fn build_with_env_missing_required() {
        let config =
            ZbobrDispatcherConfig::build(None, ZbobrDispatcherArgs::default(), &test_config_dir())
                .expect("build should succeed");
        // validate() should fail because git_user_name and git_user_email are missing
        assert!(config.validate().is_err());
    }

    #[test]
    fn toml_config_parse_minimal() {
        let toml_str = r#""#;
        let config: ZbobrDispatcherToml = toml::from_str(toml_str).unwrap();
        // empty toml yields no fields set
        assert!(config.workspaces.is_none());
    }

    #[test]
    fn toml_config_parse_full() {
        let toml_str = r#"
    workspaces = "/tmp/workspaces"
    cli_tool = "claude"
    work_branch_prefix = "my_fix"
    prompts_path = "/opt/prompts"
    planner_prompts = ["plan.md", "shared.md"]
    worker_prompts = ["work.md"]
    "#;
        let config: ZbobrDispatcherToml = toml::from_str(toml_str).unwrap();
        assert_eq!(config.cli_tool, Some(Tool::Claude));
        assert_eq!(config.prompts_path, Some(PathBuf::from("/opt/prompts")));
        assert_eq!(
            config.planner_prompts,
            Some(vec![PathBuf::from("plan.md"), PathBuf::from("shared.md")])
        );
    }

    #[test]
    fn toml_config_unknown_keys_ignored() {
        let toml_str = r#"
    default_model = "gpt-5-mini"
    unknown_top = "value"

    [prompts]
    path = "/tmp"
    extra = "ignored"

    [unknown_table]
    foo = "bar"
    "#;

        // With deny_unknown_fields, parsing should fail on unknown keys
        let res: Result<ZbobrDispatcherToml, _> = toml::from_str(toml_str);
        assert!(res.is_err());
    }

    #[test]
    fn build_with_toml() {
        let toml = ZbobrDispatcherToml {
            workspaces: Some(PathBuf::from("/tmp/toml-ws")),
            task_backend: None,
            repo_backend: None,
            agent_github_token: Some("toml-agent-token".into()),
            cli_tool: Some(Tool::Claude),
            work_branch_prefix: Some("toml_fix".into()),
            git_user_name: Some("test-user".into()),
            git_user_email: Some("test@example.com".into()),
            overwrite_author: Some(true),
            prompts_path: Some(PathBuf::from("/opt/prompts")),
            preparator_prompts: Some(vec![PathBuf::from("pre.md")]),
            planner_prompts: Some(vec![PathBuf::from("p.md")]),
            worker_prompts: Some(vec![PathBuf::from("w.md")]),
            reviewer_prompts: Some(vec![PathBuf::from("r.md")]),
            merger_prompts: Some(vec![PathBuf::from("m.md")]),
        };

        let config = ZbobrDispatcherConfig::build(
            Some(toml),
            ZbobrDispatcherArgs::default(),
            &test_config_dir(),
        )
        .unwrap();
        // Absolute path stays absolute
        assert_eq!(config.workspaces, PathBuf::from("/tmp/toml-ws"));
        assert_eq!(config.task_backend, BackendType::GitHub);
        assert_eq!(config.repo_backend, BackendType::GitHub);
        assert_eq!(config.cli_tool, Tool::Claude);
        assert_eq!(config.work_branch_prefix, "toml_fix");
        // Relative prompt paths resolved against config_dir
        assert_eq!(
            config.preparator_prompts,
            vec![PathBuf::from("/test/config/pre.md")]
        );
        assert_eq!(
            config.planner_prompts,
            vec![PathBuf::from("/test/config/p.md")]
        );
        assert_eq!(
            config.worker_prompts,
            vec![PathBuf::from("/test/config/w.md")]
        );
        assert_eq!(
            config.reviewer_prompts,
            vec![PathBuf::from("/test/config/r.md")]
        );
        // Absolute prompts_path stays absolute
        assert_eq!(config.prompts_path, Some(PathBuf::from("/opt/prompts")));
        assert_eq!(config.agent_github_token, "toml-agent-token");
        assert_eq!(config.git_user_name, "test-user");
        assert_eq!(config.git_user_email, "test@example.com");
        assert_eq!(config.overwrite_author, true);
    }

    #[test]
    fn build_defaults_without_toml() {
        let config =
            ZbobrDispatcherConfig::build(None, ZbobrDispatcherArgs::default(), &test_config_dir())
                .unwrap();
        assert_eq!(config.task_backend, BackendType::GitHub);
        assert_eq!(config.repo_backend, BackendType::GitHub);
        assert_eq!(config.cli_tool, Tool::Copilot);
        assert_eq!(config.work_branch_prefix, "zbobr_fix");
        assert_eq!(config.workspaces, PathBuf::from("./workspaces"));
        assert_eq!(config.agent_github_token, "not-configured");
        assert_eq!(config.overwrite_author, false);
    }

    #[test]
    fn backend_type_roundtrip() {
        assert_eq!(
            "github".parse::<BackendType>().unwrap(),
            BackendType::GitHub
        );
        assert_eq!(
            "fs".parse::<BackendType>().unwrap(),
            BackendType::Filesystem
        );
        assert_eq!(
            "filesystem".parse::<BackendType>().unwrap(),
            BackendType::Filesystem
        );
        assert!("stub".parse::<BackendType>().is_err());
        assert!("invalid".parse::<BackendType>().is_err());
        assert_eq!(BackendType::GitHub.to_string(), "github");
        assert_eq!(BackendType::Filesystem.to_string(), "fs");
    }

    #[test]
    fn cli_flag_default_false_when_not_specified() {
        // Default is false when neither TOML nor CLI specifies it
        let config =
            ZbobrDispatcherConfig::build(None, ZbobrDispatcherArgs::default(), &test_config_dir())
                .unwrap();
        assert_eq!(config.overwrite_author, false);
    }

    #[test]
    fn toml_overwrite_author_true() {
        // When TOML has overwrite_author = true
        let toml = ZbobrDispatcherToml {
            workspaces: None,
            task_backend: None,
            repo_backend: None,
            agent_github_token: None,
            cli_tool: None,
            work_branch_prefix: None,
            git_user_name: None,
            git_user_email: None,
            overwrite_author: Some(true),
            prompts_path: None,
            preparator_prompts: None,
            planner_prompts: None,
            worker_prompts: None,
            reviewer_prompts: None,
            merger_prompts: None,
        };

        let config = ZbobrDispatcherConfig::build(
            Some(toml),
            ZbobrDispatcherArgs::default(),
            &test_config_dir(),
        )
        .unwrap();

        assert_eq!(config.overwrite_author, true);
    }

    #[test]
    fn toml_overwrite_author_false() {
        // When TOML explicitly has overwrite_author = false
        let toml = ZbobrDispatcherToml {
            workspaces: None,
            task_backend: None,
            repo_backend: None,
            agent_github_token: None,
            cli_tool: None,
            work_branch_prefix: None,
            git_user_name: None,
            git_user_email: None,
            overwrite_author: Some(false),
            prompts_path: None,
            preparator_prompts: None,
            planner_prompts: None,
            worker_prompts: None,
            reviewer_prompts: None,
            merger_prompts: None,
        };

        let config = ZbobrDispatcherConfig::build(
            Some(toml),
            ZbobrDispatcherArgs::default(),
            &test_config_dir(),
        )
        .unwrap();

        assert_eq!(config.overwrite_author, false);
    }

    #[test]
    fn cli_flag_overrides_toml_overwrite_author() {
        // When CLI flag is set, it should override TOML value
        let toml = ZbobrDispatcherToml {
            workspaces: None,
            task_backend: None,
            repo_backend: None,
            agent_github_token: None,
            cli_tool: None,
            work_branch_prefix: None,
            git_user_name: None,
            git_user_email: None,
            overwrite_author: Some(false),
            prompts_path: None,
            preparator_prompts: None,
            planner_prompts: None,
            worker_prompts: None,
            reviewer_prompts: None,
            merger_prompts: None,
        };

        // Create args with CLI flag set to true, overriding TOML false
        let mut args = ZbobrDispatcherArgs::default();
        args.overwrite_author = Some(true);

        let config = ZbobrDispatcherConfig::build(Some(toml), args, &test_config_dir()).unwrap();

        assert_eq!(config.overwrite_author, true);
    }

    #[test]
    fn cli_flag_overrides_default() {
        // When CLI flag is set without TOML, it should override default
        let mut args = ZbobrDispatcherArgs::default();
        args.overwrite_author = Some(true);

        let config = ZbobrDispatcherConfig::build(None, args, &test_config_dir()).unwrap();

        assert_eq!(config.overwrite_author, true);
    }

    #[test]
    fn cli_flag_can_be_false() {
        // When CLI flag is explicitly set to false, it should be false
        let mut args = ZbobrDispatcherArgs::default();
        args.overwrite_author = Some(false);

        let config = ZbobrDispatcherConfig::build(None, args, &test_config_dir()).unwrap();

        assert_eq!(config.overwrite_author, false);
    }
}
