# Fix: Priority Inheritance and Executor Validation

## Issue 1: Priority Inheritance (Fixed)

**Problem:** `ProviderDefinition.priority` was `i32` with `#[serde(default = "default_provider_priority")]` returning 10. A child provider without an explicit `priority` was indistinguishable from one with `priority = 10`, so `resolve_single_provider()` always used `def.priority` directly and inheritance never kicked in.

**Fix (`zbobr-api/src/config.rs`):**
- Changed `priority: i32` → `priority: Option<i32>` with `#[serde(default, skip_serializing_if = "Option::is_none")]`
- Removed `default_provider_priority()` helper
- In `resolve_single_provider()`: child branch uses `def.priority.unwrap_or(parent.priority)`, root branch uses `def.priority.unwrap_or(10)`
- Updated all test `ProviderDefinition` constructors: `priority: 10` → `priority: None`, `priority: 5` → `priority: Some(5)`
- Updated `zbobr/src/init.rs` same way

## Issue 2: Executor Validation (Fixed)

**Problem:** `validate()` didn't check executor strings, and `build_executor()` had a `_ => ClaudeExecutor` fallback that silently ran Claude for any unknown executor name.

**Fix:**
- `zbobr-api/src/config.rs` `validate()`: added check after the existing `executor.is_none() && parent.is_none()` check — if executor is `Some`, it must be one of `Tool::CLAUDE`, `Tool::COPILOT`, `Tool::MCP_TESTER`; otherwise bail with a clear error message
- Imported `crate::task::Tool` in config.rs to use the constants
- `zbobr-dispatcher/src/lib.rs` `build_executor()`: changed return type to `anyhow::Result<Box<dyn ToolExecutor>>`, wrapped `"copilot"` and `"mcp-tester"` arms with `Ok(...)`, replaced `_ => ClaudeExecutor` fallback with an explicit `"claude"` arm + `other => anyhow::bail!(...)` for truly unknown values
- `zbobr-dispatcher/src/cli.rs`: added `?` to the `build_executor(...)` call

## Verification

- `cargo build`: clean
- `cargo test`: all tests pass (no failures or regressions)