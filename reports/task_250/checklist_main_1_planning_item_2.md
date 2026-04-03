In `zbobr/src/init.rs`, in the roles IndexMap within `default_workflow()` (around lines 339-448), add a new "linter" role.

**What:** Define the role with:
- MCP tools: StopWithError, ReportSuccess, ReportFailure, StopWithQuestion, GetCtxRec (same set as tester)
- prompt: PathBuf::from("linter.md")
- tool: "drudge" (the new cheap-model tool)

**Why:** The linter role needs the same reporting capabilities as the tester but uses the cheaper "drudge" tool since formatting checks are simple tasks.

**Pattern to follow:** The "tester" role definition — same structure, different tool and prompt assignments.