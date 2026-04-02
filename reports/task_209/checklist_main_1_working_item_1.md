In zbobr-api/src/config.rs validate():
- After checking executor.is_none() && parent.is_none(), also check that if executor is Some, it must be one of Tool::CLAUDE, Tool::COPILOT, Tool::MCP_TESTER
- Error message: "Provider '{}' has unknown executor '{}' — must be one of 'claude', 'copilot', 'mcp-tester'"

In zbobr-dispatcher/src/lib.rs build_executor():
- Change return type from `Box<dyn ToolExecutor>` to `anyhow::Result<Box<dyn ToolExecutor>>`
- Change the `_ =>` arm to `anyhow::bail!("Unknown executor '{}' for provider '{}'", provider.executor, provider.name)`
- Wrap other arms with `Ok(...)`

In zbobr-dispatcher/src/cli.rs line ~551:
- Change `self.zbobr.build_executor(...)` to `self.zbobr.build_executor(...)?`