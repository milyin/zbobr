In `zbobr/src/init.rs`, add a `"linter_worker"` entry to the roles map (where all role definitions like `test_worker`, `linter`, etc. are defined).

Role configuration:
- `tool: Some("developer".to_string())` — needs code editing capability (unlike `linter` which uses `"drudge"`)
- `prompt: Some(PathBuf::from("linter_worker.md"))`
- `mcp` tools: Same set as `test_worker` MINUS `AddChecklistItem`, `CheckChecklistItem`, and `ReportIntermediate` — the linter_worker is a focused fixer and doesn't manage checklists or report intermediate progress. So the tools are: `StopWithError`, `ReportSuccess`, `ReportFailure`, `StopWithQuestion`, `GetCtxRec`.

**Why `developer` tool**: The linter_worker needs to edit source files to fix linting/formatting issues, which requires developer-level tool access. The `drudge` tool used by the linter is sufficient for read-only checking but not for making code changes.

**Why minimal MCP tools**: The linter_worker's job is tightly scoped — run linter, apply fixes, commit, report success or failure. No need for checklist management or intermediate reporting.

**Pattern to follow**: The `test_worker` role definition in `init.rs` is the closest analog. Follow the same structure.