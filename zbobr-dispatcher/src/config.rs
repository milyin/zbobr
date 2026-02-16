use std::path::PathBuf;

use crate::{
    task::{Model, Tool},
    ZbobrError,
};

/// Backend type to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[derive(Default)]
pub enum BackendType {
    #[serde(rename = "github")]
    #[default]
    GitHub,
}


impl std::fmt::Display for BackendType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackendType::GitHub => write!(f, "github"),
        }
    }
}

impl std::str::FromStr for BackendType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "github" => Ok(BackendType::GitHub),
            _ => Err(format!("Unknown backend: {}", s)),
        }
    }
}

/// TOML prompts configuration section.
#[derive(Debug, Clone, serde::Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct TomlPrompts {
    pub path: Option<PathBuf>,
    pub planner: Option<Vec<PathBuf>>,
    pub worker: Option<Vec<PathBuf>>,
    pub reviewer: Option<Vec<PathBuf>>,
    pub merger: Option<Vec<PathBuf>>,
}

/// Configuration loaded from a TOML file.
/// All fields are optional — missing fields fall back to TOML or defaults.
#[derive(Debug, Clone, serde::Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct ZbobrDispatcherToml {
    pub default_model: Option<Model>,
    pub workspace: Option<PathBuf>,
    pub agent_github_token: Option<String>,
    pub copilot_github_token: Option<String>,
    pub cli_tool: Option<Tool>,
    pub work_branch_prefix: Option<String>,
    pub git_user_name: Option<String>,
    pub git_user_email: Option<String>,
    pub prompts: Option<TomlPrompts>,
}

/// Configuration for the zbobr dispatcher.
#[derive(Debug, Clone)]
pub struct ZbobrDispatcherConfig {
    /// Default AI model to use.
    pub default_model: Model,
    /// Workspace directory for issue work dirs.
    pub workspace: PathBuf,
    /// GitHub token with read-only access for agent processes (passed as GH_TOKEN to agents).
    pub agent_github_token: String,
    /// GitHub token for Copilot CLI with Copilot's access rights (passed as COPILOT_GITHUB_TOKEN).
    pub copilot_github_token: String,
    /// Backend to use.
    pub backend: BackendType,
    /// CLI tool to use.
    pub cli_tool: Tool,
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
}

impl Default for ZbobrDispatcherConfig {
    fn default() -> Self {
        Self {
            default_model: Model::default(),
            workspace: PathBuf::from("./workspace"),
            agent_github_token: String::new(),
            copilot_github_token: String::new(),
            backend: BackendType::default(),
            cli_tool: Tool::default(),
            planner_prompts: vec!["prompts/planner.md".into(), "prompts/common.md".into()],
            worker_prompts: vec!["prompts/worker.md".into(), "prompts/common.md".into()],
            reviewer_prompts: vec!["prompts/reviewer.md".into(), "prompts/common.md".into()],
            merger_prompts: vec!["prompts/merger.md".into(), "prompts/common.md".into()],
            work_branch_prefix: "zbobr_fix".to_string(),
            prompts_path: None,
            git_user_name: String::new(),
            git_user_email: String::new(),
        }
    }
}

trait EnvSource {
    fn var(&self, key: &str) -> Option<String>;
}

struct OsEnv;

impl EnvSource for OsEnv {
    fn var(&self, key: &str) -> Option<String> {
        std::env::var(key).ok()
    }
}

// Note: zbobr-specific env helpers were removed — configuration now comes
// from TOML/CLI or explicit external GH env vars. `EnvSource` provides an
// abstraction for reading environment variables in tests.

impl ZbobrDispatcherConfig {
    /// Build configuration by layering: defaults < TOML.
    ///
    /// Priority: TOML file > hardcoded defaults. Environment variables are not
    /// consulted for zbobr-specific parameters; only external GH token env vars
    /// (`COPILOT_GITHUB_TOKEN`, `GH_TOKEN`, `GITHUB_TOKEN`) are recognized.
    pub fn build(toml: Option<&ZbobrDispatcherToml>) -> Result<Self, ZbobrError> {
        let env = OsEnv;
        Self::build_with_env(toml, &env)
    }

