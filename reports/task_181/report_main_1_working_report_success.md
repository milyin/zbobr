# Task #181: Use system prompt parameter for Claude

## Summary
Split prompt building so Claude receives the role prompt via `--system-prompt` while Copilot keeps the concatenated `-p` approach.

## Changes (6 files)

### 1. `zbobr-dispatcher/src/prompts.rs`
- Added `SplitPrompt` struct with `system_prompt: Option<String>` and `prompt: String`
- Added `role_prompt_files_for_stage()` — extracts role prompt file paths from a stage definition
- Added `task_prompt_files_for_stage()` — extracts task (non-role) prompt file paths
- Added `resolve_prompt_paths()` helper — resolves relative paths using workflow's `prompts_dir`
- Added `render_prompt()` helper — renders a template string with variables
- Added `build_for_stage_split()` async method on `ConfiguredPromptBuilder`
- Added `build_for_stage_split_with_task()` sync method on `ConfiguredPromptBuilder`
- Added 4 unit tests for the new split functionality
- Existing `build_for_stage()` and `build_for_stage_with_task()` left unchanged

### 2. `zbobr-api/src/tool_executor.rs`
- Added `system_prompt: Option<&str>` parameter to `ToolExecutor::execute()` trait method

### 3. `zbobr-executor-claude/src/lib.rs`
- Accepts `system_prompt: Option<&str>`
- When `Some(text)`, adds `--system-prompt` and `text` to args before `-p`
- Changed args from fixed array to `Vec<&str>` to support conditional arguments

### 4. `zbobr-executor-copilot/src/lib.rs`
- Accepts `system_prompt: Option<&str>`
- When `Some(text)`, prepends it to the prompt with `\n\n` separator before passing to `-p`

### 5. `zbobr-executor-mcp-tester/src/lib.rs`
- Accepts `_system_prompt: Option<&str>` (ignored)
- Updated unit test call site to pass `None` for the new parameter

### 6. `zbobr-dispatcher/src/cli.rs`
- Changed `StageHandler::prompt()` to `split_prompt()` returning `SplitPrompt`
- Updated `execute_tool()` to accept and pass `system_prompt: Option<&str>`
- Updated `StageHandler::run()` to use split prompt; stores combined prompt in `prompt_holder` for debugging

## Verification
- `cargo build` — success
- `cargo test` — all tests pass (including 4 new split prompt tests)
