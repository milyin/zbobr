// Re-export all from zbobr-api so existing code still compiles.
pub use zbobr_api::config::*;

use std::path::Path;

use anyhow::Context;
use zbobr_executor_claude::{
    ZbobrExecutorClaudeArgs, ZbobrExecutorClaudeConfig, ZbobrExecutorClaudeToml,
    config::ZbobrExecutorClaude,
};
use zbobr_executor_copilot::{
    ZbobrExecutorCopilotArgs, ZbobrExecutorCopilotConfig, ZbobrExecutorCopilotToml,
    config::ZbobrExecutorCopilot,
};
use zbobr_executor_mcp_tester::{
    ZbobrExecutorMcpTesterArgs, ZbobrExecutorMcpTesterConfig, ZbobrExecutorMcpTesterToml,
    config::ZbobrExecutorMcpTester,
};
use zbobr_repo_backend_fs::{
    ZbobrRepoBackendFsArgs, ZbobrRepoBackendFsConfig, ZbobrRepoBackendFsToml,
    config::ZbobrRepoBackendFs,
};
use zbobr_repo_backend_github::{
    ZbobrRepoBackendGithubArgs, ZbobrRepoBackendGithubConfig, ZbobrRepoBackendGithubToml,
    config::ZbobrRepoBackendGithub,
};
use zbobr_task_backend_fs::{
    ZbobrTaskBackendFsArgs, ZbobrTaskBackendFsConfig, ZbobrTaskBackendFsToml,
    config::ZbobrTaskBackendFs,
};
use zbobr_task_backend_github::{
    ZbobrTaskBackendGithubArgs, ZbobrTaskBackendGithubConfig, ZbobrTaskBackendGithubToml,
    config::ZbobrTaskBackendGithub,
};
use zbobr_utility::config_struct;

#[derive(Clone)]
#[config_struct]
/// Task backend configuration section.
pub struct ZbobrTaskBackendConfig {
    /// GitHub issues as the task source
    #[config(nested)]
    pub github: ZbobrTaskBackendGithub,
    /// Filesystem task backend (YAML files in tasks/)
    #[config(nested)]
    pub fs: ZbobrTaskBackendFs,
}

#[derive(Clone)]
#[config_struct]
/// Repo backend configuration section.
pub struct ZbobrRepoBackendConfig {
    /// GitHub repo backend (fork + push via API)
    #[config(nested)]
    pub github: ZbobrRepoBackendGithub,
    /// Filesystem repo backend (operate on local clones)
    #[config(nested)]
    pub fs: ZbobrRepoBackendFs,
}

#[derive(Clone)]
#[config_struct]
/// Executor configuration section.
pub struct ZbobrExecutorConfig {
    /// Claude-specific defaults
    #[config(nested)]
    pub claude: ZbobrExecutorClaude,
    /// GitHub Copilot executor defaults
    #[config(nested)]
    pub copilot: ZbobrExecutorCopilot,
    /// MCP tester scenarios for validating MCP servers
    #[config(nested)]
    pub mcp_tester: ZbobrExecutorMcpTester,
}

#[derive(Clone)]
#[config_struct]
/// Root configuration for zbobr.
pub struct ZbobrConfig {
    /// Dispatcher runtime: workspaces, prompts, tokens
    #[config(nested)]
    pub dispatcher: ZbobrDispatcherConfig,
    /// Task storage backends: control where zbobr discovers tasks.
    #[config(nested)]
    pub tasks: ZbobrTaskBackendConfig,
    /// Repo backends: where zbobr clones and pushes code.
    #[config(nested)]
    pub repo: ZbobrRepoBackendConfig,
    /// Executor defaults and scenarios.
    #[config(nested)]
    pub executor: ZbobrExecutorConfig,
}

impl ZbobrConfigToml {
    /// Load a TOML config from a file path.
    /// Returns Ok(None) if the file does not exist.
    pub fn load(path: &Path) -> anyhow::Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        let config: ZbobrConfigToml = toml::from_str(&content)
            .with_context(|| format!("Failed to parse {}", path.display()))?;
        Ok(Some(config))
    }
}

impl ZbobrConfig {
    /// Build the full configuration from TOML and CLI args.
    /// Relative paths are resolved against `config_dir`.
    pub fn build(
        toml: Option<ZbobrConfigToml>,
        args: ZbobrConfigArgs,
        config_dir: &Path,
    ) -> anyhow::Result<Self> {
        let toml = toml.unwrap_or_default();

        let dispatcher =
            ZbobrDispatcherConfig::build(toml.dispatcher, args.dispatcher, config_dir)?;

        let tasks = {
            let t = toml.tasks.unwrap_or_default();
            ZbobrTaskBackendConfig {
                github:
                    <ZbobrTaskBackendGithubConfig as zbobr_api::config::BackendConfig>::build_config(
                        t.github,
                        args.tasks.github,
                        config_dir,
                    ),
                fs: <ZbobrTaskBackendFsConfig as zbobr_api::config::BackendConfig>::build_config(
                    t.fs,
                    args.tasks.fs,
                    config_dir,
                ),
            }
        };

        let repo = {
            let t = toml.repo.unwrap_or_default();
            ZbobrRepoBackendConfig {
                github:
                    <ZbobrRepoBackendGithubConfig as zbobr_api::config::BackendConfig>::build_config(
                        t.github,
                        args.repo.github,
                        config_dir,
                    ),
                fs: <ZbobrRepoBackendFsConfig as zbobr_api::config::BackendConfig>::build_config(
                    t.fs,
                    args.repo.fs,
                    config_dir,
                ),
            }
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

/*
#[cfg(test)]
mod tests {

    use super::*;
    use std::path::PathBuf;
    use crate::task::Tool;

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
            base_port: None,
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
            base_port: None,
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
            base_port: None,
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
            base_port: None,
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
*/