    fn build_with_env<E: EnvSource>(toml: Option<&ZbobrDispatcherToml>, env: &E) -> Result<Self, ZbobrError> {
        let defaults = ZbobrDispatcherConfig::default();

        let default_model = toml
            .and_then(|t| t.default_model.clone())
            .unwrap_or(defaults.default_model);

        let workspace = toml
            .and_then(|t| t.workspace.clone())
            .unwrap_or(defaults.workspace);

        let backend = defaults.backend;

        let cli_tool = toml.and_then(|t| t.cli_tool).unwrap_or(defaults.cli_tool);

        let work_branch_prefix = toml
            .and_then(|t| t.work_branch_prefix.clone())
            .unwrap_or(defaults.work_branch_prefix);

        let toml_prompts = toml.and_then(|t| t.prompts.as_ref());

        let planner_prompts = toml_prompts
            .and_then(|p| p.planner.clone())
            .unwrap_or(defaults.planner_prompts);

        let worker_prompts = toml_prompts
            .and_then(|p| p.worker.clone())
            .unwrap_or(defaults.worker_prompts);

        let reviewer_prompts = toml_prompts
            .and_then(|p| p.reviewer.clone())
            .unwrap_or(defaults.reviewer_prompts);

        let merger_prompts = toml_prompts
            .and_then(|p| p.merger.clone())
            .unwrap_or(defaults.merger_prompts);

        let prompts_path = toml_prompts.and_then(|p| p.path.clone());

        // Token resolution with proper priority

        // COPILOT_GITHUB_TOKEN: check external vars then TOML
        let copilot_github_token = env
            .var("COPILOT_GITHUB_TOKEN")
            .or_else(|| env.var("GH_TOKEN"))
            .or_else(|| env.var("GITHUB_TOKEN"))
            .or_else(|| toml.and_then(|t| t.copilot_github_token.clone()))
            .unwrap_or_default();

        // Agent token: only from TOML/CLI (do not read zbobr-specific env vars)
        let agent_github_token = toml
            .and_then(|t| t.agent_github_token.clone())
            .unwrap_or_default();

        let git_user_name = toml
            .and_then(|t| t.git_user_name.clone())
            .unwrap_or_default();

        let git_user_email = toml
            .and_then(|t| t.git_user_email.clone())
            .unwrap_or_default();

        Ok(Self {
            default_model,
            workspace,
            agent_github_token,
            copilot_github_token,
            backend,
            cli_tool,
            planner_prompts,
            worker_prompts,
            reviewer_prompts,
            merger_prompts,
            work_branch_prefix,
            prompts_path,
            git_user_name,
            git_user_email,
        })
    }

    /// Load configuration from environment variables only (backward compat).
    pub fn from_env() -> Result<Self, ZbobrError> {
        Self::build(None)
    }

