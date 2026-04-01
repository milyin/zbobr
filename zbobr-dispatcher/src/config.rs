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
