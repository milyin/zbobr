# Plan: Use `--system-prompt` for Claude CLI

## Rationale
Claude Code supports `--system-prompt` to pass system-level instructions separately from the user prompt (`-p`). Currently both role and task prompts are concatenated into a single `-p` string. Splitting them improves prompt clarity for Claude. Copilot doesn't support this flag, so it concatenates both parts into `-p` as before.

## Key Design Decision
The natural split point already exists in `prompt_files_for_stage()`: the first file is the role prompt (from `stage_def.role_prompt` or `workflow.roles[role].prompt`), and `stage_def.prompts` are additional task prompts. We formalize this split with a `SplitPrompt` struct and thread the two parts through the executor pipeline.

## No analog found
This is a new capability. The implementation follows the existing prompt-building patterns and extends them.

## Changes
1. `zbobr-dispatcher/src/prompts.rs` — `SplitPrompt` struct, split builder methods, `render_prompt` helper, tests
2. `zbobr-api/src/tool_executor.rs` — add `system_prompt: Option<&str>` to `ToolExecutor::execute()`
3. `zbobr-executor-claude/src/lib.rs` — pass `--system-prompt` when present
4. `zbobr-executor-copilot/src/lib.rs` — prepend system prompt to `-p` content
5. `zbobr-executor-mcp-tester/src/lib.rs` — ignore system prompt
6. `zbobr-dispatcher/src/cli.rs` — use `split_prompt()`, pass both parts through `execute_tool()`
