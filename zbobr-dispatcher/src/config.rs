// Re-export all from zbobr-api so existing code still compiles.
pub use zbobr_api::config::*;
use zbobr_executor_claude::{
    ZbobrExecutorClaudeArgs, ZbobrExecutorClaudeConfig, ZbobrExecutorClaudeToml,
};
use zbobr_executor_copilot::{
    ZbobrExecutorCopilotArgs, ZbobrExecutorCopilotConfig, ZbobrExecutorCopilotToml,
};
use zbobr_executor_mcp_tester::{
    ZbobrExecutorMcpTesterArgs, ZbobrExecutorMcpTesterConfig, ZbobrExecutorMcpTesterToml,
};
use zbobr_utility::config_struct;

#[derive(Clone, Default)]
#[config_struct]
/// Executor configuration section used for TOML/CLI parsing.
///
/// Runtime ownership lives directly on `ZbobrDispatcher` as separate fields.
pub struct ZbobrExecutorConfig {
    /// Claude-specific defaults
    #[config(nested)]
    pub claude: ZbobrExecutorClaudeConfig,
    /// GitHub Copilot executor defaults
    #[config(nested)]
    pub copilot: ZbobrExecutorCopilotConfig,
    /// MCP tester scenarios for validating MCP servers
    #[config(nested)]
    pub mcp_tester: ZbobrExecutorMcpTesterConfig,
}

/*
#[cfg(test)]
mod tests {

    use super::*;
    use std::path::PathBuf;
    use crate::task::{Tool, Model};

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

    [preparator]
    prompts = ["prep.md"]

    [planner]
    prompts = ["plan.md", "shared.md"]
    "#;
        let config: ZbobrDispatcherToml = toml::from_str(toml_str).unwrap();
        assert_eq!(config.cli_tool, Some(Tool::Claude));
        assert_eq!(config.prompts_path, Some(PathBuf::from("/opt/prompts")));
        assert_eq!(
            config.planner.as_ref().unwrap().prompts,
            vec![PathBuf::from("plan.md"), PathBuf::from("shared.md")]
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
            preparator: Some(StageConfigToml { tool: None, model: None, prompts: Some(vec![PathBuf::from("pre.md")]) }),
            planner: Some(StageConfigToml { tool: None, model: None, prompts: Some(vec![PathBuf::from("p.md")]) }),
            worker: Some(StageConfigToml { tool: None, model: None, prompts: Some(vec![PathBuf::from("w.md")]) }),
            reviewer: Some(StageConfigToml { tool: None, model: None, prompts: Some(vec![PathBuf::from("r.md")]) }),
            merger: Some(StageConfigToml { tool: None, model: None, prompts: Some(vec![PathBuf::from("m.md")]) }),
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
            config.preparator.prompts,
            vec![PathBuf::from("/test/config/pre.md")]
        );
        assert_eq!(
            config.planner.prompts,
            vec![PathBuf::from("/test/config/p.md")]
        );
        assert_eq!(
            config.worker.prompts,
            vec![PathBuf::from("/test/config/w.md")]
        );
        assert_eq!(
            config.reviewer.prompts,
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
        assert_eq!(config.model, Model::default());
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
            preparator: None,
            planner: None,
            worker: None,
            reviewer: None,
            merger: None,
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
            preparator: None,
            planner: None,
            worker: None,
            reviewer: None,
            merger: None,
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
            preparator: None,
            planner: None,
            worker: None,
            reviewer: None,
            merger: None,
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
    fn stage_settings_override_helpers() {
        let mut cfg = ZbobrDispatcherConfig::default();
        cfg.tool = Tool::Copilot;
        cfg.model = Model::Gpt5;
        cfg.preparator.tool = Some(Tool::Claude);
        cfg.preparator.model = Some(Model::ClaudeOpus4_5);

        assert_eq!(cfg.tool_for_role(Role::Preparator), Tool::Claude);
        assert_eq!(cfg.tool_for_role(Role::Planner), Tool::Copilot);
        assert_eq!(cfg.model_for_role(Role::Preparator), Model::ClaudeOpus4_5);
        assert_eq!(cfg.model_for_role(Role::Planner), Model::Gpt5);
    }

    #[test]
    fn validate_tool_model_global_mismatch() {
        let mut cfg = ZbobrDispatcherConfig::default();
        // set git fields so validation gets past name/email check
        cfg.git_user_name = "u".into();
        cfg.git_user_email = "e".into();

        cfg.tool = Tool::Claude;
        cfg.model = Model::Gpt5; // not usable by Claude
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn validate_tool_model_role_mismatch() {
        let mut cfg = ZbobrDispatcherConfig::default();
        cfg.git_user_name = "u".into();
        cfg.git_user_email = "e".into();

        cfg.tool = Tool::Copilot;
        cfg.model = Model::Gpt5;
        cfg.preparator.tool = Some(Tool::Claude);
        // preparator will inherit global model which is incompatible
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn validate_tool_model_combinations_ok() {
        let mut cfg = ZbobrDispatcherConfig::default();
        cfg.git_user_name = "u".into();
        cfg.git_user_email = "e".into();

        cfg.tool = Tool::Copilot;
        cfg.model = Model::Gpt5;
        cfg.preparator.tool = Some(Tool::Copilot);
        cfg.preparator.model = Some(Model::Gpt5_2);
        assert!(cfg.validate().is_ok());
    }
}
*/