    /// Validate that all required fields are set.
    pub fn validate(&self) -> Result<(), ZbobrError> {
        if self.agent_github_token.is_empty() {
            return Err(ZbobrError::Config(
                "agent GitHub token not set. Set agent_github_token in the config file or via CLI.\n  \
                 This must be a token with read-only access for agent processes."
                    .into(),
            ));
        }
        if self.git_user_name.is_empty() {
            return Err(ZbobrError::Config(
                "git user name not set. Use --git-user-name NAME or set git_user_name in the config file.\n  \
                 This is used for git commits made by the tool."
                    .into(),
            ));
        }
        if self.git_user_email.is_empty() {
            return Err(ZbobrError::Config(
                "git user email not set. Use --git-user-email EMAIL or set git_user_email in the config file.\n  \
                 This is used for git commits made by the tool."
                    .into(),
            ));
        }
        Ok(())
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct TestEnv {
        vars: HashMap<String, String>,
    }

    impl TestEnv {
        fn new(vars: &[(&str, &str)]) -> Self {
            let mut map = HashMap::new();
            for (key, value) in vars {
                map.insert((*key).to_string(), (*value).to_string());
            }
            Self { vars: map }
        }
    }

    impl EnvSource for TestEnv {
        fn var(&self, key: &str) -> Option<String> {
            self.vars.get(key).cloned()
        }
    }

    #[test]
    fn build_with_env_missing_required() {
        let env = TestEnv::new(&[]);

        let config =
            ZbobrDispatcherConfig::build_with_env(None, &env).expect("build should succeed");
        // validate() should fail because agent_github_token is missing
        assert!(config.validate().is_err());
    }

    #[test]
    fn toml_config_parse_minimal() {
        let toml_str = r#"
    default_model = "gpt-5-mini"
    "#;
        let config: ZbobrDispatcherToml = toml::from_str(toml_str).unwrap();
        assert_eq!(config.default_model, Some(Model::Gpt5Mini));
    }

    #[test]
    fn toml_config_parse_full() {
        let toml_str = r#"
    default_model = "gpt-5-mini"
    workspace = "/tmp/workspace"
    cli_tool = "claude"
    work_branch_prefix = "my_fix"

    [prompts]
    path = "/opt/prompts"
    planner = ["plan.md", "shared.md"]
    worker = ["work.md"]
    "#;
        let config: ZbobrDispatcherToml = toml::from_str(toml_str).unwrap();
        assert_eq!(config.default_model, Some(Model::Gpt5Mini));
        assert_eq!(config.cli_tool, Some(Tool::Claude));
        let prompts = config.prompts.unwrap();
        assert_eq!(prompts.path, Some(PathBuf::from("/opt/prompts")));
        assert_eq!(
            prompts.planner,
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
        let env = TestEnv::new(&[]);
        let toml = ZbobrDispatcherToml {
            default_model: Some(Model::Claude3Opus),
            workspace: Some(PathBuf::from("/tmp/toml-ws")),
            agent_github_token: Some("toml-agent-token".into()),
            copilot_github_token: Some("toml-copilot-token".into()),
            cli_tool: Some(Tool::Claude),
            work_branch_prefix: Some("toml_fix".into()),
            git_user_name: Some("test-user".into()),
            git_user_email: Some("test@example.com".into()),
            prompts: Some(TomlPrompts {
                path: Some(PathBuf::from("/opt/prompts")),
                planner: Some(vec![PathBuf::from("p.md")]),
                worker: Some(vec![PathBuf::from("w.md")]),
                reviewer: Some(vec![PathBuf::from("r.md")]),
                merger: Some(vec![PathBuf::from("m.md")]),
            }),
        };

        let config = ZbobrDispatcherConfig::build_with_env(Some(&toml), &env).unwrap();
        assert_eq!(config.default_model, Model::Claude3Opus);
        assert_eq!(config.workspace, PathBuf::from("/tmp/toml-ws"));
        assert_eq!(config.backend, BackendType::GitHub);
        assert_eq!(config.cli_tool, Tool::Claude);
        assert_eq!(config.work_branch_prefix, "toml_fix");
        assert_eq!(config.planner_prompts, vec![PathBuf::from("p.md")]);
        assert_eq!(config.worker_prompts, vec![PathBuf::from("w.md")]);
        assert_eq!(config.reviewer_prompts, vec![PathBuf::from("r.md")]);
        assert_eq!(config.prompts_path, Some(PathBuf::from("/opt/prompts")));
        assert_eq!(config.agent_github_token, "toml-agent-token");
        assert_eq!(config.copilot_github_token, "toml-copilot-token");
        assert_eq!(config.git_user_name, "test-user");
        assert_eq!(config.git_user_email, "test@example.com");
    }

    #[test]
    fn build_defaults_without_toml() {
        let env = TestEnv::new(&[]);
        let config = ZbobrDispatcherConfig::build_with_env(None, &env).unwrap();
        assert_eq!(config.default_model, Model::Gpt5Mini);
        assert_eq!(config.backend, BackendType::GitHub);
        assert_eq!(config.cli_tool, Tool::Copilot);
        assert_eq!(config.work_branch_prefix, "zbobr_fix");
        assert_eq!(config.workspace, PathBuf::from("./workspace"));
    }

    #[test]
    fn backend_type_roundtrip() {
        assert_eq!(
            "github".parse::<BackendType>().unwrap(),
            BackendType::GitHub
        );
        assert!("stub".parse::<BackendType>().is_err());
        assert!("invalid".parse::<BackendType>().is_err());
        assert_eq!(BackendType::GitHub.to_string(), "github");
    }
}
